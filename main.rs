mod app;
mod handleFile;
mod menu;
mod ui;

use app::App;

fn main() {
    let mut terminal = ratatui::init();
    let paths = menu::run(&mut terminal);
    App::new(paths).run(&mut terminal);
    ratatui::restore();
}
