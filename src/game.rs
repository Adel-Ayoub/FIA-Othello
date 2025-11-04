use std::convert::TryFrom;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use eframe::egui;

use crate::ai::Agent;
use crate::ai::AiType;
use crate::ai::MoveRequest;
use crate::ai::MoveResult;
use crate::board::Board;
use crate::board::Cell;
use crate::board::Player;
use crate::common::CellList;
use crate::referee::Outcome;
use crate::referee::Referee;
use crate::statistics::Statistics;

type Move = (usize, usize);

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
    pace_ai: bool,
    pause_at_win: bool,
    should_take_statistics: bool,
}

impl Default for GameOptions {
    fn default() -> Self {
        GameOptions {
            show_effects_of_moves: false,
            show_valid_moves: false,
            auto_restart: false,
            pace_ai: true,
            pause_at_win: true,
            should_take_statistics: true,
        }
    }
}

#[derive(Clone, Copy)]
pub struct PlayerOptions {
    ai_enabled: bool,
    ai_type: AiType,
    ai_recursion_depth: usize,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        PlayerOptions {
            ai_enabled: false,
            ai_type: AiType::Random,
            ai_recursion_depth: 1,
        }
    }
}

pub struct Game {
    board: Board,
    current_phase: Phase,
    options: GameOptions,
    player_options: [PlayerOptions; 2],
    ai_thread: Option<thread::JoinHandle<()>>,
    awaiting_ai_move: bool,
    move_request_sender: Option<mpsc::Sender<MoveRequest>>,
    move_result_receiver: mpsc::Receiver<MoveResult>,
    referee: Referee,
    valid_moves: CellList,
    flip_cells: CellList,
    scheduled_restart: Instant,
    is_board_untouched: bool,
    can_take_statistics: bool,
    statistics: Statistics,
}

impl Default for Game {
    fn default() -> Self {
        let (move_request_sender, move_request_receiver) = mpsc::channel::<MoveRequest>();
        let (move_result_sender, move_result_receiver) = mpsc::channel::<MoveResult>();

        let ai_thread = thread::spawn(move || {
            let mut agent = Agent::new(move_request_receiver, move_result_sender);
            agent.run();
        });

        let mut game = Game {
            board: Board::default(),
            current_phase: Phase::Turn(Player::Black),
            options: GameOptions::default(),
            player_options: [PlayerOptions::default(); 2],
            ai_thread: Some(ai_thread),
            awaiting_ai_move: false,
            move_request_sender: Some(move_request_sender),
            move_result_receiver,
            referee: Referee::default(),
            valid_moves: CellList::default(),
            flip_cells: CellList::default(),
            scheduled_restart: Instant::now(),
            is_board_untouched: false,
            can_take_statistics: false,
            statistics: Statistics::default(),
        };

        game.reset();

        game
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        println!("Game is being dropped. Cleaning up AI thread...");

        // Drop the sender so AI thread exits
        self.move_request_sender = None;

        // Wait for AI thread to exit
        if let Some(ai_thread) = self.ai_thread.take() {
            let _ = ai_thread.join();
            println!("...and joined AI thread");
        }
    }
}

impl Game {
    // call this from the UI thread
    fn reset(&mut self) {
        self.board = Board::default();
        self.current_phase = Phase::Turn(Player::Black);
        self.referee
            .find_all_valid_moves(&self.board, Player::Black, &mut self.valid_moves);
        self.is_board_untouched = true;
        self.can_take_statistics = true;
    }

    fn ai_setting_changed(&mut self) {
        // statistics are deemed invalid if the ai settings are changed after the game has started
        if !self.is_board_untouched {
            self.can_take_statistics = false;
        }
    }

    // call this from the UI thread
    fn tick_ai(&mut self, player: Player) {
        // either poll for ai response, non-blocking
        if self.awaiting_ai_move {
            if let Ok(move_result) = self.move_result_receiver.try_recv() {
                let (row, col) = move_result.next_move;
                if row < Board::SIZE && col < Board::SIZE {
                    if move_result.board.grid == self.board.grid && move_result.player == player {
                        assert!(self.make_move(move_result.next_move, player));
                    }
                } else {
                    // unable to come up with a valid move, it seems
                    self.player_options[player as usize].ai_enabled = false;
                }
                self.awaiting_ai_move = false;
            }
        } else {
            // or ask ai to start thinking about the next move

            if let Some(tx) = &self.move_request_sender {
                self.awaiting_ai_move = true;
                let _ = tx.send(MoveRequest {
                    board: self.board.clone(),
                    player,
                    pace_response: self.options.pace_ai,
                    algorithm_choice: self.player_options[player as usize].ai_type,
                    recursion_depth: self.player_options[player as usize].ai_recursion_depth,
                });
            }
        }
    }

