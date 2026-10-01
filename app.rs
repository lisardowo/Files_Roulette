use std::{
    collections::VecDeque,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use rand::{prelude::*, rng};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
};

use crate::handleFile;
use crate::ui;

pub const VISIBLE_ROWS: usize = 5;
pub const MIDDLE: usize = VISIBLE_ROWS / 2;
pub const SPIN_STEPS: u32 = 20;
pub const BASE_PAYOUT: u64 = 10;

pub const BURST_OPTIONS: [u32; 3] = [5, 10, 15];
const BURST_BONUS_PER_BLOCK: f64 = 0.10;
const BURST_BLOCK_SIZE: u32 = 5;
const STREAK_UPGRADE_BONUS: f64 = 0.03;

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

pub const STREAK: usize = 0;
pub const MULT: usize = 1;
pub const PASSIVE: usize = 2;
pub const AUTO: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Store,
    Burst,
}

pub struct Spin {
    pub left: u32,
    pub result_path: usize,
    pub next: Instant,
}

pub struct BurstState {
    pub total: u32,
    pub remaining: u32,
    pub accumulated: u64,
}

pub struct App {
    pub points: u64,
    pub total_won: u64,
    pub spins: u64,

    pub reel: VecDeque<usize>,

    pub paths: Vec<String>,
    pub spin: Option<Spin>,

    pub last: Option<u64>,
    pub last_path: Option<String>,
    pub burst: Option<BurstState>,
    pub upgrades: Vec<Upgrade>,
    pub focus: Focus,
    pub selected: usize,
    pub burst_selected: usize,
    pub message: String,
    last_income: Instant,
    auto_at: Instant,
    quit: bool,
    rng: ThreadRng,
}

impl App {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        let mut paths: Vec<String> = paths
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if paths.is_empty() {
            paths.push("(sin archivos)".to_string());
        }

        let mut rng = rng();
        let reel = (0..VISIBLE_ROWS)
            .map(|_| rng.random_range(0..paths.len()))
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
            paths,
            spin: None,
            last: None,
            last_path: None,
            burst: None,
            upgrades: vec![
                up("foo", "desc", 67),
                up("foo", "desc", 67),
                up("foo", "desc", 67),
                up("foo", "desc", 67),
            ],
            focus: Focus::Store,
            selected: 0,
            burst_selected: 0,
            message: String::new(),
            last_income: Instant::now(),
            auto_at: Instant::now(),
            quit: false,
            rng,
        }
    }

    fn random_path_index(&mut self) -> usize {
        self.rng.random_range(0..self.paths.len())
    }

    fn unit_payout(&self) -> u64 {
        let mult = 100 + 50 * self.upgrades[MULT].level as u64;
        BASE_PAYOUT * mult / 100
    }

    fn burst_bonus_pct(&self, n: u32) -> f64 {
        let blocks = n as f64 / BURST_BLOCK_SIZE as f64;
        let from_blocks = blocks * BURST_BONUS_PER_BLOCK;
        let from_upgrade = self.upgrades[STREAK].level as f64 * STREAK_UPGRADE_BONUS;
        from_blocks + from_upgrade
    }

    pub fn burst_preview(&self, n: u32) -> (f64, u64) {
        let pct = self.burst_bonus_pct(n);
        let base_total = self.unit_payout() * n as u64;
        let bonus = (base_total as f64 * pct).round() as u64;
        (pct, base_total + bonus)
    }

    pub fn start_burst(&mut self, n: u32) {
        if self.spin.is_some() || self.burst.is_some() {
            return;
        }
        self.burst = Some(BurstState {
            total: n,
            remaining: n,
            accumulated: 0,
        });
        self.pull();
    }

    pub fn pull(&mut self) {
        if self.spin.is_some() {
            return;
        }
        let result_path = self.random_path_index();
        self.spin = Some(Spin {
            left: SPIN_STEPS,
            result_path,
            next: Instant::now(),
        });
    }

    fn finish(&mut self) {
        self.spins += 1;
        self.last_path = self.reel.get(MIDDLE).map(|idx| self.paths[*idx].clone());
        let unit = self.unit_payout();

        match self.burst.as_mut() {
            Some(b) => {
                b.accumulated += unit;
                b.remaining -= 1;
                if b.remaining == 0 {
                    let total = b.total;
                    let accumulated = b.accumulated;
                    let pct = self.burst_bonus_pct(total);
                    let payout = accumulated + (accumulated as f64 * pct).round() as u64;
                    self.points += payout;
                    self.total_won += payout;
                    self.last = Some(payout);
                    self.message = format!(
                        " ¡Spins {total} completed! +{payout} (bonus {:.0}%) ",
                        pct * 100.0
                    );
                    self.burst = None;
                } else {
                    self.pull();
                }
            }
            None => {
                self.points += unit;
                self.total_won += unit;
                self.last = Some(unit);
            }
        }

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

        if self.spin.is_none()
            && self.burst.is_none()
            && self.upgrades[AUTO].level > 0
            && now >= self.auto_at
        {
            self.pull();
        }

        if let Some(mut s) = self.spin.take() {
            if now >= s.next {
                let landing = s.left == MIDDLE as u32 + 1;
                let path_idx = if landing {
                    s.result_path
                } else {
                    self.random_path_index()
                };
                self.reel.push_front(path_idx);
                //delete
                //let address_to_delete = self.paths.remove(path_idx);
                let address_to_delete = &self.paths[path_idx];
                handleFile::delete_file(PathBuf::from(&address_to_delete));
                self.reel.pop_back();
                s.left -= 1;
                if s.left == 0 {
                    self.finish();
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
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Store => Focus::Burst,
                    Focus::Burst => Focus::Store,
                }
            }
            KeyCode::Up | KeyCode::Char('k') => match self.focus {
                Focus::Store => {
                    self.selected = self
                        .selected
                        .checked_sub(1)
                        .unwrap_or(self.upgrades.len() - 1)
                }
                Focus::Burst => {
                    self.burst_selected = self
                        .burst_selected
                        .checked_sub(1)
                        .unwrap_or(BURST_OPTIONS.len() - 1)
                }
            },
            KeyCode::Down | KeyCode::Char('j') => match self.focus {
                Focus::Store => self.selected = (self.selected + 1) % self.upgrades.len(),
                Focus::Burst => {
                    self.burst_selected = (self.burst_selected + 1) % BURST_OPTIONS.len()
                }
            },
            KeyCode::Enter | KeyCode::Char('b') => match self.focus {
                Focus::Store => self.buy(),
                Focus::Burst => self.start_burst(BURST_OPTIONS[self.burst_selected]),
            },
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
