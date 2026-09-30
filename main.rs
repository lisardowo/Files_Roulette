mod app;
mod handleFile;
mod menu;
mod ui;

use app::App;

fn main() {
    let mut terminal = ratatui::init();
    let _paths = menu::run(&mut terminal);
    App::new().run(&mut terminal);
    ratatui::restore();
}
