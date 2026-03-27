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
    exploded_mine: Option<(usize, usize)>,
}

pub enum RevealOutcome {
    Ignored,
    Safe,
    HitMine,
}

pub enum CellState {
    Hidden,
    Flagged,
    Questioned,
    RevealedEmpty,
    RevealedNumber(u8),
    RevealedMine,
    ExplodedMine,
}

pub struct CellSnapshot {
    pub state: CellState,
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
            exploded_mine: None,
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

    pub fn cell_snapshot(&self, x: usize, y: usize) -> CellSnapshot {
        let cell = &self.cells[x][y];
        let state = if self.exploded_mine == Some((x, y)) {
            CellState::ExplodedMine
        } else if cell.flagged && !cell.revealed {
            CellState::Flagged
        } else if cell.questioned && !cell.revealed {
            CellState::Questioned
        } else if cell.revealed && cell.mine {
            CellState::RevealedMine
        } else if cell.revealed && cell.neighbor_mines > 0 {
            CellState::RevealedNumber(cell.neighbor_mines)
        } else if cell.revealed {
            CellState::RevealedEmpty
        } else {
            CellState::Hidden
        };

        CellSnapshot { state }
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
            self.exploded_mine = Some((x, y));
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
            self.cells[x][y].questioned = true;
            self.flags_placed = self.flags_placed.saturating_sub(1);
        } else if self.cells[x][y].questioned {
            self.cells[x][y].questioned = false;
        } else {
            self.cells[x][y].flagged = true;
            self.cells[x][y].questioned = false;
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

    pub fn finalize_win(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                if cell.mine {
                    cell.flagged = true;
                    cell.questioned = false;
                }
            }
        }
        self.flags_placed = self.mine_count;
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

