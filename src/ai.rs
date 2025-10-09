use std::thread;
use std::time::Duration;

use crate::board::{Board, Player};
use crate::common::CellList;
use crate::game::Move;

pub fn calculate_best_move(board: Board, valid_moves: CellList, player: Player) -> Move {
    thread::sleep(Duration::from_secs(1));
    return valid_moves.list[0];
}
