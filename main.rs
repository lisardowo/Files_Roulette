mod app;
mod ui;

use std::io;

use app::App;

fn main() {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();
}
