mod buffer;
mod command;
mod editor;
mod render;
mod selection;
mod undo;

use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use editor::Editor;
use std::io;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("\x1b[31merror:\x1b[0m no file specified");
        eprintln!("Usage: rune <file>");
        std::process::exit(1);
    }
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let mut rune = Editor::new();
    rune.load(&args[1])?;
    rune.run(&mut stdout)?;
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
