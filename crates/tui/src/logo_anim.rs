use ratatui::style::Color;
use std::time::Instant;

use crate::logo::LOGO_BRAILLE;

const BRAILLE_BASE: u32 = 0x2800;
const ASSEMBLE_DURATION_MS: u64 = 2000;
const BREATH_CYCLE_MS: u64 = 3000;
const TICK_MS: u64 = 50;

/// Pre-parsed braille cell: (row, col, dot_bits) for every non-blank cell in the logo.
struct Cell {
    row: usize,
    col: usize,
    bits: u8,
    reveal_ms: u64,
}

pub struct LogoAnimation {
    cells: Vec<Cell>,
    rows: usize,
    cols: usize,
    started: Instant,
    last_tick: Instant,
    rng_state: u64,
}

impl LogoAnimation {
    pub fn new() -> Self {
        let mut anim = Self {
            cells: Vec::new(),
            rows: 0,
            cols: 0,
            started: Instant::now(),
            last_tick: Instant::now(),
            rng_state: 0xDEAD_BEEF_CAFE_1337,
        };
        anim.parse_logo();
        anim
    }

    fn next_rand(&mut self) -> u64 {
        // xorshift64
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    fn parse_logo(&mut self) {
        let lines: Vec<&str> = LOGO_BRAILLE.lines().filter(|l| !l.is_empty()).collect();
        self.rows = lines.len();
        self.cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

        for (row, line) in lines.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let code = ch as u32;
                if code >= BRAILLE_BASE {
                    let bits = (code - BRAILLE_BASE) as u8;
                    if bits != 0 {
                        let reveal = self.next_rand() % ASSEMBLE_DURATION_MS;
                        self.cells.push(Cell {
                            row,
                            col,
                            bits,
                            reveal_ms: reveal,
                        });
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
    }

    /// Returns true if the assembly animation is still running.
    pub fn is_assembling(&self) -> bool {
        self.started.elapsed().as_millis() < ASSEMBLE_DURATION_MS as u128
    }

    /// Get the current frame's logo lines as owned Strings.
    pub fn current_frame(&self) -> Vec<String> {
        let elapsed = self.started.elapsed().as_millis() as u64;

        // Build a grid of braille codes (all blank initially)
        let mut grid: Vec<Vec<u8>> = vec![vec![0u8; self.cols]; self.rows];

        if elapsed >= ASSEMBLE_DURATION_MS {
            // Fully assembled - use the real bits
            for cell in &self.cells {
                grid[cell.row][cell.col] = cell.bits;
            }
        } else {
            // Assembly phase: only reveal dots whose individual bits have "arrived"
            for cell in &self.cells {
                if elapsed >= cell.reveal_ms {
                    grid[cell.row][cell.col] |= cell.bits;
                } else {
                    // Partial: reveal random subset of the 8 dots based on time
                    let progress = elapsed as f32 / cell.reveal_ms as f32;
                    let mut partial = 0u8;
                    for bit_idx in 0..8u8 {
                        let mask = 1u8 << bit_idx;
                        if cell.bits & mask != 0 {
                            // Each dot within a cell reveals at a fraction of the cell's reveal time
                            let dot_threshold = (bit_idx as f32 + 1.0) / 9.0;
                            if progress > dot_threshold {
                                partial |= mask;
                            }
                        }
                    }
                    grid[cell.row][cell.col] |= partial;
                }
            }
        }

        grid.iter()
            .map(|row| {
                let s: String = row
                    .iter()
                    .map(|&bits| char::from_u32(BRAILLE_BASE + bits as u32).unwrap_or(' '))
                    .collect();
                s.trim_end_matches(char::from_u32(BRAILLE_BASE).unwrap_or(' '))
                    .to_string()
            })
            .collect()
    }

    /// Compute the current breathing color: oscillates between accent and a dimmer version.
    pub fn breathing_color(&self, accent: Color) -> Color {
        let elapsed = self.started.elapsed().as_millis() as u64;
        if elapsed < ASSEMBLE_DURATION_MS {
            return accent;
        }

        let breath_elapsed = elapsed - ASSEMBLE_DURATION_MS;
        let cycle_pos = (breath_elapsed % BREATH_CYCLE_MS) as f64 / BREATH_CYCLE_MS as f64;
        // Smooth sine wave: ranges from 0.0 to 1.0 and back
        let factor = ((cycle_pos * std::f64::consts::PI * 2.0).sin() + 1.0) / 2.0;
        // Interpolate between 60% brightness and 100% brightness
        let mix = 0.6 + 0.4 * factor;

        if let Color::Rgb(r, g, b) = accent {
            Color::Rgb(
                (r as f64 * mix) as u8,
                (g as f64 * mix) as u8,
                (b as f64 * mix) as u8,
            )
        } else {
            accent
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn needs_redraw(&self) -> bool {
        self.last_tick.elapsed().as_millis() >= TICK_MS as u128
    }
}

impl Default for LogoAnimation {
    fn default() -> Self {
        Self::new()
    }
}
