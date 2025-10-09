use crate::board::Board;

type Move = (usize, usize);

// Common utility types
// a cache for re-use to avoid unnecesary memory allocations
#[derive(Clone)]
pub struct CellList {
    pub list: [Move; 64],
    pub count: usize,
}

impl Default for CellList {
    fn default() -> Self {
        CellList {
            list: [(Board::SIZE, Board::SIZE); 64],
            count: 0,
        }
    }
}

impl CellList {
    pub fn push_back(&mut self, cell: Move) {
        self.list[self.count] = cell;
        self.count += 1;
    }

    pub fn iter(&self) -> impl Iterator<Item = Move> {
        self.list[..self.count].iter().copied()
    }
}
