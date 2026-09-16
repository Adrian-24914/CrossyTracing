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

#[derive(Clone, Copy)]
pub enum Move {
    Left,
    Right,
    Forward,
    Backward,
}

pub struct Game {
    pub lanes: Vec<Lane>,
    pub player_column: usize,
    pub player_lane: usize,
    pub paused: bool,
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
            paused: false,
        }
    }

    pub fn lane_z(&self, lane: usize) -> f32 {
        BACK_Z + lane as f32 * TILE_SPACING
    }

    pub fn try_move(&mut self, direction: Move) -> bool {
        if self.paused {
            return false;
        }
        let (column, lane) = match direction {
            Move::Left if self.player_column > 0 => (self.player_column - 1, self.player_lane),
            Move::Right if self.player_column + 1 < TILE_COLUMNS => {
                (self.player_column + 1, self.player_lane)
            }
            Move::Forward if self.player_lane > 0 => (self.player_column, self.player_lane - 1),
            Move::Backward if self.player_lane + 1 < LANE_COUNT => {
                (self.player_column, self.player_lane + 1)
            }
            _ => return false,
        };

        if self.lanes[lane].obstacle_column == Some(column) {
            return false;
        }
        self.player_column = column;
        self.player_lane = lane;
        true
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
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

    #[test]
    fn player_can_move_to_an_empty_tile() {
        let mut game = Game::new();
        assert!(game.try_move(Move::Left));
        assert_eq!(game.player_column, TILE_COLUMNS / 2 - 1);
    }

    #[test]
    fn player_cannot_enter_an_obstacle() {
        let mut game = Game::new();
        let blocked_column = game.player_column - 1;
        game.lanes[game.player_lane].obstacle_column = Some(blocked_column);

        assert!(!game.try_move(Move::Left));
        assert_eq!(game.player_column, TILE_COLUMNS / 2);
    }

    #[test]
    fn paused_game_blocks_player_movement() {
        let mut game = Game::new();
        game.toggle_pause();

        assert!(!game.try_move(Move::Left));
        assert_eq!(game.player_column, TILE_COLUMNS / 2);
    }
}
