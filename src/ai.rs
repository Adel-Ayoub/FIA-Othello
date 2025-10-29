use crate::board::{Board, Cell, Player};
use crate::common::CellList;
use crate::game::Move;
use crate::referee::Referee;

pub fn calculate_best_move(board: Board, valid_moves: CellList, player: Player) -> Move {
    let mut max = -1
        * negamax(
            get_board_after_move(&board, player, valid_moves.list[0]),
            player,
            8,
            false,
            None,
            None,
        );

    let mut max_index = 0;

    for i in 1..valid_moves.count {
        let val = -1
            * negamax(
                get_board_after_move(&board, player, valid_moves.list[i]),
                player.opponent(),
                4,
                false,
                None,
                None,
            );

        if val > max {
            max = val;
            max_index = i
        }
    }

    return valid_moves.list[max_index];
}

pub fn negamax(
    board: Board,
    player: Player,
    depth: u32,
    flag: bool,
    alpha: Option<i32>,
    beta: Option<i32>,
) -> i32 {
    if depth == 0 {
        return calculate_heuristic(board, player) as i32;
    }
    let mut referee = Referee::default();
    let mut valid_moves = CellList::default();
    let mut al = alpha;

    referee.find_all_valid_moves(&board, player, &mut valid_moves);

    let opp = player.opponent();
    if valid_moves.count == 0 {
        if flag {
            return calculate_heuristic(board, player) as i32;
        } else {
            return negamax(board, opp, depth - 1, true, al, beta);
        }
    }
    let mut max = None;

    for i in 0..valid_moves.count {
        let val = -1
            * negamax(
                get_board_after_move(&board, player, valid_moves.list[i]),
                opp,
                depth - 1,
                false,
                match beta {
                    Some(b) => Some(-b),
                    None => None,
                },
                match al {
                    Some(a) => Some(-a),
                    None => None,
                },
            );

        if max.is_none() || val > max.unwrap() {
            max = Some(val);
        }

        if al.is_none() || val > al.unwrap() {
            al = Some(val)
        }

        if al.is_some() && beta.is_some() && al >= beta {
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

    return max.unwrap() as i32;
}

pub fn calculate_heuristic(board: Board, player: Player) -> u32 {
    // The heuristic for now is the number of the bot's  pieces
    calculate_num_pieces(board, player)
}

pub fn calculate_num_pieces(board: Board, player: Player) -> u32 {
    let mut sum = 0;
    for i in 0..Board::SIZE {
        for j in 0..Board::SIZE {
            match board.grid[i][j] {
                Cell::Taken(p) if p == player => sum = sum + 1,
                _ => {}
            }
        }
    }
    sum
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
