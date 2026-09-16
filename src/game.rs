pub const GRID_SIZE: usize = 7;
pub const TILE_COLUMNS: usize = GRID_SIZE;
pub const LANE_COUNT: usize = GRID_SIZE;
pub const TILE_SPACING: f32 = 1.35;
pub const BACK_Z: f32 = -4.05;

#[derive(Clone, Copy)]
pub enum LaneKind {
    Grass,
    Stone,
}

pub struct Lane {
    pub kind: LaneKind,
    pub obstacle_column: Option<usize>,
}

pub struct Game {
    pub lanes: Vec<Lane>,
    pub player_column: usize,
    pub player_lane: usize,
}

impl Game {
    pub fn new() -> Self {
        let mut lanes = Vec::with_capacity(LANE_COUNT);
        for generation in 0..LANE_COUNT {
            let obstacle_column = if generation % 3 == 0 {
                None
            } else {
                Some((generation * 7 + 1) % TILE_COLUMNS)
            };
            lanes.push(Lane {
                kind: if generation % 2 == 0 {
                    LaneKind::Grass
                } else {
                    LaneKind::Stone
                },
                obstacle_column,
            });
        }

        let player_lane = LANE_COUNT - 3;
        lanes[player_lane].obstacle_column = None;
        Self {
            lanes,
            player_column: TILE_COLUMNS / 2,
            player_lane,
        }
    }

    pub fn lane_z(&self, lane: usize) -> f32 {
        BACK_Z + lane as f32 * TILE_SPACING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_board_has_seven_safe_lanes() {
        let game = Game::new();
        assert_eq!(game.lanes.len(), LANE_COUNT);
        assert_eq!(game.player_column, TILE_COLUMNS / 2);
        assert_eq!(game.lanes[game.player_lane].obstacle_column, None);
    }
}
