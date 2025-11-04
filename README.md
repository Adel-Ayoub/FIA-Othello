# FIA‑Othello


## A desktop Othello/Reversi game with AI, built in Rust (egui/eframe).


## Installation

```sh
# Run the app (debug)
make run

# Run the app (release)
make run-release

# Build only
make build
make release

# Format, lint, test
make fmt
make lint
make test
make check
```

## Command palette (Make targets)

```text
Othello Commands:

Building:
  make build       - Build debug version
  make release     - Build optimized release

Running:
  make run         - Run debug version
  make run-release - Run release version

Quality:
  make test        - Run tests
  make fmt         - Format code
  make lint        - Run clippy
  make check       - Run all checks

Maintenance:
  make clean       - Remove build artifacts
```

---

## Architecture overview

```text
mouse/keyboard ─┐                                   ┌─ window (egui/eframe)
                ▼                                   ▼
+----------------------+         UI events      +-------------------------+
|        GUI           | ─────────────────────▶ |  Game (game.rs)         |
|  (egui/eframe app)   | ◀──────────────────────| draws board + sidebar   |
+----------------------+                         +-------------------------+
                  │                                         │
                  │                                         ▼
                  │                          +----------------------------+
                  │                          | Referee (referee.rs)       |
                  │                          | validate moves, outcomes   |
                  │                          +----------------------------+
                  │                                         │
                  ▼                                         ▼
        +------------------+                 channels        +------------------+
        |  Agent (ai.rs)   |  ◀════ mpsc ════▶               |  AI thread       |
        | Random/Negamax   |                                 |                  |
        |     /Alpha-Beta  |  ════ mpsc ════▶                |  picks next move |
        +------------------+                                 +------------------+
                  │
                  ▼
        +------------------+       +------------------+       +----------------+
        |  Board (board.rs)|  ◀──▶ | CellList (common)|  ◀──▶ | Statistics     |
        | grid + rules     |       | small helper     |       | (statistics.rs)|
        +------------------+       +------------------+       +----------------+
```

----

## AI engine

- Negamax search for move selection
- Negamax with alpha–beta pruning for deeper, faster search

 

In the right panel of the UI you can:

- Enable AI per player
- Choose AI type: Random, Negamax, or Negamax (alpha–beta)
- Adjust Minimax recursion depth (1–10)
- Toggle helpers: Show Valid Moves, Show Effects of Moves
- Control flow: Pace AI, Pause at Win, Auto Restart
- Restart the game
- View live score and aggregated statistics

---

## Game options (from the UI)

- show_valid_moves: highlight legal moves for the current player
- show_effects_of_moves: visualize flips from a tentative move
- pace_ai: slow down AI responses slightly
- pause_at_win: hold the final board briefly
- auto_restart: automatically start a new game after finish
- statistics: collect win/tie/loss aggregates by matchup

---

### Platform notes

- macOS: `cargo run` should work out of the box.
- Linux CI installs a few GUI dependencies (GTK/X11/Wayland) for building `egui/eframe`.

## License

MIT License - see [LICENSE](LICENSE) file for details.