    // call this from the UI thread
    fn make_move(&mut self, next_move: Move, player: Player) -> bool {
        // Validate and collect flip cells for ai move
        if self.referee.find_flip_cells_for_move(
            &self.board,
            player,
            next_move,
            &mut self.flip_cells,
        ) {
            Referee::apply_move(&mut self.board, player, next_move, &self.flip_cells);

            let (black_count, white_count) = count_pieces(&self.board);
            println!(
                "Turn score - Black: {}, White: {}",
                black_count, white_count
            );

            let opponent = player.opponent();

            if self
                .referee
                .find_all_valid_moves(&self.board, opponent, &mut self.valid_moves)
            {
                // switch players if the other player has valid moves
                self.current_phase = Phase::Turn(opponent);
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
            let mut names: [String; 2] = [String::new(), String::new()];
            for (i, name) in names.iter_mut().enumerate() {
                *name = if self.player_options[i].ai_enabled {
                    match self.player_options[i].ai_type {
                        AiType::Random => "Random".to_string(),
                        AiType::Minimax => {
                            format!("Minimax lvl {}", self.player_options[i].ai_recursion_depth)
                        }
                    }
                } else {
                    "Human".to_string()
                };
            }

            // sort so that another player color doesn't render another entry
            let first_player = if names[0] < names[1] {
                Player::Black
            } else {
                Player::White
            };

            self.statistics.add_datum(
                format!(
                    "{} vs {}",
                    names[first_player as usize],
                    names[(first_player as usize + 1) % 2]
                ),
                first_player,
                &outcome,
            );

            self.can_take_statistics = false;
        }
    }

    fn update_player_options_controls(&mut self, ui: &mut egui::Ui, player: Player) {
        // Define the maximum depth for the minimax algorithm
        let max_depth = 10;

        ui.label(format!("{:?} Player Options", player));
        if ui
            .checkbox(
                &mut self.player_options[player as usize].ai_enabled,
                "Enable AI",
            )
            .changed()
        {
            self.ai_setting_changed();
        }
        ui.label("AI Type");
        self.player_options[player as usize].ai_type = self.update_ai_type_radio_buttons(
            ui,
            self.player_options[player as usize].ai_type,
            player,
        );
        // a slider for the minimax algorithm recursion depth
        ui.label("AI Recursion Depth");
        if ui
            .add(
                egui::Slider::new(
                    &mut self.player_options[player as usize].ai_recursion_depth,
                    1..=max_depth,
                )
                .text(""),
            )
            .changed()
            && self.player_options[player as usize].ai_enabled
            && self.player_options[player as usize].ai_type == AiType::Minimax
        {
            self.ai_setting_changed();
        }
    }

    // closure that handles the dynamic depth options
    fn update_ai_type_radio_buttons(
        &mut self,
        ui: &mut egui::Ui,
        ai_type: AiType,
        player: Player,
    ) -> AiType {
        let options = ["Random".to_string(), "Minimax".to_string()];

        let mut result = ai_type;

        // Display dynamic depth options in a loop
        for (i, option) in options.iter().enumerate() {
            if ui.radio(ai_type as usize == i, option).clicked() {
                result = match AiType::try_from(i) {
                    Ok(agent_type) => agent_type,
                    Err(e) => {
                        panic!("AI type conversion failed: {e}")
                    }
                };

                if self.player_options[player as usize].ai_enabled {
                    self.ai_setting_changed();
                }
            }
        }

        result
    }
}

pub fn count_pieces(board: &Board) -> (usize, usize) {
    let mut black_count = 0;
    let mut white_count = 0;

    for row in 0..8 {
        for col in 0..8 {
            match board.grid[row][col] {
                Cell::Taken(Player::Black) => black_count += 1,
                Cell::Taken(Player::White) => white_count += 1,
                Cell::Empty => {}
            }
        }
    }

    (black_count, white_count)
}

impl eframe::App for Game {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                Phase::Turn(player) if self.player_options[player as usize].ai_enabled => {
                    // AI moves
                    self.tick_ai(player);
                }
                Phase::Turn(player) => {
                    // ai is disabled

                    // Awaiting human move
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
                    if self.awaiting_ai_move && self.player_options[player as usize].ai_enabled {
                        format!("{:?} is thinking...", player)
                    } else {
                        format!("{:?}'s turn", player)
                    }
                }
                Phase::Win(player) => {
                    format!("{:?} won", player)
                }
                Phase::Tie => "Tie".to_string(),
            };

            ui.label(message);

            let (black_score, white_score) = count_pieces(&self.board);
            ui.label(format!("Black: {}", black_score));
            ui.label(format!("White: {}", white_score));

            ui.separator();

            self.update_player_options_controls(ui, Player::Black);

            ui.separator();

            self.update_player_options_controls(ui, Player::White);

            ui.separator();

            ui.label("Control");
            if ui.button("Restart Game").clicked() {
                self.reset();
            }
            ui.checkbox(&mut self.options.auto_restart, "Auto Restart");

            ui.separator();

            ui.label("Flow");
            ui.checkbox(&mut self.options.pace_ai, "Pace AI");
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
