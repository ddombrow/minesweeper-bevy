use rand::prelude::*;

#[derive(Clone, Default)]
struct CellData {
    mine: bool,
    revealed: bool,
    flagged: bool,
    questioned: bool,
    neighbor_mines: u8,
}

pub struct MinesweeperBoard {
    width: u16,
    height: u16,
    mine_count: usize,
    cells: Vec<Vec<CellData>>, // [x][y]
    first_click: bool,
    flags_placed: usize,
}

pub enum RevealOutcome {
    Ignored,
    Safe,
    HitMine,
}

pub enum CellFace {
    Hidden,
    Flagged,
    RevealedEmpty,
    RevealedNumber(u8),
    RevealedMine,
}

pub struct CellView {
    pub face: CellFace,
}

impl MinesweeperBoard {
    pub fn new(width: u16, height: u16, mine_count: usize) -> Self {
        Self {
            width,
            height,
            mine_count,
            cells: vec![vec![CellData::default(); height as usize]; width as usize],
            first_click: true,
            flags_placed: 0,
        }
    }

    pub fn width(&self) -> usize {
        self.width as usize
    }

    pub fn height(&self) -> usize {
        self.height as usize
    }

    pub fn mine_count(&self) -> usize {
        self.mine_count
    }

    pub fn flags_placed(&self) -> usize {
        self.flags_placed
    }

    pub fn is_revealed(&self, x: usize, y: usize) -> bool {
        self.cells[x][y].revealed
    }

    pub fn is_flagged(&self, x: usize, y: usize) -> bool {
        self.cells[x][y].flagged
    }

    pub fn cell_view(&self, x: usize, y: usize) -> CellView {
        let cell = &self.cells[x][y];
        let face = if cell.flagged && !cell.revealed {
            CellFace::Flagged
        } else if cell.revealed && cell.mine {
            CellFace::RevealedMine
        } else if cell.revealed && cell.neighbor_mines > 0 {
            CellFace::RevealedNumber(cell.neighbor_mines)
        } else if cell.revealed {
            CellFace::RevealedEmpty
        } else {
            CellFace::Hidden
        };

        CellView { face }
    }

    pub fn reveal_at(&mut self, x: usize, y: usize) -> RevealOutcome {
        if self.is_flagged(x, y) || self.is_revealed(x, y) {
            return RevealOutcome::Ignored;
        }

        if self.first_click {
            self.first_click = false;
            self.place_mines(x, y);
        }

        if self.reveal(x, y) {
            RevealOutcome::HitMine
        } else {
            RevealOutcome::Safe
        }
    }

    pub fn place_mines(&mut self, safe_x: usize, safe_y: usize) {
        let mut rng = thread_rng();
        let mut placed = 0usize;
        let safe_x = safe_x as u16;
        let safe_y = safe_y as u16;

        while placed < self.mine_count {
            let x = rng.gen_range(0..self.width);
            let y = rng.gen_range(0..self.height);

            // Keep a safe zone around the first click.
            if x.abs_diff(safe_x) <= 1 && y.abs_diff(safe_y) <= 1 {
                continue;
            }

            if !self.cells[x as usize][y as usize].mine {
                self.cells[x as usize][y as usize].mine = true;
                placed += 1;
            }
        }

        for x in 0..self.width {
            for y in 0..self.height {
                if !self.cells[x as usize][y as usize].mine {
                    self.cells[x as usize][y as usize].neighbor_mines =
                        self.count_mine_neighbors(x, y);
                }
            }
        }
    }

    fn count_mine_neighbors(&self, x: u16, y: u16) -> u8 {
        self.neighbor_positions(x as usize, y as usize)
            .into_iter()
            .filter(|&(nx, ny)| self.cells[nx][ny].mine)
            .count() as u8
    }

    pub fn reveal(&mut self, x: usize, y: usize) -> bool {
        if self.cells[x][y].flagged || self.cells[x][y].revealed {
            return false;
        }

        if self.cells[x][y].mine {
            self.reveal_all_mines();
            return true;
        }

        self.flood_fill(x, y);
        false
    }

    fn flood_fill(&mut self, x: usize, y: usize) {
        if self.cells[x][y].revealed || self.cells[x][y].flagged {
            return;
        }

        self.cells[x][y].revealed = true;

        if self.cells[x][y].neighbor_mines == 0 {
            for (nx, ny) in self.neighbor_positions(x, y) {
                self.flood_fill(nx, ny);
            }
        }
    }

    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.cells[x][y].revealed {
            return;
        }

        if self.cells[x][y].flagged {
            self.cells[x][y].flagged = false;
            self.flags_placed = self.flags_placed.saturating_sub(1);
        } else {
            self.cells[x][y].flagged = true;
            self.flags_placed += 1;
        }
    }

    pub fn check_win(&self) -> bool {
        self.cells
            .iter()
            .flatten()
            .filter(|cell| !cell.mine)
            .all(|cell| cell.revealed)
    }

    pub fn adjacent_flag_count(&self, x: usize, y: usize) -> u8 {
        self.neighbor_positions(x, y)
            .into_iter()
            .filter(|&(nx, ny)| self.cells[nx][ny].flagged)
            .count() as u8
    }

    pub fn hidden_unflagged_neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        self.neighbor_positions(x, y)
            .into_iter()
            .filter(|&(nx, ny)| !self.cells[nx][ny].revealed && !self.cells[nx][ny].flagged)
            .collect()
    }

    pub fn chord_reveal_candidates(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let cell = &self.cells[x][y];
        if !cell.revealed || cell.mine || cell.neighbor_mines == 0 {
            return Vec::new();
        }

        if self.adjacent_flag_count(x, y) != cell.neighbor_mines {
            return Vec::new();
        }

        self.hidden_unflagged_neighbors(x, y)
    }

    /// Chord-reveal: if the clicked cell is a revealed number and its adjacent
    /// flag count equals its mine count, reveal all non-flagged hidden neighbors.
    /// Returns true if any mine was hit.
    pub fn chord(&mut self, x: usize, y: usize) -> bool {
        let mut hit_mine = false;
        for (nx, ny) in self.chord_reveal_candidates(x, y) {
            if self.reveal(nx, ny) {
                hit_mine = true;
            }
        }

        hit_mine
    }

    fn reveal_all_mines(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                if cell.mine {
                    cell.revealed = true;
                }
            }
        }
    }

    fn neighbor_positions(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut neighbors = Vec::with_capacity(8);

        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < self.width()
                    && (ny as usize) < self.height()
                {
                    neighbors.push((nx as usize, ny as usize));
                }
            }
        }

        neighbors
    }
}
