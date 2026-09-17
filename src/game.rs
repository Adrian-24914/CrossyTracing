use crate::{
    random::Random,
    train::train_x_bounds,
    world::{
        Environment, ForestSectionKind, Lane, LaneKind, RailwayPhase, RailwayState, Section,
        SectionKind, SectionSignature, TrainDirection,
    },
};
use std::{
    collections::VecDeque,
    time::{SystemTime, UNIX_EPOCH},
};

pub const GRID_SIZE: usize = 7;
pub const TILE_COLUMNS: usize = GRID_SIZE;
pub const LANE_COUNT: usize = GRID_SIZE;
pub const TILE_SPACING: f32 = 1.35;
pub const BACK_Z: f32 = -4.05;
pub const WORLD_HALF_WIDTH: f32 = TILE_COLUMNS as f32 * TILE_SPACING * 0.5;
const DROP_START: f32 = 0.52;
const PLAYER_COLLISION_RADIUS: f32 = 0.33;
const RECENT_SECTION_LIMIT: usize = 6;
const BOARD_ATTEMPTS: usize = 512;
const SECTION_ATTEMPTS: usize = 96;
const RAILWAY_CHANCE_PERCENT: usize = 10;
const RIVER_CHANCE_PERCENT: usize = 35;
pub const RAILWAY_REST_SECONDS: f32 = 5.5;
pub const RAILWAY_WARNING_SECONDS: f32 = 1.5;
pub const TRAIN_CROSSING_SECONDS: f32 = 1.10;

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
    next_section_id: usize,
    pending_lanes: VecDeque<Lane>,
    recent_sections: VecDeque<SectionSignature>,
    random: Random,
}

