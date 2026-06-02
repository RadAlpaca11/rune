use crate::editor::Editor;
use crossterm::event::KeyCode;
use std::io;

impl Editor {
    pub fn load(&mut self, path: &str) -> io::Result<()> {
        self.path = path.to_string();
        if std::path::Path::new(path).exists() {
            let contents = std::fs::read_to_string(path)?;
            self.buffer = contents
                .lines()
                .map(|line: &str| line.to_string())
                .collect();
        } else {
            self.buffer = vec![String::new()];
        }
        self.modified = false;
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        std::fs::write(&self.path, self.buffer.join("\n"))?;
        self.modified = false;
        Ok(())
    }

    pub fn check_insert(&mut self, key: KeyCode) {
        if self.selection.is_some() {
            match key {
                KeyCode::Backspace | KeyCode::Delete => {
                    self.delete_selection();
                }
                KeyCode::Char(_) | KeyCode::Enter | KeyCode::Tab => {
                    self.delete_selection();
                    self.insert(key);
                }
                _ => {}
            }
        } else {
            self.insert(key);
        }
    }

    pub fn insert(&mut self, key: KeyCode) {
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
                self.cursor_y += 1;
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

    pub fn delete_selection(&mut self) {
        if let Some(ref sel) = self.selection.clone() {
            let (start_x, start_y, end_x, end_y) = sel.normalize();

            if start_y == end_y {
                self.buffer[start_y].drain(start_x as usize..end_x as usize);
            } else {
                self.buffer[start_y].drain(start_x as usize..);
                self.buffer[end_y].drain(..end_x as usize);

                self.buffer.drain(start_y + 1..end_y);
                let last = self.buffer.remove(start_y + 1);
                self.buffer[start_y].push_str(&last);
            }
            self.selection = None;
            self.modified = true;
            self.cursor_x = start_x;
            self.cursor_y = (start_y - self.scroll_y) as u16;
        }
    }
}
