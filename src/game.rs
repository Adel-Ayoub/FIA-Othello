use crate::ai::calculate_best_move;
use crate::board::Board;
use crate::board::Cell;
use crate::board::Player;
use crate::common::CellList;
use crate::referee::Outcome;
use crate::referee::Referee;
use crate::statistics::Statistics;
use eframe::egui;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub type Move = (usize, usize);

#[derive(Clone, Copy)]
enum Phase {
    Turn(Player),
    Win(Player),
    Tie,
}

pub struct GameOptions {
    show_effects_of_moves: bool,
    show_valid_moves: bool,
    auto_restart: bool,
    pause_at_win: bool,
    should_take_statistics: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            show_effects_of_moves: false,
            show_valid_moves: false,
            auto_restart: false,
            pause_at_win: true,
            should_take_statistics: true,
        }
    }
}

pub struct Game {
    board: Board,
    current_phase: Phase,
    options: GameOptions,
    referee: Referee,
    valid_moves: CellList,
    flip_cells: CellList,
    scheduled_restart: Instant,
    is_board_untouched: bool,
    can_take_statistics: bool,
    statistics: Statistics,
    bot: Player,
    pending_bot_move: Option<Receiver<Move>>,
}

impl Default for Game {
    fn default() -> Self {
        let mut game = Game {
            board: Board::default(),
            current_phase: Phase::Turn(Player::Black),
            options: GameOptions::default(),
            referee: Referee::default(),
            valid_moves: CellList::default(),
            flip_cells: CellList::default(),
            scheduled_restart: Instant::now(),
            is_board_untouched: false,
            can_take_statistics: false,
            statistics: Statistics::default(),
            bot: Player::White,
            pending_bot_move: None,
        };

        game.reset();

        game
    }
}

impl Game {
    // call this from the UI thread
    // Initialize the game
    fn reset(&mut self) {
        self.board = Board::default();
        self.current_phase = Phase::Turn(Player::Black);
        self.referee
            .find_all_valid_moves(&self.board, Player::Black, &mut self.valid_moves);
        self.is_board_untouched = true;
        self.can_take_statistics = true;
        self.set_current_player_turn(Player::Black);
    }

    fn set_current_player_turn(&mut self, player: Player) {
        // Assume the game has NOT ended.
        self.current_phase = Phase::Turn(player);

        if self.bot != player {
            return;
        }
        self.play_bot_turn(player);
    }

