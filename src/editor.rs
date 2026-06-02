use crate::command::Command;
use crate::selection::Selection;
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, Event::{self}, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self},
};
use std::io::{self, Write};

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
    pub clipboard: ClipboardContext,
    pub undo_stack: Vec<Command>,
    pub redo_stack: Vec<Command>,
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
            clipboard: ClipboardContext::new().unwrap(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn move_cursor(&mut self, press: KeyCode) {
        match press {
            KeyCode::Up => {
                if self.cursor_y > 0 {
                    self.cursor_y = self.cursor_y.saturating_sub(1);
                } else if self.scroll_y > 0 {
                    self.scroll_y -= 1;
                }
                let new_line_len = self.buffer[self.cursor_y as usize + self.scroll_y].len() as u16;
                if self.cursor_x > new_line_len {
                    self.cursor_x = new_line_len;
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
                let new_line_len = self.buffer[self.cursor_y as usize + self.scroll_y].len() as u16;
                if self.cursor_x > new_line_len {
                    self.cursor_x = new_line_len;
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
                } else if line_idx < self.buffer.len() - 1 {
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
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.copy_selection(),
                    (KeyCode::Char('v'), KeyModifiers::CONTROL) => self.paste(),
                    (KeyCode::Char('z'), KeyModifiers::CONTROL) => self.undo(),
                    (KeyCode::Char('y'), KeyModifiers::CONTROL) => self.redo(),
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
}
