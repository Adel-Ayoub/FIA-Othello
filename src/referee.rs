use crate::board::Board;
use crate::board::Cell;
use crate::board::Player;
use crate::common::CellList;

pub enum Outcome {
    Won(Player),
    Tie,
}

type Move = (usize, usize);

// not thread-safe, every thread needs its own Referee
#[derive(Default)]
pub struct Referee {
    // a cache for the result of find_adjacent_opposites
    adjacent_opposites: CellList,

    // a cache for the result of find_flip_cells
    flip_cells: CellList,
}

impl Referee {
    pub fn validate_move(&mut self, board: &Board, player: Player, maybe_move: Move) -> bool {
        Self::find_flip_cells_for_move_internal(
            board,
            player,
            maybe_move,
            &mut self.adjacent_opposites,
            &mut self.flip_cells,
        )
    }

    pub fn find_flip_cells_for_move(
        &mut self,
        board: &Board,
        player: Player,
        maybe_move: Move,
        result: &mut CellList,
    ) -> bool {
        Self::find_flip_cells_for_move_internal(
            board,
            player,
            maybe_move,
            &mut self.adjacent_opposites,
            result,
        )
    }

    pub fn find_all_valid_moves(
        &mut self,
        board: &Board,
        player: Player,
        result: &mut CellList,
    ) -> bool {
        result.count = 0;

        for row in 0..Board::SIZE {
            for col in 0..Board::SIZE {
                if self.validate_move(board, player, (row, col)) {
                    result.push_back((row, col));
                }
            }
        }

        result.count != 0
    }

    // returns (player disk count, opponent disk count)
    pub fn count_disks(board: &Board, player: Player) -> (i32, i32) {
        let mut black_count = 0;
        let mut white_count = 0;

        for row in 0..Board::SIZE {
            for col in 0..Board::SIZE {
                match board.grid[row][col] {
                    Cell::Empty => {}
                    Cell::Taken(Player::Black) => black_count += 1,
                    Cell::Taken(Player::White) => white_count += 1,
                }
            }
        }

        match player {
            Player::Black => (black_count, white_count),
            Player::White => (white_count, black_count),
        }
    }

    pub fn check_outcome(board: &Board) -> Outcome {
        let (black_count, white_count) = Self::count_disks(board, Player::Black);

        if black_count > white_count {
            Outcome::Won(Player::Black)
        } else if white_count > black_count {
            Outcome::Won(Player::White)
        } else {
            Outcome::Tie
        }
    }

    pub fn apply_move(board: &mut Board, player: Player, (row, col): Move, flip_cells: &CellList) {
        // Place the current player's piece
        board.grid[row][col] = Cell::Taken(player);

        // flip cells
        for (flip_row, flip_col) in flip_cells.iter() {
            board.grid[flip_row][flip_col] = Cell::Taken(player);
        }
    }

    pub fn get_board_after_move(
        board: &Board,
        player: Player,
        (row, col): Move,
        flip_cells: &CellList,
    ) -> Board {
        // Place the current player's piece
        let mut new_board = board.clone();
        new_board.grid[row][col] = Cell::Taken(player);

        // flip cells
        for (flip_row, flip_col) in flip_cells.iter() {
            new_board.grid[flip_row][flip_col] = Cell::Taken(player);
        }

        new_board
    }

    // internal

    fn find_flip_cells_for_move_internal(
        board: &Board,
        player: Player,
        maybe_move: Move,
        adjacent_opposites: &mut CellList,
        flip_cells: &mut CellList,
    ) -> bool {
        match board.cell(maybe_move) {
            Cell::Empty => {
                if Self::find_adjacent_opposites(board, player, maybe_move, adjacent_opposites) {
                    Self::find_flip_cells(board, player, maybe_move, adjacent_opposites, flip_cells)
                } else {
                    false
                }
            }
            Cell::Taken(_) => false,
        }
    }

    fn find_adjacent_opposites(
        board: &Board,
        player: Player,
        (row, col): Move,
        result: &mut CellList,
    ) -> bool {
        let start_row = match row {
            0 => 0,
            current_row => current_row - 1,
        };
        let end_row = match row + 1 {
            Board::SIZE => Board::SIZE,
            next_row => next_row + 1,
        };
        let start_col = match col {
            0 => 0,
            current_col => current_col - 1,
        };
        let end_col = match col + 1 {
            Board::SIZE => Board::SIZE,
            next_col => next_col + 1,
        };

        result.count = 0;

        for other_row in start_row..end_row {
            for other_col in start_col..end_col {
                if other_row != row || other_col != col {
                    if let Cell::Taken(other_disk) = board.grid[other_row][other_col] {
                        if other_disk != player {
                            result.push_back((other_row, other_col));
                        }
                    }
                }
            }
        }

        result.count != 0
    }

    // expects result to already be filled with adjacent opposites
    fn find_flip_cells(
        board: &Board,
        player: Player,
        (row, col): Move,
        adjacent_opposites: &CellList,
        result: &mut CellList,
    ) -> bool {
        result.count = 0;

        for (adjacent_row, adjacent_col) in adjacent_opposites.iter() {
            let direction = (
                adjacent_row as i32 - row as i32,
                adjacent_col as i32 - col as i32,
            );
            Self::cast_ray_recursive(
                board,
                player,
                (adjacent_row, adjacent_col),
                direction,
                result,
            );
        }

        result.count != 0
    }

    /* cast a ray in the checked direction
    the ray is successful if a cell belonging to the player is found
    then all disks in between should be flipped */

    /* this problem lends itself to a recursive approach */
    fn cast_ray_recursive(
        board: &Board,
        player: Player,
        (row, col): Move,
        (row_direction, col_direction): (i32, i32),
        result: &mut CellList,
    ) -> bool {
        match board.grid[row][col] {
            Cell::Empty => false,
            Cell::Taken(color) if color == player => {
                result.push_back((row, col));
                true
            }
            Cell::Taken(_) => {
                let new_row = row as i32 + row_direction;
                let new_col = col as i32 + col_direction;
                if new_row < 0
                    || new_row >= Board::SIZE as i32
                    || new_col < 0
                    || new_col >= Board::SIZE as i32
                {
                    false
                } else if Self::cast_ray_recursive(
                    board,
                    player,
                    (new_row as usize, new_col as usize),
                    (row_direction, col_direction),
                    result,
                ) {
                    result.push_back((row, col));
                    true
                } else {
                    false
                }
            }
        }
    }
}
