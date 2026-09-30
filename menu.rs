use std::{
    io,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Gauge, Paragraph},
};

use crate::handleFile::getfiles;

pub const TITLE: &str = r#"
 ░▒▓██████▓▒░░▒▓███████▓▒░ ░▒▓██████▓▒░ ░▒▓██████▓▒░ ░▒▓██████▓▒░░▒▓███████▓▒░ ░▒▓██████▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒▒▓███▓▒░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒▒▓███▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓████████▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░
 ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░ ░▒▓██████▓▒░ ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░  ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░
"#;

pub const PREPARE_MESSAGE: &str = r#"
______        __  __          _
| ___ \      / _|/ _|        (_)
| |_/ /_   _| |_| |_ ___ _ __ _ _ __   __ _   _   _  ___  _   _ _ __  ___
| ___ \ | | |  _|  _/ _ \ '__| | '_ \ / _` | | | | |/ _ \| | | | '__|/ __|
| |_/ / |_| | | | ||  __/ |  | | | | | (_| | | |_| | (_) | |_| | |   \__ \
\____/ \__,_|_| |_| \___|_|  |_|_| |_|\__, |  \__, |\___/ \__,_|_|   |___/
                                       __/ |   __/ |
                                      |___/   |___/
"#;

const FAKEBUFFERING: Duration = Duration::from_secs(3);
const FAKEMAXBUFFERING: f32 = 72.0;

const RUSHTOEND: f32 = 3.5;

enum Phase {
    FAKE,
    RUSH,
    DONE,
}

pub fn run(terminal: &mut DefaultTerminal) -> io::Result<Vec<PathBuf>> {
    let (transmitter, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        getfiles(&mut buffer);
        let _ = transmitter.send(buffer);
    });

    let start = Instant::now();
    let mut percent: f32 = 0.0;
    let mut phase = Phase::FAKE;
    let mut scanned: Option<Vec<PathBuf>> = None;
    let mut done_at: Option<Instant> = None;

    loop {
        if scanned.is_none() {
            if let Ok(paths) = receiver.try_recv() {
                scanned = Some(paths);
            }
        }

        match phase {
            Phase::FAKE => {
                let t = start.elapsed().as_secs_f32() / FAKEBUFFERING.as_secs_f32();
                percent = FAKEMAXBUFFERING * (1.0 - (1.0 - t.min(1.0)).powi(3));
                if scanned.is_some() && start.elapsed() >= FAKEBUFFERING {
                    phase = Phase::RUSH;
                }
            }
            Phase::RUSH => {
                percent = (percent + RUSHTOEND).min(100.0);
                if percent >= 100.0 {
                    phase = Phase::DONE;
                    done_at = Some(Instant::now());
                }
            }
            Phase::DONE => {
                if done_at.is_some_and(|t| t.elapsed() >= Duration::from_millis(400)) {
                    break;
                }
            }
        }
        terminal.draw(|f| draw(f, percent))?;
        thread::sleep(Duration::from_millis(16));
    }
    Ok(scanned.unwrap_or_default())
}

fn draw(f: &mut Frame, percent: f32) {
    let area = f.area();
    let [_, title, bar_area, message, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(9),
        Constraint::Length(3),
        Constraint::Length(9),
        Constraint::Fill(1),
    ])
    .areas(area);

    f.render_widget(
        Paragraph::new(TITLE)
            .alignment(Alignment::Center)
            .fg(Color::Cyan)
            .bold(),
        title,
    );

    let [_, bar_center, _] = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Percentage(60),
        Constraint::Percentage(20),
    ])
    .areas(bar_area);

    let label = format!("{:.0}%", percent);
    let gauge = Gauge::default()
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(Line::from(" Buffering ").centered()),
        )
        .gauge_style(Style::new().fg(Color::Green))
        .percent(percent.round().clamp(0.0, 100.0) as u16)
        .label(label);

    f.render_widget(gauge, bar_center);

    f.render_widget(
        Paragraph::new(PREPARE_MESSAGE)
            .alignment(Alignment::Center)
            .fg(Color::DarkGray),
        message,
    );
}
