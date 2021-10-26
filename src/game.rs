use log::*;
use rand::{thread_rng, Rng};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum CellState {
    Uncovered,
    Covered,
    Flagged,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct GameCell {
    pub state: CellState,
    pub mine: bool,
}

impl Default for GameCell {
    fn default() -> Self {
        Self {
            state: CellState::Covered,
            mine: false,
        }
    }
}

impl GameCell {
    pub fn new() -> Self {
        Self {
            state: CellState::Covered,
            mine: thread_rng().gen_bool(1.0 / 4.0),
        }
    }
}

impl std::fmt::Display for GameCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            match self.state {
                CellState::Covered => "C",
                CellState::Flagged => "F",
                CellState::Uncovered => "U",
            },
            if self.mine { "X" } else { "O" }
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum GameState {
    Won,
    Continue,
    Lost,
}

#[derive(Debug)]
pub struct Game {
    h: u8,
    w: u8,
    cells: Vec<GameCell>,
    state: GameState,
}

impl Game {
    pub fn new(w: u8, h: u8) -> Self {
        Self {
            h,
            w,
            cells: (0..h * w).into_iter().map(|_| GameCell::new()).collect(),
            state: GameState::Continue,
        }
    }

    pub fn height(&self) -> u8 {
        self.h
    }

    pub fn width(&self) -> u8 {
        self.w
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn cell_state(&self, x: u8, y: u8) -> Option<CellState> {
        self.cell(x, y).map(|cell| cell.state)
    }

    pub fn has_mine(&self, x: u8, y: u8) -> Option<bool> {
        self.cell(x, y).map(|cell| cell.mine)
    }

    pub fn adjacent_mines(&self, x: u8, y: u8) -> Option<usize> {
        self.cell(x, y).and_then(|_| {
            let mines = self
                .adj(x, y)
                .into_iter()
                .filter(|(x, y)| self.cells[(y * self.w + x) as usize].mine)
                .count();

            Some(mines)
        })
    }

    pub fn mines(&self) -> usize {
        self.cells.iter().filter(|c| c.mine).count()
    }

    pub fn flagged(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| c.state == CellState::Flagged)
            .count()
    }

    #[allow(dead_code)]
    pub fn dump(&self, x: u8, y: u8) -> Option<String> {
        self.cell(x, y).map(|cell| {
            format!(
                "{:?}, adjacent mines: {}",
                cell,
                self.adj(x, y)
                    .into_iter()
                    .filter(|(x, y)| self.cell(*x, *y).unwrap().mine)
                    .count()
            )
        })
    }

    pub fn open(&mut self, x: u8, y: u8) {
        let cell = if let Some(cell) = self.cell(x, y) {
            cell
        } else {
            return;
        };

        trace!("User clicked on {:#?}", cell);
        if cell.mine {
            self.state = GameState::Lost;
            return;
        }

        let mut visited = vec![false; self.cells.len()];
        self.visit(&mut visited, x, y);

        if self
            .cells
            .iter()
            .find(|c| c.state == CellState::Covered && !c.mine)
            .is_none()
        {
            self.state = GameState::Won;
        }
    }

    pub fn flag(&mut self, x: u8, y: u8) -> Option<bool> {
        let cell = if let Some(cell) = self.cell_mut(x, y) {
            cell
        } else {
            return None;
        };

        match cell.state {
            CellState::Covered => {
                cell.state = CellState::Flagged;
                Some(true)
            }
            CellState::Flagged => {
                cell.state = CellState::Covered;
                Some(false)
            }
            CellState::Uncovered => None,
        }
    }

    fn cell(&self, x: u8, y: u8) -> Option<&GameCell> {
        if !(x >= self.w || y >= self.h) {
            self.cells.get((y * self.w + x) as usize)
        } else {
            None
        }
    }

    fn cell_mut(&mut self, x: u8, y: u8) -> Option<&mut GameCell> {
        if !(x >= self.w || y >= self.h) {
            self.cells.get_mut((y * self.w + x) as usize)
        } else {
            None
        }
    }

    fn adj(&self, x: u8, y: u8) -> Vec<(u8, u8)> {
        let mut adjacent = vec![];

        if let Some(y) = y.checked_sub(1) {
            if let Some(x) = x.checked_sub(1) {
                adjacent.push((x, y));
            }

            adjacent.push((x, y));

            if x + 1 < self.w {
                adjacent.push((x + 1, y));
            }
        }

        if let Some(x) = x.checked_sub(1) {
            adjacent.push((x, y));
        }

        if x + 1 < self.w {
            adjacent.push((x + 1, y));
        }

        if y + 1 < self.h {
            let y = y + 1;

            if let Some(x) = x.checked_sub(1) {
                adjacent.push((x, y));
            }

            adjacent.push((x, y));

            if x + 1 < self.w {
                adjacent.push((x + 1, y));
            }
        }

        adjacent
    }

    fn visit(&mut self, visited: &mut [bool], x: u8, y: u8) {
        let cell_idx = (y * self.w + x) as usize;

        if visited[cell_idx] {
            return;
        }

        let adj = self.adj(x, y);
        let mines = adj
            .iter()
            .filter(|(x, y)| self.cells[(y * self.w + x) as usize].mine)
            .count();
        let cell = &mut self.cells[cell_idx];

        trace!("Visiting {:?}, adjacent mines: {}", cell, mines);

        cell.state = CellState::Uncovered;
        visited[cell_idx] = true;

        if adj
            .iter()
            .find(|(x, y)| self.cells[(y * self.w + x) as usize].mine)
            .is_none()
        {
            // Adjacent cells don't have mines. Keep opening...
            let to_visit: Vec<_> = adj
                .into_iter()
                .filter(|(x, y)| {
                    self.cells[(y * self.w + x) as usize].state != CellState::Uncovered
                })
                .collect();
            for (x, y) in to_visit {
                self.visit(visited, x, y);
            }
        }
    }
}

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n",
            (0..self.w)
                .into_iter()
                .fold(String::from("   "), |mut s, c| {
                    s += &format!("{}   ", c);
                    s
                })
        )?;
        for y in 0..self.h {
            write!(f, "{} ", y)?;
            for x in 0..self.w {
                write!(
                    f,
                    "{}{} ",
                    self.cell(x, y).unwrap(),
                    self.adjacent_mines(x, y).unwrap()
                )?;
            }
            write!(f, "\n")?;
        }
        write!(f, "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getters() {
        const H: u8 = 6;
        const W: u8 = 5;
        let mut game = Game::new(W, H);

        assert_eq!(game.height(), H);
        assert_eq!(game.width(), W);
        assert_eq!(game.state(), GameState::Continue);

        // Cell state
        // After initialization all cells should be covered
        for y in 0..H {
            for x in 0..W {
                assert_eq!(game.cell_state(x, y), Some(CellState::Covered));
            }
        }
        // Check out of bounds access
        assert_eq!(game.cell_state(W, H), None);
        assert_eq!(game.cell_state(H - 1, W - 1), None);

        // Flagged
        game.cells = vec![GameCell::default(); (H * W) as usize];
        game.cell_mut(0, 0).unwrap().state = CellState::Flagged;
        game.cell_mut(W - 1, H - 1).unwrap().state = CellState::Flagged;
        assert_eq!(game.flagged(), 2);

        // Mines
        // Place a mine into the first cell and check that its reflected correctly
        game.cell_mut(0, 0).unwrap().mine = true;
        assert_eq!(game.has_mine(0, 0), Some(true));
        // Check out of bounds access
        assert_eq!(game.has_mine(W, H), None);
        assert_eq!(game.has_mine(H - 1, W - 1), None);
        // Check adjacent mines
        game.cells = vec![GameCell::default(); (H * W) as usize];
        // No mines at all
        for y in 0..H {
            for x in 0..W {
                assert_eq!(game.adjacent_mines(x, y), Some(0));
            }
        }
        // 12321
        // 2xxx2
        // 3x8x3
        // 2xxx2
        // 12321
        // 00000
        game.cell_mut(1, 1).unwrap().mine = true;
        game.cell_mut(2, 1).unwrap().mine = true;
        game.cell_mut(3, 1).unwrap().mine = true;
        game.cell_mut(1, 2).unwrap().mine = true;
        game.cell_mut(3, 2).unwrap().mine = true;
        game.cell_mut(1, 3).unwrap().mine = true;
        game.cell_mut(2, 3).unwrap().mine = true;
        game.cell_mut(3, 3).unwrap().mine = true;

        assert_eq!(game.adjacent_mines(0, 0), Some(1));
        assert_eq!(game.adjacent_mines(1, 0), Some(2));
        assert_eq!(game.adjacent_mines(2, 0), Some(3));
        assert_eq!(game.adjacent_mines(3, 0), Some(2));
        assert_eq!(game.adjacent_mines(4, 0), Some(1));

        assert_eq!(game.adjacent_mines(0, 1), Some(2));
        assert_eq!(game.adjacent_mines(4, 1), Some(2));

        assert_eq!(game.adjacent_mines(0, 2), Some(3));
        assert_eq!(game.adjacent_mines(2, 2), Some(8));
        assert_eq!(game.adjacent_mines(4, 2), Some(3));

        assert_eq!(game.adjacent_mines(0, 3), Some(2));
        assert_eq!(game.adjacent_mines(4, 3), Some(2));

        assert_eq!(game.adjacent_mines(0, 4), Some(1));
        assert_eq!(game.adjacent_mines(1, 4), Some(2));
        assert_eq!(game.adjacent_mines(2, 4), Some(3));
        assert_eq!(game.adjacent_mines(3, 4), Some(2));
        assert_eq!(game.adjacent_mines(4, 4), Some(1));

        assert_eq!(game.adjacent_mines(0, 5), Some(0));
        assert_eq!(game.adjacent_mines(1, 5), Some(0));
        assert_eq!(game.adjacent_mines(2, 5), Some(0));
        assert_eq!(game.adjacent_mines(3, 5), Some(0));
        assert_eq!(game.adjacent_mines(4, 5), Some(0));

        // Check out of bounds access
        assert_eq!(game.adjacent_mines(W, H), None);
        assert_eq!(game.adjacent_mines(H - 1, W - 1), None);
    }

    #[test]
    fn open() {
        const N: u8 = 4;
        let mut game = Game::new(N, N);

        // 0000
        // 0000
        // 0000
        // 0000
        game.cells = vec![GameCell::default(); (N * N) as usize];
        game.open(0, 0);
        // Because there're no mines, opening any cell will result in
        // uncovering the whole board
        for y in 0..N {
            for x in 0..N {
                assert_eq!(game.cell_state(x, y), Some(CellState::Uncovered));
            }
        }

        // 0000
        // 0x00
        // 0000
        // 0000
        game.cells = vec![GameCell::default(); (N * N) as usize];
        game.cell_mut(1, 1).unwrap().mine = true;

        // All adjacent cells have at least one adjacent mine - should remain covered
        game.open(0, 0);
        assert_eq!(game.cell_state(0, 0), Some(CellState::Uncovered));
        assert_eq!(
            0,
            game.adj(0, 0)
                .into_iter()
                .filter(|(x, y)| game.cell_state(*x, *y) == Some(CellState::Uncovered))
                .count()
        );

        // No adjacent cells have mines - can uncover them
        game.open(3, 3);
        assert_eq!(game.cell_state(3, 2), Some(CellState::Uncovered));
        assert_eq!(game.cell_state(3, 3), Some(CellState::Uncovered));
        assert_eq!(game.cell_state(2, 2), Some(CellState::Uncovered));
        assert_eq!(game.cell_state(2, 3), Some(CellState::Uncovered));
    }

    #[test]
    fn flag() {
        const H: u8 = 6;
        const W: u8 = 5;
        let mut game = Game::new(W, H);

        assert_eq!(game.flag(0, 0), Some(true));
        assert_eq!(game.flagged(), 1);
        assert_eq!(game.flag(0, 0), Some(false));
    }
}