    fn play_bot_turn(&mut self, bot_player: Player) {
        self.referee
            .find_all_valid_moves(&self.board, bot_player, &mut self.valid_moves);
        // Pick one of valid moves
        let board = self.board.clone();
        let valid_moves = self.valid_moves.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let best_move = calculate_best_move(board, valid_moves, bot_player);
            tx.send(best_move).ok();
        });

        self.pending_bot_move = Some(rx);
    }

    fn is_currently_bot_turn(&mut self) -> bool {
        matches!(self.current_phase,Phase::Turn(p) if p == self.bot)
    }

    // call this from the UI thread
    // Make a move by a player
    fn make_move(&mut self, next_move: Move, player: Player) -> bool {
        // Validate and collect flip cells for ai move
        if self.referee.find_flip_cells_for_move(
            &self.board,
            player,
            next_move,
            &mut self.flip_cells,
        ) {
            Referee::apply_move(&mut self.board, player, next_move, &self.flip_cells);

            let opponent = player.opponent();

            if self
                .referee
                .find_all_valid_moves(&self.board, opponent, &mut self.valid_moves)
            {
                // switch players if the other player has valid moves
                self.set_current_player_turn(opponent);
            } else if !self
                .referee
                .find_all_valid_moves(&self.board, player, &mut self.valid_moves)
            {
                // no player has any valid moves, game ends
                let outcome = Referee::check_outcome(&self.board);
                self.current_phase = match outcome {
                    Outcome::Won(player) => Phase::Win(player),
                    Outcome::Tie => Phase::Tie,
                };

                self.take_statistics(outcome);

                // only used if auto_restart is enabled
                self.scheduled_restart = Instant::now();
                if self.options.pause_at_win {
                    self.scheduled_restart += Duration::from_secs(1);
                }
            }

            if self.is_board_untouched {
                // you can mess with the settings before the first move and still take statistics
                self.can_take_statistics = true;
                self.is_board_untouched = false;
            }

            true
        } else {
            false
        }
    }

    fn take_statistics(&mut self, outcome: Outcome) {
        if self.can_take_statistics {
            // sort so that another player color doesn't render another entry
            let first_player = Player::Black;

            self.statistics
                .add_datum("Human vs Human".to_string(), first_player, &outcome);

            self.can_take_statistics = false;
        }
    }
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.current_phase {
            Phase::Turn(player) => {
                if let Some(rx) = &self.pending_bot_move {
                    if let Ok(bot_move) = rx.try_recv() {
                        self.make_move(bot_move, player);
                        self.pending_bot_move = None; // Clear after receiving
                    }
                }
            }
            _ => {}
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // UI drawing
            let rect = ui.available_rect_before_wrap();
            let square_size = rect.width().min(rect.height()) / Board::SIZE as f32;
            let line_width = square_size * 0.01;

            let to_color = |player| match player {
                Player::Black => egui::Color32::BLACK,
                Player::White => egui::Color32::WHITE,
            };

            let get_square_rect = |row, col| {
                let square_pos = egui::Pos2 {
                    x: rect.left() + col as f32 * square_size,
                    y: rect.top() + row as f32 * square_size,
                };
                egui::Rect::from_min_size(square_pos, egui::Vec2::splat(square_size))
            };

            // draw the current board state
            for row in 0..Board::SIZE {
                for col in 0..Board::SIZE {
                    let square_rect = get_square_rect(row, col);

                    ui.painter()
                        .rect_filled(square_rect, 0.0, egui::Color32::DARK_GREEN);

                    let stroke = egui::Stroke {
                        width: line_width,
                        color: egui::Color32::BLACK,
                    };
                    ui.painter()
                        .rect_stroke(square_rect, 0.0, stroke, egui::StrokeKind::Inside);

                    if let Cell::Taken(cell_state) = self.board.grid[row][col] {
                        ui.painter().circle_filled(
                            square_rect.center(),
                            square_size / 2.0 * 0.93,
                            to_color(cell_state),
                        );
                    }
                }
            }

            match self.current_phase {
                Phase::Turn(player) => {
                    //player

                    // Awaiting next move
                    if self.options.show_valid_moves {
                        for (valid_row, valid_col) in self.valid_moves.iter() {
                            let square_rect = get_square_rect(valid_row, valid_col);
                            let highlight_color = match player {
                                Player::Black => {
                                    egui::Color32::from_rgba_premultiplied(0, 100, 0, 40)
                                }
                                Player::White => {
                                    egui::Color32::from_rgba_premultiplied(100, 100, 100, 30)
                                }
                            };
                            ui.painter().rect_filled(square_rect, 0.0, highlight_color);
                        }
                    }

                    // Mouse handling
                    let mut row = Board::SIZE;
                    let mut col = Board::SIZE;

                    let mut is_valid_move = false;

                    // check mouse hovering
                    if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos()) {
                        row = ((mouse_pos.y - rect.top()) / square_size) as usize;
                        col = ((mouse_pos.x - rect.left()) / square_size) as usize;

                        if row < Board::SIZE && col < Board::SIZE {
                            // this could be optimized by only doing it when the mouse changes cells
                            is_valid_move = self.referee.find_flip_cells_for_move(
                                &self.board,
                                player,
                                (row, col),
                                &mut self.flip_cells,
                            );

                            if is_valid_move {
                                // show move effects with connecting lines
                                if self.options.show_effects_of_moves {
                                    let hovered_rect = get_square_rect(row, col);
                                    let hovered_center = hovered_rect.center();

                                    // Highlight the hovered square with a bright border
                                    ui.painter().rect_stroke(
                                        hovered_rect,
                                        0.0,
                                        egui::Stroke::new(
                                            3.0,
                                            egui::Color32::from_rgb(255, 255, 0),
                                        ),
                                        egui::StrokeKind::Inside,
                                    );

                                    for (flip_row, flip_col) in self.flip_cells.iter() {
                                        let flip_rect = get_square_rect(flip_row, flip_col);
                                        let flip_center = flip_rect.center();

                                        // Draw line from hovered center to flip center
                                        ui.painter().line_segment(
                                            [hovered_center, flip_center],
                                            egui::Stroke::new(2.0, to_color(player)),
                                        );

                                        // Simple dot at flip cell instead of arrowhead
                                        ui.painter().circle_filled(
                                            flip_center,
                                            square_size * 0.08,
                                            to_color(player),
                                        );

                                        // Highlight the flip cell with a border
                                        ui.painter().rect_stroke(
                                            flip_rect,
                                            0.0,
                                            egui::Stroke::new(2.0, to_color(player)),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    // handle mouse clicks to make moves
                    if ui.input(|i| i.pointer.any_down())
                        && !self.is_currently_bot_turn()
                        && row < Board::SIZE
                        && col < Board::SIZE
                        && is_valid_move
                    {
                        assert!(self.make_move((row, col), player));
                    }
                }
                Phase::Win(_) | Phase::Tie => {
                    if self.options.auto_restart && Instant::now() >= self.scheduled_restart {
                        self.reset();
                    }
                }
            }

            ctx.request_repaint();
        });

        egui::SidePanel::right("right_panel").show(ctx, move |ui| {
            ui.separator();

            // Current-status message
            let message = match self.current_phase {
                Phase::Turn(player) => {
                    format!("{:?}'s turn", player)
                }
                Phase::Win(player) => {
                    format!("{:?} won", player)
                }
                Phase::Tie => "Tie".to_string(),
            };

            ui.label(message);

            ui.separator();

            ui.label("Control");
            if ui.button("Restart Game").clicked() {
                self.reset();
            }
            ui.checkbox(&mut self.options.auto_restart, "Auto Restart");

            ui.separator();

            ui.label("Flow");
            ui.checkbox(&mut self.options.pause_at_win, "Pause at Win");

            ui.separator();

            ui.label("Help");
            ui.checkbox(&mut self.options.show_valid_moves, "Show Valid Moves");
            ui.checkbox(
                &mut self.options.show_effects_of_moves,
                "Show Effects of Moves",
            );

            ui.separator();

            ui.label("Statistics");
            ui.checkbox(&mut self.options.should_take_statistics, "Take Statistics");
            let modus = match (
                self.can_take_statistics,
                self.options.should_take_statistics,
            ) {
                (true, true) => "will",
                (false, true) => "cannot",
                (_, false) => "will not",
            };
            ui.label(format!("Statistics {modus} be taken"));

            ui.separator();

            ui.label("Won%, Tied%, Lost%, (Total):");
            for (name, statistic) in self.statistics.data.iter() {
                ui.label(format!("{name}:\n{statistic}"));
            }
        });
    }
}
