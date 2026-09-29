mod app;
mod handleFile;
mod menu;
mod ui;

use app::App;

fn main() {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();
}
