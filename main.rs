use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

use rand::{distr::weighted::WeightedIndex, prelude::*, rng};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

/// (glifo, nombre, color, valor en puntos, peso base de aparición)
const SYMBOLS: [(&str, &str, Color, u64, u32); 6] = [
    ("●", "Cereza", Color::Red, 1, 40),
    ("◆", "Limón", Color::Yellow, 2, 28),
    ("♣", "Trébol", Color::Green, 3, 18),
    ("♪", "Campana", Color::Cyan, 5, 9),
    ("★", "Estrella", Color::Magenta, 10, 4),
    ("7", "Siete", Color::LightRed, 50, 1),
];

const VISIBLE_ROWS: usize = 5;
const MIDDLE: usize = VISIBLE_ROWS / 2;
const SPIN_STEPS: u32 = 20;

struct Upgrade {
    name: &'static str,
    desc: &'static str,
    base_cost: u64,
    level: u32,
    max: u32,
}

impl Upgrade {
    fn cost(&self) -> u64 {
        (self.base_cost as f64 * 1.7f64.powi(self.level as i32)) as u64
    }
    fn maxed(&self) -> bool {
        self.level >= self.max
    }
}

// Índices dentro de `App::upgrades`
const LUCK: usize = 0;
const MULT: usize = 1;
const PASSIVE: usize = 2;
const AUTO: usize = 3;

struct Spin {
    left: u32,
    result: usize,
    next: Instant,
}

struct App {
    points: u64,
    total_won: u64,
    spins: u64,
    reel: VecDeque<usize>,
    spin: Option<Spin>,
    last: Option<(usize, u64)>,
    upgrades: Vec<Upgrade>,
    selected: usize,
    message: String,
    last_income: Instant,
    auto_at: Instant,
    quit: bool,
    random: ThreadRng,
}

impl App {
    fn new() -> Self {
        let mut random = rng();
        let reel = (0..VISIBLE_ROWS)
            .map(|_| random.random_range(0..SYMBOLS.len()))
            .collect();
        let up = |name, desc, base_cost| Upgrade {
            name,
            desc,
            base_cost,
            level: 0,
            max: 5,
        };
        Self {
            points: 0,
            total_won: 0,
            spins: 0,
            reel,
            spin: None,
            last: None,
            upgrades: vec![
                up("Suerte", "Símbolos raros más probables", 30),
                up("Multiplicador", "+50% a cada premio", 50),
                up("Ingreso pasivo", "+1 punto por segundo", 40),
                up("Auto-palanca", "Tira sola cada cierto tiempo", 60),
            ],
            selected: 0,
            message: String::new(),
            last_income: Instant::now(),
            auto_at: Instant::now(),
            quit: false,
            random,
        }
    }

    // ---------- lógica ----------

    fn pick_result(&mut self) -> usize {
        let luck = self.upgrades[LUCK].level;
        let weights: Vec<u32> = SYMBOLS
            .iter()
            .enumerate()
            .map(|(i, s)| s.4 * (100 + luck * 25 * i as u32))
            .collect();
        WeightedIndex::new(weights)
            .unwrap()
            .sample(&mut self.random)
    }

    fn pull(&mut self) {
        if self.spin.is_some() {
            return;
        }
        let result = self.pick_result();
        self.spin = Some(Spin {
            left: SPIN_STEPS,
            result,
            next: Instant::now(),
        });
    }

    fn finish(&mut self, sym: usize) {
        let mult = 100 + 50 * self.upgrades[MULT].level as u64;
        let payout = SYMBOLS[sym].3 * mult / 100;
        self.points += payout;
        self.total_won += payout;
        self.spins += 1;
        self.last = Some((sym, payout));
        let auto = self.upgrades[AUTO].level as u64;
        self.auto_at = Instant::now() + Duration::from_millis(8000u64.saturating_sub(1500 * auto));
    }