impl Game {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xC0FFEE);
        Self::with_seed(seed)
    }

    fn with_seed(seed: u64) -> Self {
        let mut game = Self {
            lanes: Vec::with_capacity(LANE_COUNT),
            player_column: TILE_COLUMNS / 2,
            player_lane: LANE_COUNT - 3,
            cycle: 0.0,
            score: 0,
            game_over: false,
            paused: false,
            next_section_id: 0,
            pending_lanes: VecDeque::new(),
            recent_sections: VecDeque::with_capacity(RECENT_SECTION_LIMIT),
            random: Random::new(seed),
        };
        game.populate_initial_lanes();
        game
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn update(&mut self, delta_seconds: f32) -> bool {
        if self.paused || self.game_over || delta_seconds <= 0.0 {
            return false;
        }
        self.update_railways(delta_seconds);
        self.cycle += delta_seconds / self.cycle_seconds();
        while self.cycle >= 1.0 {
            self.cycle -= 1.0;
            self.recycle_front_lane();
        }
        self.check_train_collision();
        true
    }

    fn check_train_collision(&mut self) {
        let lane = &self.lanes[self.player_lane];
        let Some(railway) = &lane.railway else {
            return;
        };
        if railway.phase != RailwayPhase::Crossing {
            return;
        }

        let progress = (railway.elapsed / TRAIN_CROSSING_SECONDS).clamp(0.0, 1.0);
        let Some((train_min, train_max)) =
            train_x_bounds(progress, railway.direction, WORLD_HALF_WIDTH)
        else {
            return;
        };
        let player_x = column_x(self.player_column);
        if player_x + PLAYER_COLLISION_RADIUS >= train_min
            && player_x - PLAYER_COLLISION_RADIUS <= train_max
        {
            self.game_over = true;
        }
    }

    fn update_railways(&mut self, delta_seconds: f32) {
        for lane in &mut self.lanes {
            let Some(railway) = &mut lane.railway else {
                continue;
            };

            railway.elapsed += delta_seconds;
            loop {
                let duration = match railway.phase {
                    RailwayPhase::Rest => RAILWAY_REST_SECONDS,
                    RailwayPhase::Warning => RAILWAY_WARNING_SECONDS,
                    RailwayPhase::Crossing => TRAIN_CROSSING_SECONDS,
                };
                if railway.elapsed < duration {
                    break;
                }

                railway.elapsed -= duration;
                railway.phase = match railway.phase {
                    RailwayPhase::Rest => RailwayPhase::Warning,
                    RailwayPhase::Warning => RailwayPhase::Crossing,
                    RailwayPhase::Crossing => {
                        railway.direction = railway.direction.opposite();
                        RailwayPhase::Rest
                    }
                };
            }
        }
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
        let previous_lane = self.player_lane;
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

        if self.lanes[lane].blocks_movement(column) {
            return false;
        }
        let supported = self.lanes[lane].supports_player(column);
        self.player_column = column;
        self.player_lane = lane;
        if !supported {
            self.game_over = true;
        } else {
            self.check_train_collision();
            if !self.game_over {
                if lane < previous_lane {
                    self.score += 1;
                } else if lane > previous_lane {
                    self.score = self.score.saturating_sub(1);
                }
            }
        }
        true
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    fn populate_initial_lanes(&mut self) {
        let mut best_layout = None;
        let mut best_quality = 0;

        for _ in 0..BOARD_ATTEMPTS {
            self.pending_lanes.clear();
            self.recent_sections.clear();
            let lanes = self.generate_initial_world_window();
            let Some(lateral_moves) =
                minimum_lateral_moves(&lanes, self.player_lane, self.player_column, 0)
            else {
                continue;
            };

            let exit_count =
                reachable_columns_at_lane(&lanes, self.player_lane, self.player_column, 0).len();
            let quality = lateral_moves * 10 + exit_count * 3;
            if quality > best_quality {
                best_quality = quality;
                best_layout = Some((
                    lanes.clone(),
                    self.pending_lanes.clone(),
                    self.recent_sections.clone(),
                ));
            }

            if lateral_moves >= 2 && exit_count >= 2 {
                self.lanes = lanes;
                return;
            }
        }

        if let Some((lanes, pending_lanes, recent_sections)) = best_layout {
            self.lanes = lanes;
            self.pending_lanes = pending_lanes;
            self.recent_sections = recent_sections;
            return;
        }

        self.lanes = self.emergency_forest_window();
    }

    fn generate_initial_world_window(&mut self) -> Vec<Lane> {
        let mut stream = Vec::with_capacity(LANE_COUNT + 3);
        let starting_forest = self.generate_forest_section();
        stream.extend(starting_forest.lanes);
        while stream.len() < LANE_COUNT {
            let section = self.generate_next_section();
            stream.extend(section.lanes);
        }

        let future = stream.split_off(LANE_COUNT);
        self.pending_lanes.extend(future);
        stream.reverse();
        stream
    }

    fn recycle_front_lane(&mut self) {
        self.lanes.pop();
        let new_lane = self.next_valid_lane();
        self.lanes.insert(0, new_lane);
        self.player_lane += 1;
        if self.player_lane >= LANE_COUNT {
            self.player_lane = LANE_COUNT - 1;
            self.game_over = true;
        }
    }

    fn next_valid_lane(&mut self) -> Lane {
        for _ in 0..SECTION_ATTEMPTS {
            if self.pending_lanes.is_empty() {
                let section = self.generate_next_section();
                self.pending_lanes.extend(section.lanes);
            }

            let candidate = self.pending_lanes.pop_front().unwrap();
            let mut proposed = self.lanes.clone();
            proposed.insert(0, candidate.clone());
            let next_player_lane = (self.player_lane + 1).min(LANE_COUNT - 1);
            let exits =
                reachable_columns_at_lane(&proposed, next_player_lane, self.player_column, 0);
            if exits.len() >= 2 {
                return candidate;
            }

            let rejected_section = candidate.section_id;
            self.pending_lanes
                .retain(|lane| lane.section_id != rejected_section);
        }

        self.generate_emergency_lane()
    }

    fn generate_next_section(&mut self) -> Section {
        let previous_was_crossing = self
            .recent_sections
            .back()
            .map(|section| matches!(section.kind, SectionKind::River | SectionKind::Railway))
            .unwrap_or(false);
        if previous_was_crossing {
            return self.generate_forest_section();
        }

        let roll = self.random.range(100);
        if roll < RAILWAY_CHANCE_PERCENT {
            self.generate_railway_section()
        } else if roll < RAILWAY_CHANCE_PERCENT + RIVER_CHANCE_PERCENT {
            self.generate_river_section()
        } else {
            self.generate_forest_section()
        }
    }

    fn generate_forest_section(&mut self) -> Section {
        let section_id = self.next_section_id;
        let mut fallback = None;

        for _ in 0..SECTION_ATTEMPTS {
            let kind = self.choose_forest_kind();
            let section = self.generate_section_candidate(section_id, kind);
            if fallback.is_none() && section_has_crossing(&section.lanes) {
                fallback = Some(section.clone());
            }
            if section_has_crossing(&section.lanes)
                && !self.section_is_too_similar(&section.signature)
            {
                self.accept_section(&section.signature);
                return section;
            }
        }

        let section = fallback.unwrap_or_else(|| {
            self.generate_section_candidate(section_id, ForestSectionKind::Clearing)
        });
        self.accept_section(&section.signature);
        section
    }

    fn generate_river_section(&mut self) -> Section {
        let section_id = self.next_section_id;
        let mut fallback = None;

        for _ in 0..SECTION_ATTEMPTS {
            let section = self.generate_river_section_candidate(section_id);
            if fallback.is_none() && section_has_crossing(&section.lanes) {
                fallback = Some(section.clone());
            }
            if section_has_crossing(&section.lanes)
                && !self.section_is_too_similar(&section.signature)
            {
                self.accept_section(&section.signature);
                return section;
            }
        }

        let section = fallback.unwrap_or_else(|| self.generate_river_section_candidate(section_id));
        self.accept_section(&section.signature);
        section
    }

    fn generate_railway_section(&mut self) -> Section {
        let section_id = self.next_section_id;
        let direction = if self.random.range(2) == 0 {
            TrainDirection::LeftToRight
        } else {
            TrainDirection::RightToLeft
        };
        let lane = Lane {
            environment: Environment::Railway,
            section_kind: SectionKind::Railway,
            section_id,
            kind: LaneKind::Rail,
            obstacle_columns: Vec::new(),
            platform_columns: Vec::new(),
            railway: Some(RailwayState {
                phase: RailwayPhase::Warning,
                elapsed: 0.0,
                direction,
            }),
        };
        let section = Section::new(vec![lane], SectionKind::Railway);
        self.accept_section(&section.signature);
        section
    }

    fn generate_section_candidate(
        &mut self,
        section_id: usize,
        kind: ForestSectionKind,
    ) -> Section {
        let lane_count = match kind {
            ForestSectionKind::Clearing => 2 + self.random.range(2),
            ForestSectionKind::Grove => 2 + self.random.range(3),
            ForestSectionKind::Thicket => 2 + self.random.range(2),
        };
        let mut obstacle_counts = Vec::with_capacity(lane_count);
        for _ in 0..lane_count {
            obstacle_counts.push(match kind {
                ForestSectionKind::Clearing => 3 + self.random.range(2),
                ForestSectionKind::Grove => 3 + self.random.range(2),
                ForestSectionKind::Thicket => 4,
            });
        }
        match kind {
            ForestSectionKind::Clearing => {
                let open_lane = self.random.range(lane_count);
                obstacle_counts[open_lane] = if self.random.range(100) < 35 { 0 } else { 3 };
            }
            ForestSectionKind::Grove => {}
            ForestSectionKind::Thicket => obstacle_counts[self.random.range(lane_count)] = 4,
        }
        self.random.shuffle(&mut obstacle_counts);

        let mut lanes: Vec<Lane> = Vec::with_capacity(lane_count);
        for obstacle_count in obstacle_counts {
            let lane = (0..24)
                .map(|_| self.generate_forest_lane(section_id, kind, obstacle_count, None))
                .find(|candidate| {
                    lanes
                        .last()
                        .map(|previous| previous.layout_mask() != candidate.layout_mask())
                        .unwrap_or(true)
                })
                .unwrap();
            lanes.push(lane);
        }
        Section::new(lanes, SectionKind::Forest(kind))
    }

    fn generate_river_section_candidate(&mut self, section_id: usize) -> Section {
        let lane_count = 2 + self.random.range(3);
        let mut lanes: Vec<Lane> = Vec::with_capacity(lane_count);
        for _ in 0..lane_count {
            let platform_count = 3 + self.random.range(2);
            let lane = (0..24)
                .map(|_| self.generate_river_lane(section_id, platform_count))
                .find(|candidate| {
                    lanes
                        .last()
                        .map(|previous| previous.layout_mask() != candidate.layout_mask())
                        .unwrap_or(true)
                })
                .unwrap();
            lanes.push(lane);
        }
        Section::new(lanes, SectionKind::River)
    }

    fn generate_forest_lane(
        &mut self,
        section_id: usize,
        section_kind: ForestSectionKind,
        obstacle_count: usize,
        forced_open: Option<usize>,
    ) -> Lane {
        let mut obstacle_columns = Vec::with_capacity(obstacle_count);
        while obstacle_columns.len() < obstacle_count {
            let column = self.random.range(TILE_COLUMNS);
            if Some(column) != forced_open && !obstacle_columns.contains(&column) {
                obstacle_columns.push(column);
            }
        }
        obstacle_columns.sort_unstable();

        Lane {
            environment: Environment::Forest,
            section_kind: SectionKind::Forest(section_kind),
            section_id,
            kind: if self.random.range(7) == 0 {
                LaneKind::Stone
            } else {
                LaneKind::Grass
            },
            obstacle_columns,
            platform_columns: Vec::new(),
            railway: None,
        }
    }

    fn generate_river_lane(&mut self, section_id: usize, platform_count: usize) -> Lane {
        let pair_start = self.random.range(TILE_COLUMNS - 1);
        let mut platform_columns = vec![pair_start, pair_start + 1];
        while platform_columns.len() < platform_count {
            let column = self.random.range(TILE_COLUMNS);
            if !platform_columns.contains(&column) {
                platform_columns.push(column);
            }
        }
        platform_columns.sort_unstable();

        Lane {
            environment: Environment::River,
            section_kind: SectionKind::River,
            section_id,
            kind: LaneKind::Water,
            obstacle_columns: Vec::new(),
            platform_columns,
            railway: None,
        }
    }

    fn choose_forest_kind(&mut self) -> ForestSectionKind {
        let previous_kind =
            self.recent_sections
                .iter()
                .rev()
                .find_map(|section| match section.kind {
                    SectionKind::Forest(kind) => Some(kind),
                    SectionKind::River | SectionKind::Railway => None,
                });
        for _ in 0..12 {
            let roll = self.random.range(100);
            let kind = match self.score {
                0..=9 if roll < 42 => ForestSectionKind::Clearing,
                0..=9 if roll < 87 => ForestSectionKind::Grove,
                0..=9 => ForestSectionKind::Thicket,
                10..=34 if roll < 28 => ForestSectionKind::Clearing,
                10..=34 if roll < 72 => ForestSectionKind::Grove,
                10..=34 => ForestSectionKind::Thicket,
                _ if roll < 18 => ForestSectionKind::Clearing,
                _ if roll < 55 => ForestSectionKind::Grove,
                _ => ForestSectionKind::Thicket,
            };
            if Some(kind) != previous_kind {
                return kind;
            }
        }

        match previous_kind {
            Some(ForestSectionKind::Clearing) => ForestSectionKind::Grove,
            Some(ForestSectionKind::Grove) => ForestSectionKind::Thicket,
            _ => ForestSectionKind::Clearing,
        }
    }

    fn section_is_too_similar(&self, candidate: &SectionSignature) -> bool {
        self.recent_sections.iter().any(|previous| {
            if previous.kind != candidate.kind
                || previous.lane_masks.len() != candidate.lane_masks.len()
            {
                return false;
            }

            let direct_difference = mask_difference(&previous.lane_masks, &candidate.lane_masks);
            let mirrored: Vec<_> = candidate
                .lane_masks
                .iter()
                .map(|mask| mirror_mask(*mask))
                .collect();
            let mirrored_difference = mask_difference(&previous.lane_masks, &mirrored);
            direct_difference.min(mirrored_difference) <= 2
        })
    }

    fn accept_section(&mut self, signature: &SectionSignature) {
        self.next_section_id += 1;
        self.recent_sections.push_back(signature.clone());
        while self.recent_sections.len() > RECENT_SECTION_LIMIT {
            self.recent_sections.pop_front();
        }
    }

    fn generate_emergency_lane(&mut self) -> Lane {
        let section_id = self.next_section_id;
        self.next_section_id += 1;
        self.generate_forest_lane(section_id, ForestSectionKind::Clearing, 0, None)
    }

    fn emergency_forest_window(&mut self) -> Vec<Lane> {
        let mut lanes = Vec::with_capacity(LANE_COUNT);
        for lane_index in 0..LANE_COUNT {
            let obstacle_count = 3 + lane_index % 2;
            lanes.push(self.generate_forest_lane(
                self.next_section_id,
                ForestSectionKind::Grove,
                obstacle_count,
                Some(self.player_column),
            ));
        }
        self.next_section_id += 1;
        lanes
    }

    fn cycle_seconds(&self) -> f32 {
        match self.score {
            0..=9 => 1.00,
            10..=19 => 0.86,
            20..=34 => 0.74,
            35..=49 => 0.64,
            _ => 0.54,
        }
    }
}

fn section_has_crossing(lanes: &[Lane]) -> bool {
    let mut ordered = lanes.to_vec();
    ordered.reverse();
    (0..TILE_COLUMNS).any(|column| {
        ordered[ordered.len() - 1].supports_player(column)
            && has_path(&ordered, ordered.len() - 1, column, 0)
    })
}

fn reachable_columns_at_lane(
    lanes: &[Lane],
    start_lane: usize,
    start_column: usize,
    target_lane: usize,
) -> Vec<usize> {
    let visited = reachable_tiles(lanes, start_lane, start_column);
    if target_lane >= visited.len() {
        return Vec::new();
    }
    visited[target_lane]
        .iter()
        .enumerate()
        .filter_map(|(column, reachable)| reachable.then_some(column))
        .collect()
}

fn has_path(lanes: &[Lane], start_lane: usize, start_column: usize, target_lane: usize) -> bool {
    !reachable_columns_at_lane(lanes, start_lane, start_column, target_lane).is_empty()
}

fn reachable_tiles(lanes: &[Lane], start_lane: usize, start_column: usize) -> Vec<Vec<bool>> {
    let mut visited = vec![vec![false; TILE_COLUMNS]; lanes.len()];
    if start_lane >= lanes.len()
        || start_column >= TILE_COLUMNS
        || !lanes[start_lane].supports_player(start_column)
    {
        return visited;
    }

    let mut queue = VecDeque::from([(start_lane, start_column)]);
    visited[start_lane][start_column] = true;
    while let Some((lane, column)) = queue.pop_front() {
        for (next_lane, next_column) in forward_neighbors(lane, column) {
            if !visited[next_lane][next_column] && lanes[next_lane].supports_player(next_column) {
                visited[next_lane][next_column] = true;
                queue.push_back((next_lane, next_column));
            }
        }
    }
    visited
}

fn minimum_lateral_moves(
    lanes: &[Lane],
    start_lane: usize,
    start_column: usize,
    target_lane: usize,
) -> Option<usize> {
    if start_lane >= lanes.len() || !lanes[start_lane].supports_player(start_column) {
        return None;
    }

    let mut distances = vec![vec![usize::MAX; TILE_COLUMNS]; lanes.len()];
    let mut queue = VecDeque::from([(start_lane, start_column)]);
    distances[start_lane][start_column] = 0;
    while let Some((lane, column)) = queue.pop_front() {
        let current_distance = distances[lane][column];
        for (next_lane, next_column) in forward_neighbors(lane, column) {
            if !lanes[next_lane].supports_player(next_column) {
                continue;
            }
            let lateral_cost = usize::from(next_column != column);
            let next_distance = current_distance + lateral_cost;
            if next_distance < distances[next_lane][next_column] {
                distances[next_lane][next_column] = next_distance;
                if lateral_cost == 0 {
                    queue.push_front((next_lane, next_column));
                } else {
                    queue.push_back((next_lane, next_column));
                }
            }
        }
    }

    distances
        .get(target_lane)?
        .iter()
        .copied()
        .filter(|distance| *distance != usize::MAX)
        .min()
}

fn forward_neighbors(lane: usize, column: usize) -> Vec<(usize, usize)> {
    [
        lane.checked_sub(1).map(|next| (next, column)),
        column.checked_sub(1).map(|next| (lane, next)),
        (column + 1 < TILE_COLUMNS).then_some((lane, column + 1)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn mask_difference(first: &[u8], second: &[u8]) -> u32 {
    first
        .iter()
        .zip(second)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum()
}

fn mirror_mask(mask: u8) -> u8 {
    (0..TILE_COLUMNS).fold(0, |mirrored, column| {
        if mask & (1 << column) != 0 {
            mirrored | (1 << (TILE_COLUMNS - 1 - column))
        } else {
            mirrored
        }
    })
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn column_x(column: usize) -> f32 {
    (column as f32 - (TILE_COLUMNS as f32 - 1.0) * 0.5) * TILE_SPACING
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place_test_railway(game: &mut Game, lane_index: usize) -> usize {
        let railway = game.generate_railway_section().lanes.remove(0);
        game.lanes[lane_index] = railway;
        lane_index
    }

    #[test]
    fn initial_board_has_a_path_that_requires_lateral_movement() {
        for seed in 0..100 {
            let game = Game::with_seed(seed);
            let lateral_moves =
                minimum_lateral_moves(&game.lanes, game.player_lane, game.player_column, 0)
                    .unwrap();
            assert!(
                lateral_moves >= 2,
                "seed {seed} only required {lateral_moves}"
            );
        }
    }

    #[test]
    fn initial_board_has_a_safe_start() {
        let game = Game::with_seed(99);
        assert_eq!(
            game.lanes[game.player_lane].environment,
            Environment::Forest
        );
        assert!(game.lanes[game.player_lane].supports_player(game.player_column));
        for lane in &game.lanes {
            match lane.environment {
                Environment::Forest => {
                    assert!(
                        lane.obstacle_columns.is_empty()
                            || (3..=4).contains(&lane.obstacle_columns.len())
                    )
                }
                Environment::River => {
                    assert!((3..=4).contains(&lane.platform_columns.len()))
                }
                Environment::Railway => assert!(lane.obstacle_columns.is_empty()),
            }
        }

        let mut completed = Vec::new();
        let mut previous = None;
        for lane in &game.lanes {
            if previous != Some(lane.section_id) {
                assert!(!completed.contains(&lane.section_id));
                if let Some(section_id) = previous {
                    completed.push(section_id);
                }
                previous = Some(lane.section_id);
            }
        }
    }

    #[test]
    fn forest_sections_never_exceed_four_obstacles() {
        let mut game = Game::with_seed(12);
        let clearing = game.generate_section_candidate(100, ForestSectionKind::Clearing);
        let thicket = game.generate_section_candidate(101, ForestSectionKind::Thicket);
        assert!(clearing
            .lanes
            .iter()
            .any(|lane| lane.obstacle_columns.len() == 3));
        assert!(thicket
            .lanes
            .iter()
            .all(|lane| lane.obstacle_columns.len() == 4));
        assert!(clearing
            .lanes
            .iter()
            .chain(&thicket.lanes)
            .all(|lane| lane.obstacle_columns.len() <= 4));
    }

    #[test]
    fn clearings_sometimes_include_one_fully_open_lane() {
        let mut game = Game::with_seed(44);
        let mut sections_with_open_lane = 0;
        for section_id in 0..100 {
            let section = game.generate_section_candidate(section_id, ForestSectionKind::Clearing);
            let open_lanes = section
                .lanes
                .iter()
                .filter(|lane| lane.obstacle_columns.is_empty())
                .count();
            assert!(open_lanes <= 1);
            sections_with_open_lane += usize::from(open_lanes == 1);
        }
        assert!((20..=50).contains(&sections_with_open_lane));
    }

    #[test]
    fn initial_board_exposes_multiple_routes() {
        for seed in 0..100 {
            let game = Game::with_seed(seed);
            let exits =
                reachable_columns_at_lane(&game.lanes, game.player_lane, game.player_column, 0);
            assert!(exits.len() >= 2, "seed {seed} only exposed {exits:?}");
        }
    }

    #[test]
    fn equal_seeds_produce_equal_world_layouts() {
        let first = Game::with_seed(123);
        let second = Game::with_seed(123);
        for (first_lane, second_lane) in first.lanes.iter().zip(&second.lanes) {
            assert_eq!(first_lane.section_kind, second_lane.section_kind);
            assert_eq!(first_lane.obstacle_columns, second_lane.obstacle_columns);
            assert_eq!(first_lane.platform_columns, second_lane.platform_columns);
        }
    }

    #[test]
    fn recent_section_memory_stays_bounded() {
        let mut game = Game::with_seed(31);
        for _ in 0..20 {
            game.generate_forest_section();
        }
        assert_eq!(game.recent_sections.len(), RECENT_SECTION_LIMIT);
    }

    #[test]
    fn player_can_move_to_an_empty_tile() {
        let mut game = Game::with_seed(1);
        game.player_column = 3;
        let lane = game.generate_forest_lane(700, ForestSectionKind::Clearing, 0, None);
        game.lanes[game.player_lane] = lane;
        assert!(game.try_move(Move::Left));
        assert_eq!(game.player_column, 2);
    }

    #[test]
    fn player_cannot_enter_an_obstacle() {
        let mut game = Game::with_seed(1);
        game.player_column = 3;
        let lane = game.generate_forest_lane(701, ForestSectionKind::Clearing, 1, Some(3));
        game.lanes[game.player_lane] = lane;
        game.lanes[game.player_lane].obstacle_columns = vec![2];
        assert!(!game.try_move(Move::Left));
        assert_eq!(game.player_column, 3);
    }

    #[test]
    fn paused_game_blocks_player_movement() {
        let mut game = Game::with_seed(1);
        let initial_column = game.player_column;
        game.toggle_pause();
        assert!(!game.try_move(Move::Left));
        assert_eq!(game.player_column, initial_column);
    }

    #[test]
    fn moving_forward_adds_a_point() {
        let mut game = Game::with_seed(5);
        let target_lane = game.player_lane - 1;
        let player_column = game.player_column;
        match game.lanes[target_lane].environment {
            Environment::Forest => game.lanes[target_lane]
                .obstacle_columns
                .retain(|column| *column != player_column),
            Environment::River => {
                if !game.lanes[target_lane]
                    .platform_columns
                    .contains(&player_column)
                {
                    game.lanes[target_lane].platform_columns.push(player_column);
                }
            }
            Environment::Railway => {}
        }
        assert!(game.try_move(Move::Forward));
        assert_eq!(game.score, 1);
    }

    #[test]
    fn moving_backward_removes_the_point_before_it_can_be_recovered() {
        let mut game = Game::with_seed(5);
        let current_lane = game.player_lane;
        let back_lane = current_lane + 1;
        game.lanes[current_lane] =
            game.generate_forest_lane(800, ForestSectionKind::Clearing, 0, None);
        game.lanes[back_lane] =
            game.generate_forest_lane(801, ForestSectionKind::Clearing, 0, None);
        game.score = 7;

        assert!(game.try_move(Move::Backward));
        assert_eq!(game.score, 6);
        assert!(game.try_move(Move::Forward));
        assert_eq!(game.score, 7);
        assert!(game.try_move(Move::Backward));
        assert!(game.try_move(Move::Forward));
        assert_eq!(game.score, 7);
    }

    #[test]
    fn moving_backward_never_makes_the_score_underflow() {
        let mut game = Game::with_seed(5);
        let back_lane = game.player_lane + 1;
        game.lanes[back_lane] =
            game.generate_forest_lane(802, ForestSectionKind::Clearing, 0, None);
        game.score = 0;

        assert!(game.try_move(Move::Backward));
        assert_eq!(game.score, 0);
    }

    #[test]
    fn entering_river_without_a_log_ends_the_run() {
        let mut game = Game::with_seed(18);
        let target_lane = game.player_lane - 1;
        let player_column = game.player_column;
        let mut river = game.generate_river_lane(900, 3);
        river
            .platform_columns
            .retain(|column| *column != player_column);
        game.lanes[target_lane] = river;

        assert!(game.try_move(Move::Forward));
        assert!(game.game_over);
        assert_eq!(game.score, 0);
        assert_eq!(game.player_lane, target_lane);
    }

    #[test]
    fn a_static_log_supports_the_player() {
        let mut game = Game::with_seed(21);
        let target_lane = game.player_lane - 1;
        let player_column = game.player_column;
        let mut river = game.generate_river_lane(901, 3);
        if !river.platform_columns.contains(&player_column) {
            river.platform_columns[0] = player_column;
            river.platform_columns.sort_unstable();
            river.platform_columns.dedup();
        }
        game.lanes[target_lane] = river;

        assert!(game.try_move(Move::Forward));
        assert!(!game.game_over);
        assert_eq!(game.player_lane, target_lane);
    }

    #[test]
    fn railway_is_safe_across_the_whole_row() {
        let mut game = Game::with_seed(24);
        let railway = game.generate_railway_section();
        assert_eq!(railway.lanes.len(), 1);
        assert!((0..TILE_COLUMNS).all(|column| railway.lanes[0].supports_player(column)));
        let state = railway.lanes[0].railway.as_ref().unwrap();
        assert_eq!(state.phase, RailwayPhase::Warning);
        assert_eq!(state.elapsed, 0.0);
    }

    #[test]
    fn environment_generation_uses_the_requested_distribution() {
        assert_eq!(RAILWAY_CHANCE_PERCENT, 10);
        assert_eq!(RIVER_CHANCE_PERCENT, 35);
        assert_eq!(100 - RAILWAY_CHANCE_PERCENT - RIVER_CHANCE_PERCENT, 55);
    }

    #[test]
    fn railway_warns_before_the_train_crosses() {
        let mut game = Game::with_seed(24);
        let railway_index = place_test_railway(&mut game, 0);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Rest;
        railway.elapsed = RAILWAY_REST_SECONDS - 0.05;

        game.update_railways(0.06);
        let railway = game.lanes[railway_index].railway.as_ref().unwrap();
        assert_eq!(railway.phase, RailwayPhase::Warning);
        assert!(railway.elapsed < 0.02);

        game.update_railways(RAILWAY_WARNING_SECONDS);
        assert_eq!(
            game.lanes[railway_index].railway.as_ref().unwrap().phase,
            RailwayPhase::Crossing
        );
    }

    #[test]
    fn pause_freezes_the_railway_cycle() {
        let mut game = Game::with_seed(24);
        let railway_index = place_test_railway(&mut game, 0);
        let before = game.lanes[railway_index].railway.clone().unwrap();

        game.toggle_pause();
        assert!(!game.update(1.0));
        assert_eq!(game.lanes[railway_index].railway.as_ref().unwrap(), &before);
    }

    #[test]
    fn each_train_crosses_in_the_opposite_direction() {
        let mut game = Game::with_seed(24);
        let railway_index = place_test_railway(&mut game, 0);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Crossing;
        railway.elapsed = TRAIN_CROSSING_SECONDS - 0.05;
        let first_direction = railway.direction;

        game.update_railways(0.06);
        let railway = game.lanes[railway_index].railway.as_ref().unwrap();
        assert_eq!(railway.phase, RailwayPhase::Rest);
        assert_eq!(railway.direction, first_direction.opposite());
    }

    #[test]
    fn crossing_train_hits_a_player_waiting_on_the_rails() {
        let mut game = Game::with_seed(30);
        let railway_index = game.player_lane;
        place_test_railway(&mut game, railway_index);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Crossing;
        railway.elapsed = TRAIN_CROSSING_SECONDS * 0.5;

        game.update(0.01);
        assert!(game.game_over);
    }

    #[test]
    fn warning_lights_do_not_hurt_the_player() {
        let mut game = Game::with_seed(30);
        let railway_index = game.player_lane;
        place_test_railway(&mut game, railway_index);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Warning;
        railway.elapsed = 0.5;

        game.update(0.01);
        assert!(!game.game_over);
    }

    #[test]
    fn train_does_not_hit_a_player_on_another_lane() {
        let mut game = Game::with_seed(30);
        let railway_index = game.player_lane - 1;
        place_test_railway(&mut game, railway_index);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Crossing;
        railway.elapsed = TRAIN_CROSSING_SECONDS * 0.5;

        game.update(0.01);
        assert!(!game.game_over);
    }

    #[test]
    fn walking_into_a_passing_train_ends_the_run_without_scoring() {
        let mut game = Game::with_seed(30);
        let railway_index = game.player_lane - 1;
        place_test_railway(&mut game, railway_index);
        let railway = game.lanes[railway_index].railway.as_mut().unwrap();
        railway.phase = RailwayPhase::Crossing;
        railway.elapsed = TRAIN_CROSSING_SECONDS * 0.5;

        assert!(game.try_move(Move::Forward));
        assert!(game.game_over);
        assert_eq!(game.score, 0);
    }

    #[test]
    fn crossings_are_separated_by_forest_sections() {
        let mut game = Game::with_seed(25);
        let mut previous_kind = None;
        for _ in 0..100 {
            let section = game.generate_next_section();
            let current_kind = section.signature.kind;
            let previous_was_crossing = previous_kind
                .is_some_and(|kind| matches!(kind, SectionKind::River | SectionKind::Railway));
            let current_is_crossing =
                matches!(current_kind, SectionKind::River | SectionKind::Railway);
            assert!(!(previous_was_crossing && current_is_crossing));
            previous_kind = Some(current_kind);
        }
    }

    #[test]
    fn many_recycled_boards_remain_solvable() {
        for seed in 0..100 {
            let mut game = Game::with_seed(seed);
            for step in 0..20 {
                game.recycle_front_lane();
                assert!(
                    has_path(&game.lanes, game.player_lane, game.player_column, 0),
                    "seed {seed} failed after recycle {step}"
                );
                assert!(
                    reachable_columns_at_lane(&game.lanes, game.player_lane, game.player_column, 0,)
                        .len() >= 2,
                    "seed {seed} exposed only one route after recycle {step}"
                );

                let next_lane = game.player_lane - 1;
                let reachable = reachable_columns_at_lane(
                    &game.lanes,
                    game.player_lane,
                    game.player_column,
                    next_lane,
                );
                game.player_column = reachable
                    .into_iter()
                    .find(|column| has_path(&game.lanes, next_lane, *column, 0))
                    .unwrap();
                game.player_lane = next_lane;
            }
        }
    }

    #[test]
    fn speed_increases_at_the_requested_score_thresholds() {
        let mut game = Game::with_seed(7);
        let mut speeds = Vec::new();
        for score in [0, 10, 20, 35, 50] {
            game.score = score;
            speeds.push(game.cycle_seconds());
        }
        assert!(speeds.windows(2).all(|pair| pair[1] < pair[0]));
    }

    #[test]
    fn reset_restores_the_initial_state() {
        let mut game = Game::with_seed(4);
        game.score = 25;
        game.game_over = true;
        game.reset();
        assert_eq!(game.score, 0);
        assert!(!game.game_over);
        assert!(!game.paused);
        assert_eq!(game.player_lane, LANE_COUNT - 3);
    }
}
