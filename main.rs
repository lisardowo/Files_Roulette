mod app;
mod handleFile;
mod menu;
mod ui;

use app::App;
use std::io;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = menu::run(&mut terminal).and_then(|paths| App::new(paths).run(&mut terminal));
    ratatui::restore();
    result
}
