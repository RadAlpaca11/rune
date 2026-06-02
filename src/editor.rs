use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use cli_clipboard::{ClipboardContext, ClipboardProvider};

use std::io::{self, Write};

use crate::selection::Selection;

const COLOR_PRI: Color = Color::White;
const COLOR_SEC: Color = Color::Black;
const COLOR_ERR: Color = Color::Red;
const COLOR_HIL: Color = Color::Magenta;

pub enum Mode {
    Viewing,
    Insert,
}

pub struct Editor {
    pub mode: Mode,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub window_width: u16,
    pub window_height: u16,
    pub buffer: Vec<String>,
    pub scroll_y: usize,
    pub path: String,
    pub modified: bool,
    pub selection: Option<Selection>,
    pub clipboard: String,
}

impl Editor {
    pub fn new() -> Self {
        let (window_width, window_height) = terminal::size().unwrap();
        Editor {
            mode: Mode::Viewing,
            cursor_x: 0,
            cursor_y: 0,
            window_width,
            window_height,
            buffer: Vec::new(),
            scroll_y: 0,
            path: String::new(),
            modified: false,
            selection: None,
            clipboard: String::new(),
        }
    }

    pub fn draw(&self, stdout: &mut impl Write) -> io::Result<()> {
        execute!(stdout, terminal::Clear(ClearType::All))?;

        let line_number_width: u16 = self.buffer.len().to_string().len() as u16 + 2;
        let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = if let Some(ref sel) = self.selection
        {
            sel.normalize()
        } else {
            (0, 0, 0, 0)
        };
        let has_selection: bool = self.selection.is_some();

        for row in 0..self.window_height - 1 {
            execute!(stdout, cursor::MoveTo(0, row))?;
            let buffer_line = row as usize + self.scroll_y;
            if let Some(line) = self.buffer.get(buffer_line) {
                // write line number first
                write!(
                    stdout,
                    "{:>width$} ",
                    buffer_line + 1,
                    width = (line_number_width - 2) as usize
                )?;

                // write each character with highlighting
                for (i, c) in line.chars().enumerate() {
                    let highlighted = has_selection
                        && if sel_start_y == sel_end_y {
                            buffer_line == sel_start_y
                                && i >= sel_start_x as usize
                                && i < sel_end_x as usize
                        } else if buffer_line == sel_start_y {
                            i >= sel_start_x as usize
                        } else if buffer_line == sel_end_y {
                            i < sel_end_x as usize
                        } else {
                            buffer_line > sel_start_y && buffer_line < sel_end_y
                        };

                    if highlighted {
                        execute!(stdout, SetBackgroundColor(COLOR_HIL))?;
                    }
                    write!(stdout, "{}", c)?;
                    if highlighted {
                        execute!(stdout, ResetColor)?;
                    }
                }
            } else {
                write!(stdout, "~")?;
            }
        }

        execute!(
            stdout,
            cursor::MoveTo(0, self.window_height - 1),
            SetBackgroundColor(COLOR_PRI),
            SetForegroundColor(COLOR_SEC)
        )?;

        let mode_str = match self.mode {
            Mode::Viewing => "VIEWING",
            Mode::Insert => "INSERT",
        };
        let modified_str = if self.modified { "[+]" } else { "" };

        let status = format!(
            "x: {}, y: {}  MODE:{}  FILE:{}   {}",
            self.cursor_x, self.cursor_y, mode_str, self.path, modified_str
        );
        let padding = " ".repeat((self.window_width as usize).saturating_sub(status.len()));

        write!(stdout, "{}{}", status, padding)?;

        execute!(stdout, ResetColor)?;
        execute!(
            stdout,
            cursor::MoveTo(self.cursor_x + line_number_width - 1, self.cursor_y)
        )?;
        stdout.flush()?;
        Ok(())
    }

