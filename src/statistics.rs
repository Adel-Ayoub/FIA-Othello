use std::{collections::HashMap, fmt::Display};

use crate::{board::Player, referee::Outcome};

pub struct Statistic {
    win_ratio: f64,
    tie_ratio: f64,
    lose_ratio: f64,
    count: f64,
}
impl Default for Statistic {
    fn default() -> Self {
        Statistic {
            win_ratio: 0.0,
            tie_ratio: 0.0,
            lose_ratio: 0.0,
            count: 0.0,
        }
    }
}

impl Display for Statistic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.1}%, {:.1}%, {:.1}%, ({:.0})",
            self.win_ratio * 100.0,
            self.tie_ratio * 100.0,
            self.lose_ratio * 100.0,
            self.count
        )
    }
}

pub struct Statistics {
    pub data: HashMap<String, Statistic>,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            data: HashMap::new(),
        }
    }
}

impl Statistics {
    pub fn add_datum(&mut self, name: String, player: Player, outcome: &Outcome) {
        let statistic = self.data.entry(name).or_insert(Statistic::default());
        let (win_value, tie_value, lose_value) = match *outcome {
            Outcome::Won(winning_player) => {
                if player == winning_player {
                    (1.0, 0.0, 0.0)
                } else {
                    (0.0, 0.0, 1.0)
                }
            }
            Outcome::Tie => (0.0, 1.0, 0.0),
        };

        let mut ratios = [
            (&mut statistic.win_ratio, win_value),
            (&mut statistic.tie_ratio, tie_value),
            (&mut statistic.lose_ratio, lose_value),
        ];

        for (ratio, value) in ratios.iter_mut() {
            let new_ratio = **ratio * statistic.count + *value;
            **ratio = new_ratio / (statistic.count + 1.0);
        }
        statistic.count += 1.0;
    }
}
