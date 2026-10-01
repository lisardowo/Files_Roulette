use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::app::{App, BURST_OPTIONS, Focus, MIDDLE, VISIBLE_ROWS};

pub fn draw(app: &App, f: &mut Frame) {
    let [main, footer] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(f.area());
    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(28),
        Constraint::Percentage(34),
        Constraint::Percentage(38),
    ])
    .areas(main);

    draw_lever(app, f, left);
    draw_reel(app, f, center);
    draw_store(app, f, right);

    f.render_widget(
        Paragraph::new(
            "TAB: Change Focus Space: lever j/k: Choose Power Up Enter: Buy power up q: exit",
        )
        .style(Style::new().fg(Color::DarkGray)),
        footer,
    );
}

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(Line::from(format!(" {title} ")).bold().centered())
}

fn focusable_panel(title: &str, focused: bool) -> Block<'_> {
    let color = if focused {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(color))
        .title(Line::from(format!(" {title} ")).bold().centered())
}

fn shorten_path(path: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max {
        return path.to_string();
    }
    let tail: String = chars[chars.len() - (max - 1)..].iter().collect();
    format!("..{tail}")
}
fn draw_lever(app: &App, f: &mut Frame, area: Rect) {
    let block = panel("Lever");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [lever_area, stats_area, burst_area] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Min(0),
        Constraint::Length(5),
    ])
    .areas(inner);
    let down = app.lever_down();
    let knob = Style::new().fg(Color::Red).bold();
    let rod = Style::new().fg(Color::Gray);
    let base = Style::new().fg(Color::DarkGray);
    let lever: Vec<Line> = if down {
        vec![
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::styled("═══╤═══", base),
            Line::styled("   │", rod),
            Line::styled("   │", rod),
            Line::styled("   ●", knob),
            Line::raw(""),
        ]
    } else {
        vec![
            Line::styled("   ●", knob),
            Line::styled("   │", rod),
            Line::styled("   │", rod),
            Line::styled("═══╧═══", base),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
            Line::raw(""),
        ]
    };

    f.render_widget(
        Paragraph::new(lever).alignment(Alignment::Center),
        lever_area,
    );

    let mut lines = vec![
        Line::from(vec![
            Span::raw(" Points: "),
            Span::styled(
                app.points.to_string(),
                Style::new().fg(Color::Yellow).bold(),
            ),
        ]),
        Line::from(format!(" Total points: {}", app.total_won)),
        Line::from(format!(" Spin #: {}", app.spins)),
        Line::raw(""),
        Line::styled(
            " Last Pulls: :",
            Style::new().add_modifier(Modifier::UNDERLINED),
        ),
    ];

    match (&app.burst, &app.last) {
        (Some(b), _) => lines.push(Line::styled(
            format!(" Spins sequenced: {}/{}", b.total - b.remaining, b.total),
            Style::new().fg(Color::Cyan),
        )),
        (None, Some(pay)) => {
            let path = app.last_path.as_deref().unwrap_or("—");
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(shorten_path(path, 22), Style::new().fg(Color::Green).bold()),
            ]));
            lines.push(Line::styled(
                format!(" +{pay}"),
                Style::new().fg(Color::Green),
            ));
        }
        (None, None) => lines.push(Line::styled(" —", Style::new().fg(Color::DarkGray))),
    }
    if app.upgrades[crate::app::PASSIVE].level > 0 {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!(" +{}/s powerups", app.upgrades[crate::app::PASSIVE].level),
            Style::new().fg(Color::DarkGray),
        ));
    }

    f.render_widget(Paragraph::new(lines), stats_area);

    draw_burst_menu(app, f, burst_area);
}

fn draw_burst_menu(app: &App, f: &mut Frame, area: Rect) {
    let focused = app.focus == Focus::Burst;
    let block = focusable_panel("Racha", focused);

    let items: Vec<ListItem> = BURST_OPTIONS
        .iter()
        .map(|&n| {
            let (pct, total) = app.burst_preview(n);
            ListItem::new(format!(" x{n}  ~{total} pts  (+{:.0}%)", pct * 100.0))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol(if focused { "▶ " } else { "  " })
        .highlight_style(if focused {
            Style::new()
                .bg(Color::Rgb(40, 40, 40))
                .fg(Color::Yellow)
                .bold()
        } else {
            Style::new().fg(Color::Gray)
        });
    let mut state = ListState::default().with_selected(Some(app.burst_selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_reel(app: &App, f: &mut Frame, area: Rect) {
    let block = panel("Slot");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::vertical([Constraint::Fill(1); VISIBLE_ROWS]).split(inner);

    for (i, (&path_idx, &row)) in app.reel.iter().zip(rows.iter()).enumerate() {
        let last = i == VISIBLE_ROWS - 1;
        let borders = if last {
            Borders::TOP | Borders::BOTTOM
        } else {
            Borders::TOP
        };
        let cell = Block::default()
            .borders(borders)
            .border_style(Style::new().fg(Color::DarkGray));
        let cell_inner = cell.inner(row);
        f.render_widget(cell, row);

        let mid = i == MIDDLE;
        let style = if mid {
            Style::new()
                .fg(Color::Yellow)
                .bold()
                .bg(Color::Rgb(40, 40, 40))
        } else {
            Style::new().fg(Color::Gray).add_modifier(Modifier::DIM)
        };
        let mut spans = vec![];
        if mid {
            spans.push(Span::styled("► ", Style::new().fg(Color::Yellow).bold()));
        }
        let avail = cell_inner
            .width
            .saturating_sub(if mid { 4 } else { 0 })
            .max(1) as usize;
        let text = shorten_path(&app.paths[path_idx], avail);
        spans.push(Span::styled(text, style));
        if mid {
            spans.push(Span::styled(" ◄", Style::new().fg(Color::Yellow).bold()));
        }

        let pad = cell_inner.height.saturating_sub(1) / 2;
        let mut lines: Vec<Line> = (0..pad).map(|_| Line::raw("")).collect();
        lines.push(Line::from(spans));
        let mut p = Paragraph::new(lines).alignment(Alignment::Center);
        if mid {
            p = p.style(Style::new().bg(Color::Rgb(40, 40, 40)));
        }
        f.render_widget(p, cell_inner);
    }
}

fn draw_store(app: &App, f: &mut Frame, area: Rect) {
    let mut block = panel("Store");
    if !app.message.is_empty() {
        block = block.title_bottom(Line::from(app.message.as_str()).yellow().centered());
    }
    let items: Vec<ListItem> = app
        .upgrades
        .iter()
        .map(|u| {
            let price = if u.maxed() {
                Span::styled("MAX", Style::new().fg(Color::Magenta).bold())
            } else if app.points >= u.cost() {
                Span::styled(format!("{} pts", u.cost()), Style::new().fg(Color::Green))
            } else {
                Span::styled(format!("{} pts", u.cost()), Style::new().fg(Color::Red))
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(u.name, Style::new().bold()),
                    Span::styled(
                        format!(" lvl {}/{}", u.level, u.max),
                        Style::new().fg(Color::Cyan),
                    ),
                ]),
                Line::from(vec![Span::styled(u.desc, Style::new().fg(Color::Gray))]),
                Line::from(vec![Span::raw("Price: "), price]),
                Line::raw(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(Style::new().bg(Color::Rgb(40, 40, 40)));
    let mut state = ListState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}
