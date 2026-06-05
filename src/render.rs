use crate::editor::{Editor, Mode};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

const COLOR_PRI: Color = Color::White;
const COLOR_SEC: Color = Color::Black;
const COLOR_ERR: Color = Color::Red;
const COLOR_HIL: Color = Color::Magenta;

impl Editor {
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
            self.cursor_x + 1, self.cursor_y + 1, mode_str, self.path, modified_str
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
