mod board;
mod game;
mod agent;
mod common;
mod referee;
mod statistics;

use eframe::egui;
use game::Game;

fn main() {
    let app = Game::default();
    let _ = eframe::run_native(
        "Othello",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(app))),
    );
}