    pub fn move_cursor(&mut self, press: KeyCode) {
        match press {
            KeyCode::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                } else if self.scroll_y > 0 {
                    self.scroll_y -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_y as usize + self.scroll_y < self.buffer.len() - 1 {
                    if self.cursor_y < self.window_height - 2 {
                        self.cursor_y += 1;
                    } else {
                        self.scroll_y += 1;
                    }
                }
            }
            KeyCode::Left => {
                if self.cursor_x > 0 {
                    self.cursor_x = self.cursor_x.saturating_sub(1);
                } else if self.cursor_y > 0 {
                    self.move_cursor(KeyCode::Up);
                    let line_idx = self.cursor_y as usize + self.scroll_y;
                    let line_len = self.buffer[line_idx].len() as u16;
                    self.cursor_x = line_len;
                }
            }
            KeyCode::Right => {
                let line_idx = self.cursor_y as usize + self.scroll_y;
                let line_len = self.buffer[line_idx].len() as u16;
                if self.cursor_x < line_len {
                    self.cursor_x += 1;
                } else {
                    self.move_cursor(KeyCode::Down);
                    self.cursor_x = 0;
                }
            }
            _ => {}
        }
    }

    pub fn start_selection(&mut self) {
        self.selection = Some(Selection {
            start_x: self.cursor_x,
            start_y: self.cursor_y as usize + self.scroll_y,
            end_x: self.cursor_x,
            end_y: self.cursor_y as usize + self.scroll_y,
        });
    }

    pub fn update_selection(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end_x = self.cursor_x;
            sel.end_y = self.cursor_y as usize + self.scroll_y;
        }
    }

    pub fn run(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        loop {
            self.draw(stdout)?;
            match event::read()? {
                Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => match (code, modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                        self.close(stdout)?;
                        break;
                    }
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.save()?,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        self.clipboard = self.copy_selection()
                    }
                    (KeyCode::Char('v'), KeyModifiers::CONTROL) => self.paste_selection(),
                    (KeyCode::Esc, KeyModifiers::NONE) => self.mode = Mode::Viewing,
                    (KeyCode::Char('i'), KeyModifiers::NONE)
                        if matches!(self.mode, Mode::Viewing) =>
                    {
                        self.mode = Mode::Insert;
                    }
                    (KeyCode::Up, KeyModifiers::SHIFT) => {
                        if self.selection.is_none() {
                            self.start_selection();
                        }
                        self.move_cursor(KeyCode::Up);
                        self.update_selection();
                    }
                    (KeyCode::Down, KeyModifiers::SHIFT) => {
                        if self.selection.is_none() {
                            self.start_selection();
                        }
                        self.move_cursor(KeyCode::Down);
                        self.update_selection();
                    }
                    (KeyCode::Left, KeyModifiers::SHIFT) => {
                        if self.selection.is_none() {
                            self.start_selection();
                        }
                        self.move_cursor(KeyCode::Left);
                        self.update_selection();
                    }
                    (KeyCode::Right, KeyModifiers::SHIFT) => {
                        if self.selection.is_none() {
                            self.start_selection();
                        }
                        self.move_cursor(KeyCode::Right);
                        self.update_selection();
                    }
                    (code, _) => match self.mode {
                        Mode::Viewing => {
                            self.selection = None;
                            self.move_cursor(code);
                        }
                        Mode::Insert => match code {
                            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                                self.selection = None;
                                self.move_cursor(code)
                            }
                            _ => self.check_insert(code),
                        },
                    },
                },
                Event::Resize(width, height) => {
                    self.window_width = width;
                    self.window_height = height;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn prompt_save(&mut self, stdout: &mut impl Write) -> io::Result<bool> {
        execute!(
            stdout,
            cursor::MoveTo(0, self.window_height - 1),
            terminal::Clear(ClearType::CurrentLine),
            SetBackgroundColor(COLOR_ERR),
            SetForegroundColor(COLOR_PRI),
        )?;
        write!(stdout, "UNSAVED CHANGES! Save before exiting? (y/n) ")?;
        execute!(stdout, ResetColor)?;
        stdout.flush()?;

        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('y') => return Ok(true),
                    KeyCode::Char('n') => return Ok(false),
                    _ => {}
                }
            }
        }
    }

    pub fn close(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        if self.modified {
            if self.prompt_save(stdout)? {
                self.save()?;
            }
        }
        Ok(())
    }
}
