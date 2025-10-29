use crate::board::{Board, Cell, Player};
use crate::common::CellList;
use crate::game::Move;
use crate::referee::Referee;

const OTHELLO_WEIGHTS: [[i32; 8]; 8] = [
    [7, 2, 5, 4, 4, 5, 2, 7],
    [2, 1, 3, 3, 3, 3, 1, 2],
    [5, 3, 5, 5, 5, 5, 3, 5],
    [4, 3, 5, 6, 6, 5, 3, 4],
    [4, 3, 5, 6, 6, 5, 3, 4],
    [5, 3, 5, 5, 5, 5, 3, 5],
    [2, 1, 3, 3, 3, 3, 1, 2],
    [7, 2, 5, 4, 4, 5, 2, 7],
];

pub fn calculate_best_move(board: Board, valid_moves: CellList, player: Player) -> Move {
    let mut max: Option<f32> = None;
    let mut max_index: Option<usize> = None;

    for i in 0..valid_moves.count {
        let val = (-1 as f32)
            * negamax(
                get_board_after_move(&board, player, valid_moves.list[i]),
                player.opponent(),
                8,
                false,
                None,
                None,
            );

        if max_index.is_none() || val > max.unwrap() {
            max = Some(val);
            max_index = Some(i);
        }
    }

    return valid_moves.list[max_index.unwrap()];
}

pub fn negamax(
    board: Board,
    player: Player,
    depth: u32,
    previous_player_has_played: bool, // Used to detect end of game, if the past player had no moves, and the current
    // player also doesnt have moves, then we quit (terminal node)
    alpha: Option<f32>,
    beta: Option<f32>,
) -> f32 {
    if depth == 0 {
        return calculate_heuristic(board, player);
    }
    let mut referee = Referee::default();
    let mut valid_moves = CellList::default();
    let mut current_alpha = alpha;

    referee.find_all_valid_moves(&board, player, &mut valid_moves);

    let opp = player.opponent();
    if valid_moves.count == 0 {
        if previous_player_has_played {
            return calculate_heuristic(board, player);
        } else {
            return negamax(board, opp, depth - 1, true, current_alpha, beta);
        }
    }
    let mut max = None;

    for i in 0..valid_moves.count {
        let val = (-1 as f32)
            * negamax(
                get_board_after_move(&board, player, valid_moves.list[i]),
                opp,
                depth - 1,
                false,
                negate(beta),
                negate(current_alpha),
            );

        if max.is_none() || val > max.unwrap() {
            max = Some(val);
        }

        if current_alpha.is_none() || val > current_alpha.unwrap() {
            current_alpha = Some(val)
        }

        if current_alpha.is_some() && beta.is_some() && current_alpha >= beta {
            if depth > 3 {
                println!(
                    "Prunned at child {} out of {} and depth {}",
                    i + 1,
                    valid_moves.count,
                    depth
                );
            }
            break;
        }
    }

    return max.unwrap();
}

pub fn calculate_heuristic(board: Board, player: Player) -> f32 {
    calculate_weighted_piece_positions(board, player)
}

pub fn calculate_weighted_piece_positions(board: Board, player: Player) -> f32 {
    let mut sum = 0;
    for i in 0..Board::SIZE {
        for j in 0..Board::SIZE {
            match board.grid[i][j] {
                Cell::Taken(p) if p == player => sum = sum + OTHELLO_WEIGHTS[i][j],
                _ => {}
            }
        }
    }
    sum as f32
}

pub fn get_board_after_move(board: &Board, player: Player, (row, col): Move) -> Board {
    let mut referee = Referee::default();
    let mut new_board = board.clone();
    let mut flip_cells = CellList::default();

    if referee.find_flip_cells_for_move(&board, player, (row, col), &mut flip_cells) {
        Referee::apply_move(&mut new_board, player, (row, col), &flip_cells);
    }

    new_board.grid[row][col] = Cell::Taken(player);

    // flip cells
    for (flip_row, flip_col) in flip_cells.iter() {
        new_board.grid[flip_row][flip_col] = Cell::Taken(player);
    }

    new_board
}

fn negate(value: Option<f32>) -> Option<f32> {
    match value {
        Some(v) => Some(-v),
        None => None,
    }
}