    pub fn chord_hint_cells(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let cell = &self.cells[x][y];
        if !cell.revealed || cell.mine || cell.neighbor_mines == 0 {
            return Vec::new();
        }

        self.hidden_unflagged_neighbors(x, y)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn board_with_mines(width: u16, height: u16, mines: &[(usize, usize)]) -> MinesweeperBoard {
        let mut board = MinesweeperBoard::new(width, height, mines.len());
        board.first_click = false;

        for &(x, y) in mines {
            board.cells[x][y].mine = true;
        }

        recompute_neighbors(&mut board);
        board
    }

    fn recompute_neighbors(board: &mut MinesweeperBoard) {
        for x in 0..board.width() {
            for y in 0..board.height() {
                board.cells[x][y].neighbor_mines = if board.cells[x][y].mine {
                    0
                } else {
                    board.count_mine_neighbors(x as u16, y as u16)
                };
            }
        }
    }

    fn sorted(mut cells: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        cells.sort_unstable();
        cells
    }

    #[test]
    fn new_board_starts_with_expected_dimensions_and_counts() {
        let board = MinesweeperBoard::new(8, 10, 12);

        assert_eq!(board.width(), 8);
        assert_eq!(board.height(), 10);
        assert_eq!(board.mine_count(), 12);
        assert_eq!(board.flags_placed(), 0);
        assert!(board.first_click);
    }

    #[test]
    fn place_mines_respects_safe_zone_and_total_count() {
        let mut board = MinesweeperBoard::new(6, 6, 8);

        board.place_mines(2, 2);

        let total_mines = board
            .cells
            .iter()
            .flatten()
            .filter(|cell| cell.mine)
            .count();
        assert_eq!(total_mines, 8);

        for x in 1..=3 {
            for y in 1..=3 {
                assert!(!board.cells[x][y].mine);
            }
        }

        for x in 0..board.width() {
            for y in 0..board.height() {
                if !board.cells[x][y].mine {
                    assert_eq!(
                        board.cells[x][y].neighbor_mines,
                        board.count_mine_neighbors(x as u16, y as u16)
                    );
                }
            }
        }
    }

    #[test]
    fn is_revealed_and_is_flagged_reflect_cell_state() {
        let mut board = MinesweeperBoard::new(2, 2, 0);
        board.cells[1][0].revealed = true;
        board.cells[0][1].flagged = true;

        assert!(board.is_revealed(1, 0));
        assert!(!board.is_revealed(0, 0));
        assert!(board.is_flagged(0, 1));
        assert!(!board.is_flagged(1, 1));
    }

    #[test]
    fn cell_snapshot_maps_internal_state_to_public_state() {
        let mut board = MinesweeperBoard::new(3, 2, 0);
        board.cells[0][0].flagged = true;
        board.cells[1][0].revealed = true;
        board.cells[1][0].neighbor_mines = 2;
        board.cells[2][0].revealed = true;
        board.cells[2][0].mine = true;
        board.cells[0][1].revealed = true;
        board.cells[1][1].questioned = true;

        assert!(matches!(
            board.cell_snapshot(0, 0).state,
            CellState::Flagged
        ));
        assert!(matches!(
            board.cell_snapshot(1, 0).state,
            CellState::RevealedNumber(2)
        ));
        assert!(matches!(
            board.cell_snapshot(2, 0).state,
            CellState::RevealedMine
        ));
        assert!(matches!(
            board.cell_snapshot(0, 1).state,
            CellState::RevealedEmpty
        ));
        assert!(matches!(
            board.cell_snapshot(1, 1).state,
            CellState::Questioned
        ));
    }

    #[test]
    fn reveal_at_ignores_flagged_and_revealed_cells() {
        let mut board = MinesweeperBoard::new(2, 2, 0);
        board.first_click = false;
        board.cells[0][0].flagged = true;
        board.cells[1][1].revealed = true;

        assert!(matches!(board.reveal_at(0, 0), RevealOutcome::Ignored));
        assert!(matches!(board.reveal_at(1, 1), RevealOutcome::Ignored));
    }

    #[test]
    fn reveal_at_places_mines_on_first_click_and_reveals_safe_cell() {
        let mut board = MinesweeperBoard::new(6, 6, 5);

        let outcome = board.reveal_at(2, 2);

        assert!(matches!(outcome, RevealOutcome::Safe));
        assert!(!board.first_click);
        assert!(board.is_revealed(2, 2));

        let total_mines = board
            .cells
            .iter()
            .flatten()
            .filter(|cell| cell.mine)
            .count();
        assert_eq!(total_mines, 5);
    }

    #[test]
    fn reveal_returns_true_and_reveals_all_mines_when_hitting_a_mine() {
        let mut board = board_with_mines(3, 3, &[(0, 0), (2, 2)]);

        assert!(board.reveal(0, 0));
        assert!(board.cells[0][0].revealed);
        assert!(board.cells[2][2].revealed);
    }

    #[test]
    fn reveal_flood_fills_empty_region_but_leaves_flagged_cells_hidden() {
        let mut board = board_with_mines(4, 4, &[(3, 3)]);
        board.cells[1][1].flagged = true;

        assert!(!board.reveal(0, 0));
        assert!(board.cells[0][0].revealed);
        assert!(board.cells[2][2].revealed);
        assert!(!board.cells[1][1].revealed);
    }

    #[test]
    fn flood_fill_reveals_border_numbers_around_empty_area() {
        let mut board = board_with_mines(4, 4, &[(3, 3)]);

        board.flood_fill(0, 0);

        assert!(board.cells[2][2].revealed);
        assert_eq!(board.cells[2][2].neighbor_mines, 1);
        assert!(!board.cells[3][3].revealed);
    }

    #[test]
    fn toggle_flag_cycles_flag_question_hidden_and_ignores_revealed_cells() {
        let mut board = MinesweeperBoard::new(2, 2, 0);
        board.cells[1][1].revealed = true;

        board.toggle_flag(0, 0);
        assert!(board.is_flagged(0, 0));
        assert_eq!(board.flags_placed(), 1);

        board.toggle_flag(0, 0);
        assert!(matches!(
            board.cell_snapshot(0, 0).state,
            CellState::Questioned
        ));
        assert_eq!(board.flags_placed(), 0);

        board.toggle_flag(0, 0);
        assert!(!board.is_flagged(0, 0));
        assert_eq!(board.flags_placed(), 0);

        board.toggle_flag(1, 1);
        assert_eq!(board.flags_placed(), 0);
    }

    #[test]
    fn check_win_only_succeeds_when_all_safe_cells_are_revealed() {
        let mut board = board_with_mines(2, 2, &[(1, 1)]);
        assert!(!board.check_win());

        board.cells[0][0].revealed = true;
        board.cells[0][1].revealed = true;
        assert!(!board.check_win());

        board.cells[1][0].revealed = true;
        assert!(board.check_win());
    }

    #[test]
    fn finalize_win_flags_all_mines_and_clears_question_marks() {
        let mut board = board_with_mines(2, 2, &[(1, 1)]);
        board.cells[1][1].questioned = true;
        board.flags_placed = 0;

        board.finalize_win();

        assert!(board.cells[1][1].flagged);
        assert!(!board.cells[1][1].questioned);
        assert_eq!(board.flags_placed(), 1);
    }

    #[test]
    fn clicked_mine_is_marked_as_exploded() {
        let mut board = board_with_mines(2, 2, &[(1, 1)]);

        assert!(board.reveal(1, 1));
        assert!(matches!(
            board.cell_snapshot(1, 1).state,
            CellState::ExplodedMine
        ));
        assert!(matches!(
            board.cell_snapshot(0, 0).state,
            CellState::Hidden
        ));
    }

    #[test]
    fn adjacent_flag_count_counts_only_neighboring_flags() {
        let mut board = MinesweeperBoard::new(3, 3, 0);
        board.cells[0][0].flagged = true;
        board.cells[2][1].flagged = true;
        board.cells[2][2].flagged = true;

        assert_eq!(board.adjacent_flag_count(1, 1), 3);
        assert_eq!(board.adjacent_flag_count(0, 2), 0);
    }

    #[test]
    fn hidden_unflagged_neighbors_excludes_revealed_and_flagged_cells() {
        let mut board = MinesweeperBoard::new(3, 3, 0);
        board.cells[0][0].revealed = true;
        board.cells[1][0].flagged = true;

        let neighbors = sorted(board.hidden_unflagged_neighbors(1, 1));

        assert_eq!(
            neighbors,
            vec![(0, 1), (0, 2), (1, 2), (2, 0), (2, 1), (2, 2)]
        );
    }

    #[test]
    fn chord_hint_cells_returns_hidden_neighbors_for_revealed_number() {
        let mut board = board_with_mines(3, 3, &[(0, 0)]);
        board.cells[1][1].revealed = true;
        board.cells[0][1].flagged = true;

        let hinted = sorted(board.chord_hint_cells(1, 1));

        assert_eq!(
            hinted,
            vec![(0, 0), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1), (2, 2)]
        );
    }

