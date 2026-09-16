pub const GRID_SIZE: usize = 7;
pub const TILE_COLUMNS: usize = GRID_SIZE;
pub const LANE_COUNT: usize = GRID_SIZE;
pub const TILE_SPACING: f32 = 1.35;
pub const BACK_Z: f32 = -4.05;
const CYCLE_SECONDS: f32 = 2.0;
const DROP_START: f32 = 0.52;

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
    pub cycle: f32,
    pub score: u32,
    pub game_over: bool,
    pub paused: bool,
    next_generation: usize,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Self {
            lanes: Vec::with_capacity(LANE_COUNT),
            player_column: TILE_COLUMNS / 2,
            player_lane: LANE_COUNT - 3,
            cycle: 0.0,
            score: 0,
            game_over: false,
            paused: false,
            next_generation: 0,
        };
        for _ in 0..LANE_COUNT {
            let lane = game.generate_lane();
            game.lanes.push(lane);
        }
        game.lanes[game.player_lane].obstacle_column = None;
        game
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn update(&mut self, delta_seconds: f32) -> bool {
        if self.paused || self.game_over || delta_seconds <= 0.0 {
            return false;
        }
        self.cycle += delta_seconds / CYCLE_SECONDS;
        while self.cycle >= 1.0 {
            self.cycle -= 1.0;
            self.recycle_front_lane();
        }
        true
    }

    pub fn lane_position(&self, lane: usize) -> (f32, f32) {
        let z = BACK_Z + (lane as f32 + self.cycle) * TILE_SPACING;
        let y = if lane == LANE_COUNT - 1 {
            -smoothstep(((self.cycle - DROP_START) / (1.0 - DROP_START)).clamp(0.0, 1.0)) * 2.8
        } else {
            0.0
        };
        (y, z)
    }

    pub fn try_move(&mut self, direction: Move) -> bool {
        if self.paused || self.game_over {
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
        if lane < self.player_lane {
            self.score += 1;
        }
        self.player_column = column;
        self.player_lane = lane;
        true
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    fn recycle_front_lane(&mut self) {
        self.lanes.pop();
        let new_lane = self.generate_lane();
        self.lanes.insert(0, new_lane);
        self.player_lane += 1;
        if self.player_lane >= LANE_COUNT {
            self.player_lane = LANE_COUNT - 1;
            self.game_over = true;
        }
    }

    fn generate_lane(&mut self) -> Lane {
        let generation = self.next_generation;
        self.next_generation += 1;
        Lane {
            kind: if generation % 2 == 0 {
                LaneKind::Grass
            } else {
                LaneKind::Stone
            },
            obstacle_column: if generation % 3 == 0 {
                None
            } else {
                Some((generation * 7 + 1) % TILE_COLUMNS)
            },
        }
    }
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
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

    #[test]
    fn moving_forward_adds_a_point() {
        let mut game = Game::new();
        assert!(game.try_move(Move::Forward));
        assert_eq!(game.score, 1);
    }

    #[test]
    fn recycling_preserves_the_players_world_position() {
        let mut game = Game::new();
        let previous_lane = game.player_lane;

        game.update(CYCLE_SECONDS);

        assert_eq!(game.player_lane, previous_lane + 1);
        assert_eq!(game.lanes.len(), LANE_COUNT);
        assert_eq!(game.cycle, 0.0);
    }

    #[test]
    fn reset_restores_the_initial_state() {
        let mut game = Game::new();
        game.try_move(Move::Forward);
        game.update(CYCLE_SECONDS);

        game.reset();

        assert_eq!(game.score, 0);
        assert!(!game.game_over);
        assert!(!game.paused);
        assert_eq!(game.player_lane, LANE_COUNT - 3);
    }
}
