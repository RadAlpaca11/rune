use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io::{self, Write};


const COLOR_PRI: Color = Color::White;
const COLOR_SEC: Color = Color::Black;
const COLOR_ERR: Color = Color::Red;

enum Mode {
    Viewing,
    Insert,
}

struct Editor {
    mode: Mode,
    cursor_x: u16,
    cursor_y: u16,
    window_width: u16,
    window_height: u16,
    buffer: Vec<String>,
    scroll_y: usize,
    path: String,
    modified: bool,
}

impl Editor {
    fn new() -> Self {
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
        }
    }

    fn load(&mut self, path: &str) -> io::Result<()> {
        self.path = path.to_string();
        if std::path::Path::new(path).exists() {
            let contents = std::fs::read_to_string(path)?;
            self.buffer = contents.lines().map(|line: &str| line.to_string()).collect();
        } else {
            self.buffer = vec![String::new()];
        }
        self.modified = false;
        Ok(())
    }

    fn save(&mut self) -> io::Result<()> {
        std::fs::write(&self.path, self.buffer.join("\n"))?;
        self.modified = false;
        Ok(())
    }

    fn draw(&self, stdout: &mut impl Write) -> io::Result<()> {
        execute!(stdout, terminal::Clear(ClearType::All))?;
        
        let line_number_width: u16 = self.buffer.len().to_string().len() as u16 + 2;

        for row in 0..self.window_height - 1 {
            execute!(stdout, cursor::MoveTo(0, row))?;
            let buffer_line = row as usize + self.scroll_y;
            if let Some(line) = self.buffer.get(buffer_line) {
                write!(stdout, "{:>width$} {}", buffer_line + 1, line, width = (line_number_width - 2) as usize)?;
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
        let modified_str = if self.modified { "[+]"} else { "" };

        let status = format!("x: {}, y: {}  MODE:{}  FILE:{}   {}", self.cursor_x, self.cursor_y, mode_str, self.path, modified_str);
        let padding = " ".repeat((self.window_width as usize).saturating_sub(status.len()));

        write!(stdout, "{}{}", status, padding)?;

        execute!(stdout, ResetColor)?;
        execute!(stdout, cursor::MoveTo(self.cursor_x + line_number_width - 1, self.cursor_y))?;
        stdout.flush()?;
        Ok(())
    }

    fn move_cursor(&mut self, press: KeyCode) {
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
                } else if self.cursor_y > 0{
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

    fn insert(&mut self, key:KeyCode) {
        let line_idx = self.cursor_y as usize + self.scroll_y;
        self.modified = true;

        match key {
            KeyCode::Char(c) => {
                self.buffer[line_idx].insert(self.cursor_x as usize, c);
                self.cursor_x += 1;
            }
            KeyCode::Enter => {
                let new_line = self.buffer[line_idx].split_off(self.cursor_x as usize);
                self.buffer.insert(line_idx + 1, new_line);
                self.cursor_x = 0;
                self.cursor_y +=1;
            }
            KeyCode::Backspace => {
                if self.cursor_x > 0 {
                    self.buffer[line_idx].remove((self.cursor_x - 1) as usize);
                    self.cursor_x -= 1;
                } else if line_idx > 0 {
                    let prev_line_len = self.buffer[line_idx - 1].len() as u16;
                    let current_line = self.buffer.remove(line_idx);
                    self.buffer[line_idx - 1].push_str(&current_line);
                    self.cursor_x = prev_line_len;
                    self.cursor_y -= 1;
                }
            }
            KeyCode::Tab => {
                self.buffer[line_idx].insert_str(self.cursor_x as usize, &" ".repeat(4));
                self.cursor_x += 4;
            }
            _ => {}
        }
    }

    fn run(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        loop {
            self.draw(stdout)?;
            match event::read()? {
                Event::Key(KeyEvent { code, modifiers, .. }) => {
                    match (code, modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            self.close(stdout)?;
                            break;
                        }
                        (KeyCode::Char('s'), KeyModifiers::CONTROL) => self.save()?,
                        (KeyCode::Esc, KeyModifiers::NONE) => self.mode = Mode::Viewing,
                        (KeyCode::Char('i'), KeyModifiers::NONE) if matches!(self.mode, Mode::Viewing) => {
                            self.mode = Mode::Insert;
                        }
                        (code, _) => match self.mode {
                            Mode::Viewing => self.move_cursor(code),
                            Mode::Insert => {
                                match code {
                                    KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => self.move_cursor(code),
                                    _ => self.insert(code),
                                }
                            }
                        }
                    }
                }
                Event::Resize(width, height) => {
                    self.window_width = width;
                    self.window_height = height;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn prompt_save(&mut self, stdout: &mut impl Write) -> io::Result<bool> {
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

    fn close(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        if self.modified {
            if self.prompt_save(stdout)? {
                self.save()?;
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let mut rune = Editor::new();
    let args: Vec<String> = std::env::args().collect();
    rune.load(&args[1])?;
    rune.run(&mut stdout)?;
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
