#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Player {
    Black,
    White,
}

impl Player {
    pub fn opponent(&self) -> Self {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Empty,
    Taken(Player),
}

#[derive(Clone, Debug)]
pub struct Board {
    pub grid: [[Cell; Board::SIZE]; Board::SIZE],
}

type Move = (usize, usize);

impl Board {
    pub const SIZE: usize = 8;

    pub fn cell(&self, (row, col): Move) -> Cell {
        self.grid[row][col]
    }
}

impl Default for Board {
    fn default() -> Self {
        let mut grid = [[Cell::Empty; Board::SIZE]; Board::SIZE];
        grid[3][3] = Cell::Taken(Player::White);
        grid[4][4] = Cell::Taken(Player::White);
        grid[3][4] = Cell::Taken(Player::Black);
        grid[4][3] = Cell::Taken(Player::Black);

        Board { grid }
    }
}
