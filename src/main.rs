mod buffer;
mod editor;
mod selection;

use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use editor::Editor;
use std::io;

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