    fn buy(&mut self) {
        let u = &mut self.upgrades[self.selected];
        if u.maxed() {
            self.message = format!(" {} ya está al máximo ", u.name);
        } else if self.points < u.cost() {
            self.message = format!(" Te faltan {} pts ", u.cost() - self.points);
        } else {
            self.points -= u.cost();
            u.level += 1;
            self.message = format!(" ¡{} nivel {}! ", u.name, u.level);
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();

        // ingreso pasivo
        while now.duration_since(self.last_income) >= Duration::from_secs(1) {
            self.last_income += Duration::from_secs(1);
            self.points += self.upgrades[PASSIVE].level as u64;
        }

        // auto-palanca
        if self.spin.is_none() && self.upgrades[AUTO].level > 0 && now >= self.auto_at {
            self.pull();
        }

        // animación del carrete
        if let Some(mut s) = self.spin.take() {
            if now >= s.next {
                // el símbolo ganador entra 2 pasos antes de terminar => queda en el centro
                let sym = if s.left == MIDDLE as u32 + 1 {
                    s.result
                } else {
                    self.random.random_range(0..SYMBOLS.len())
                };
                self.reel.push_front(sym);
                self.reel.pop_back();
                s.left -= 1;
                if s.left == 0 {
                    self.finish(s.result);
                    return;
                }
                let step = (SPIN_STEPS - s.left) as u64;
                s.next = now + Duration::from_millis(35 + 5 * step);
            }
            self.spin = Some(s);
        }
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char(' ') => self.pull(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.upgrades.len() - 1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % self.upgrades.len()
            }
            KeyCode::Enter | KeyCode::Char('b') => self.buy(),
            _ => {}
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|f| self.draw(f))?;
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        self.on_key(k.code);
                    }
                }
            }
            self.tick();
        }
        Ok(())
    }

    // ---------- dibujo ----------

    fn draw(&self, f: &mut Frame) {
        let [main, footer] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(f.area());
        let [left, center, right] = Layout::horizontal([
            Constraint::Percentage(28),
            Constraint::Percentage(34),
            Constraint::Percentage(38),
        ])
        .areas(main);

        self.draw_lever(f, left);
        self.draw_reel(f, center);
        self.draw_store(f, right);

        f.render_widget(
            Paragraph::new(" Espacio: palanca   ↑↓: elegir mejora   Enter: comprar   q: salir")
                .style(Style::new().fg(Color::DarkGray)),
            footer,
        );
    }

    fn panel(title: &str) -> Block<'_> {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(Line::from(format!(" {title} ")).bold().centered())
    }

    fn draw_lever(&self, f: &mut Frame, area: Rect) {
        let block = Self::panel("Palanca");
        let inner = block.inner(area);
        f.render_widget(block, area);
        let [lever_area, stats_area] =
            Layout::vertical([Constraint::Length(9), Constraint::Min(0)]).areas(inner);

        // la palanca baja mientras arranca el giro
        let down = self.spin.as_ref().is_some_and(|s| s.left > SPIN_STEPS - 4);
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
                Span::raw(" Puntos: "),
                Span::styled(
                    self.points.to_string(),
                    Style::new().fg(Color::Yellow).bold(),
                ),
            ]),
            Line::from(format!(" Total ganado: {}", self.total_won)),
            Line::from(format!(" Tiradas: {}", self.spins)),
            Line::raw(""),
            Line::styled(
                " Último resultado:",
                Style::new().add_modifier(Modifier::UNDERLINED),
            ),
        ];
        match self.last {
            Some((sym, pay)) => {
                let (glyph, name, color, ..) = SYMBOLS[sym];
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(format!("{glyph} {name}"), Style::new().fg(color).bold()),
                    Span::styled(format!("  +{pay}"), Style::new().fg(Color::Green)),
                ]));
            }
            None => lines.push(Line::styled(" —", Style::new().fg(Color::DarkGray))),
        }
        if self.upgrades[PASSIVE].level > 0 {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                format!(" +{}/s pasivo", self.upgrades[PASSIVE].level),
                Style::new().fg(Color::DarkGray),
            ));
        }
        f.render_widget(Paragraph::new(lines), stats_area);
    }

    fn draw_reel(&self, f: &mut Frame, area: Rect) {
        let block = Self::panel("Slot");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::vertical([Constraint::Fill(1); VISIBLE_ROWS]).split(inner);
        for (i, (&sym, &row)) in self.reel.iter().zip(rows.iter()).enumerate() {
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

            let (glyph, name, color, ..) = SYMBOLS[sym];
            let mid = i == MIDDLE;
            let mut style = Style::new().fg(color);
            if mid {
                style = style.bold().bg(Color::Rgb(40, 40, 40));
            } else {
                style = style.add_modifier(Modifier::DIM);
            }
            let mut spans = vec![];
            if mid {
                spans.push(Span::styled("► ", Style::new().fg(Color::Yellow).bold()));
            }
            spans.push(Span::styled(format!("{glyph} {name}"), style));
            if mid {
                spans.push(Span::styled(" ◄", Style::new().fg(Color::Yellow).bold()));
            }

            // centrado vertical dentro de la celda
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

    fn draw_store(&self, f: &mut Frame, area: Rect) {
        let mut block = Self::panel("Tienda");
        if !self.message.is_empty() {
            block = block.title_bottom(Line::from(self.message.as_str()).yellow().centered());
        }
        let items: Vec<ListItem> = self
            .upgrades
            .iter()
            .map(|u| {
                let price = if u.maxed() {
                    Span::styled("MAX", Style::new().fg(Color::Magenta).bold())
                } else if self.points >= u.cost() {
                    Span::styled(format!("{} pts", u.cost()), Style::new().fg(Color::Green))
                } else {
                    Span::styled(format!("{} pts", u.cost()), Style::new().fg(Color::Red))
                };
                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(u.name, Style::new().bold()),
                        Span::styled(
                            format!("  Nv {}/{}", u.level, u.max),
                            Style::new().fg(Color::Cyan),
                        ),
                    ]),
                    Line::from(vec![Span::styled(u.desc, Style::new().fg(Color::Gray))]),
                    Line::from(vec![Span::raw("Costo: "), price]),
                    Line::raw(""),
                ])
            })
            .collect();
        let list = List::new(items)
            .block(block)
            .highlight_symbol("▶ ")
            .highlight_style(Style::new().bg(Color::Rgb(40, 40, 40)));
        let mut state = ListState::default().with_selected(Some(self.selected));
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn spin_lands_result_in_middle_and_renders() {
        let mut app = App::new();
        let mut term = Terminal::new(TestBackend::new(100, 32)).unwrap();
        app.pull();
        let expected = app.spin.as_ref().unwrap().result;
        while app.spin.is_some() {
            app.tick();
            term.draw(|f| app.draw(f)).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(app.reel[MIDDLE], expected);
        assert!(app.last.is_some());
        println!("{}", term.backend());
    }
}
