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

struct Editor {
    cursor_x: u16,
    cursor_y: u16,
    window_width: u16,
    window_height: u16,
    buffer: Vec<String>,
    scroll_y: usize,
}

impl Editor {
    fn new() -> Self {
        let (window_width, window_height) = terminal::size().unwrap();
        Editor {
            cursor_x: 0,
            cursor_y: 0,
            window_width,
            window_height,
            buffer: Vec::new(),
            scroll_y: 0,
        }
    }

    fn load(&mut self, path: &str) -> io::Result<()> {
        let contents = std::fs::read_to_string(path)?;
        self.buffer = contents.lines().map(|line| line.to_string()).collect();
        Ok(())
    }

    fn draw(&self, stdout: &mut impl Write) -> io::Result<()> {
        execute!(stdout, terminal::Clear(ClearType::All))?;
        
        let line_number_width = self.buffer.len().to_string().len() as u16 + 2;

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

        let status = format!("x: {}, y: {}", self.cursor_x, self.cursor_y);
        let padding = " ".repeat((self.window_width as usize).saturating_sub(status.len()));

        write!(stdout, "{}{}", status, padding)?;

        execute!(stdout, ResetColor)?;
        execute!(stdout, cursor::MoveTo(self.cursor_x, self.cursor_y))?;
        stdout.flush()?;
        Ok(())
    }

    fn move_cursor(&mut self, press: KeyCode) {
        match press {
            KeyCode::Up => {
                self.cursor_y = self.cursor_y.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.cursor_y < self.window_height - 2 { self.cursor_y += 1; }
            }
            KeyCode::Left => {
                self.cursor_x = self.cursor_x.saturating_sub(1);
            }
            KeyCode::Right => {
                if self.cursor_x < self.window_width - 1 { self.cursor_x += 1; }
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
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                        (code, _) => self.move_cursor(code),
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