    #[test]
    fn chord_hint_cells_returns_empty_for_non_revealed_or_zero_cells() {
        let mut board = MinesweeperBoard::new(3, 3, 0);
        board.cells[1][1].revealed = true;

        assert!(board.chord_hint_cells(0, 0).is_empty());
        assert!(board.chord_hint_cells(1, 1).is_empty());
    }

    #[test]
    fn chord_reveal_candidates_require_matching_flag_count() {
        let mut board = board_with_mines(3, 3, &[(0, 0), (2, 2)]);
        board.cells[1][1].revealed = true;
        board.cells[0][0].flagged = true;

        assert!(board.chord_reveal_candidates(1, 1).is_empty());

        board.cells[2][2].flagged = true;
        let candidates = sorted(board.chord_reveal_candidates(1, 1));
        assert_eq!(
            candidates,
            vec![(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)]
        );
    }

    #[test]
    fn chord_reveals_candidates_and_reports_mine_hit() {
        let mut board = board_with_mines(3, 3, &[(0, 0)]);
        board.cells[1][1].revealed = true;
        board.cells[0][0].flagged = true;

        assert!(!board.chord(1, 1));
        assert!(board.cells[2][2].revealed);

        board.cells[0][0].flagged = false;
        board.cells[2][2].revealed = false;

        assert!(board.reveal(1, 1) == false);
        board.cells[1][1].revealed = true;
        board.cells[0][0].flagged = true;
    }

    #[test]
    fn chord_hits_mine_when_flags_match_but_are_wrong() {
        let mut board = board_with_mines(3, 3, &[(0, 0)]);
        board.cells[1][1].revealed = true;
        board.cells[0][1].flagged = true;

        assert!(board.chord(1, 1));
        assert!(board.cells[0][0].revealed);
    }

    #[test]
    fn reveal_all_mines_reveals_every_mine_only() {
        let mut board = board_with_mines(3, 3, &[(0, 0), (2, 2)]);

        board.reveal_all_mines();

        assert!(board.cells[0][0].revealed);
        assert!(board.cells[2][2].revealed);
        assert!(!board.cells[1][1].revealed);
    }

    #[test]
    fn neighbor_positions_stays_in_bounds_for_corner_and_center() {
        let board = MinesweeperBoard::new(4, 4, 0);

        assert_eq!(sorted(board.neighbor_positions(0, 0)), vec![(0, 1), (1, 0), (1, 1)]);
        assert_eq!(board.neighbor_positions(1, 1).len(), 8);
    }

    #[test]
    fn count_mine_neighbors_counts_adjacent_mines() {
        let board = board_with_mines(3, 3, &[(0, 0), (0, 1), (2, 2)]);

        assert_eq!(board.count_mine_neighbors(1, 1), 3);
        assert_eq!(board.count_mine_neighbors(2, 1), 1);
    }
}
