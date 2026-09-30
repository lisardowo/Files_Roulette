use std::{
    collections::VecDeque,
    io,
    time::{Duration, Instant},
};

use rand::{distr::weighted::WeightedIndex, prelude::*, rng};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    style::Color,
};

use crate::handleFile;
use crate::ui;

pub const SYMBOLS: [(&str, Color, u64, u32); 6] = [
    //name, Color, points, odds
    ("a", Color::Red, 1, 40),
    ("b", Color::Yellow, 2, 28),
    ("c", Color::Green, 3, 18),
    ("d", Color::Cyan, 5, 9),
    ("e", Color::Magenta, 10, 4),
    ("f", Color::LightRed, 50, 1),
];

pub const VISIBLE_ROWS: usize = 5;
pub const MIDDLE: usize = VISIBLE_ROWS / 2;
pub const SPIN_STEPS: u32 = 20;

pub struct Upgrade {
    pub name: &'static str,
    pub desc: &'static str,
    pub base_cost: u64,
    pub level: u32,
    pub max: u32,
}

impl Upgrade {
    pub fn cost(&self) -> u64 {
        (self.base_cost as f64 * 1.7f64.powi(self.level as i32)) as u64
    }
    pub fn maxed(&self) -> bool {
        self.level >= self.max
    }
}

pub const LUCK: usize = 0;
pub const MULT: usize = 1;
pub const PASSIVE: usize = 2;
pub const AUTO: usize = 3;

pub struct Spin {
    pub left: u32,
    pub result: usize,
    pub next: Instant,
}

pub struct App {
    pub points: u64,
    pub total_won: u64,
    pub spins: u64,
    pub reel: VecDeque<usize>,
    pub spin: Option<Spin>,
    pub last: Option<(usize, u64)>,
    pub upgrades: Vec<Upgrade>,
    pub selected: usize,
    pub message: String,
    last_income: Instant,
    auto_at: Instant,
    quit: bool,
    rng: ThreadRng,
}

impl App {
    pub fn new() -> Self {
        let mut rng = rng();
        let reel = (0..VISIBLE_ROWS)
            .map(|_| rng.random_range(0..SYMBOLS.len()))
            .collect();
        let up = |name, desc, base_cost| Upgrade {
            //TODO
            //unused up
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
                up("foo", "desc", 67),
                up("foo", "desc", 67),
                up("foo", "desc", 67),
                up("foo", "desc", 67),
                up("foo", "desc", 67),
            ],
            selected: 0,
            message: String::new(),
            last_income: Instant::now(),
            auto_at: Instant::now(),
            quit: false,
            rng,
        }
    }

    fn pick_result(&mut self) -> usize {
        let luck = self.upgrades[LUCK].level;
        let weights: Vec<u32> = SYMBOLS
            .iter()
            .enumerate()
            .map(|(i, s)| s.3 * (100 + luck * 25 * i as u32))
            .collect();
        WeightedIndex::new(weights).unwrap().sample(&mut self.rng)
    }

    pub fn pull(&mut self) {
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
        let payout = SYMBOLS[sym].2 * mult / 100;
        self.points += payout;
        self.total_won += payout;
        self.spins += 1;
        self.last = Some((sym, payout));
        let auto = self.upgrades[AUTO].level as u64;
        self.auto_at = Instant::now() + Duration::from_millis(8000u64.saturating_sub(1500 * auto));
    }

    pub fn buy(&mut self) {
        let u = &mut self.upgrades[self.selected];
        if u.maxed() {
            self.message = format!(" {} maxed ", u.name);
        } else if self.points < u.cost() {
            self.message = format!(" You need {} pts ", u.cost());
        } else {
            self.points -= u.cost();
            u.level += 1;
            self.message = format!(" {} level {}! ", u.name, u.level);
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();

        while now.duration_since(self.last_income) >= Duration::from_secs(1) {
            self.last_income += Duration::from_secs(1);
            self.points += self.upgrades[PASSIVE].level as u64;
        }

        if self.spin.is_none() && self.upgrades[AUTO].level > 0 && now >= self.auto_at {
            self.pull();
        }

        if let Some(mut s) = self.spin.take() {
            if now >= s.next {
                let sym = if s.left == MIDDLE as u32 + 1 {
                    s.result
                } else {
                    self.rng.random_range(0..SYMBOLS.len())
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

    pub fn lever_down(&self) -> bool {
        self.spin.as_ref().is_some_and(|s| s.left > SPIN_STEPS - 4)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|f| ui::draw(self, f))?;
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
}
