use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    game::{Game, LANE_COUNT, TILE_COLUMNS, TILE_SPACING},
    math::Vec3,
    sphere::Sphere,
    train::{add_train, TRAIN_LENGTH},
    world::{Environment, ForestSectionKind, LaneKind, RailwayPhase, RailwayState, SectionKind},
};

const WORLD_HALF_WIDTH: f32 = TILE_COLUMNS as f32 * TILE_SPACING * 0.5;

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub spheres: Vec<Sphere>,
    pub cylinders: Vec<Cylinder>,
}

pub fn build_scene(game: &Game) -> Scene {
    let mut cubes = Vec::with_capacity(LANE_COUNT * (TILE_COLUMNS + 3));
    let mut spheres = Vec::new();
    let mut cylinders = Vec::new();

    for (lane_index, lane) in game.lanes.iter().enumerate() {
        let (lane_y, lane_z) = game.lane_position(lane_index);
        let base_color = match (lane.section_kind, lane.kind) {
            (SectionKind::Forest(ForestSectionKind::Clearing), LaneKind::Grass) => {
                Color::new(111, 177, 91)
            }
            (SectionKind::Forest(ForestSectionKind::Grove), LaneKind::Grass) => {
                Color::new(78, 145, 82)
            }
            (SectionKind::Forest(ForestSectionKind::Thicket), LaneKind::Grass) => {
                Color::new(58, 119, 72)
            }
            (SectionKind::Forest(_), LaneKind::Stone) => Color::new(107, 123, 112),
            (SectionKind::River, LaneKind::Water) => Color::new(52, 129, 164),
            (SectionKind::Railway, LaneKind::Rail) => Color::new(89, 84, 78),
            _ => Color::new(92, 145, 102),
        };

        let obstacle_color = match lane.section_kind {
            SectionKind::Forest(ForestSectionKind::Clearing) => Color::new(166, 105, 62),
            SectionKind::Forest(ForestSectionKind::Grove) => Color::new(132, 82, 52),
            SectionKind::Forest(ForestSectionKind::Thicket) => Color::new(99, 68, 48),
            SectionKind::River => Color::new(137, 84, 48),
            SectionKind::Railway => Color::new(78, 73, 68),
        };

        let (tile_y, tile_height) = match lane.environment {
            Environment::Forest => (lane_y - 0.68, 1.64),
            Environment::River => (lane_y - 0.80, 1.38),
            Environment::Railway => (lane_y - 0.71, 1.52),
        };

        for column in 0..TILE_COLUMNS {
            let color = if column % 2 == 0 {
                base_color
            } else {
                tint(base_color, 10)
            };
            cubes.push(Cube::new(
                Vec3::new(column_x(column), tile_y, lane_z),
                Vec3::new(TILE_SPACING - 0.02, tile_height, TILE_SPACING - 0.02),
                color,
            ));
        }

        match lane.environment {
            Environment::Forest => {
                for &column in &lane.obstacle_columns {
                    cubes.push(Cube::new(
                        Vec3::new(column_x(column), lane_y + 0.55, lane_z),
                        Vec3::new(0.72, 0.82, 0.72),
                        obstacle_color,
                    ));
                }
            }
            Environment::River => {
                for (start, end) in contiguous_runs(&lane.platform_columns) {
                    let start_x = column_x(start);
                    let end_x = column_x(end);
                    cylinders.push(Cylinder::new_x(
                        Vec3::new((start_x + end_x) * 0.5, lane_y + 0.13, lane_z),
                        end_x - start_x + TILE_SPACING * 0.82,
                        0.48,
                        Color::new(137, 84, 48),
                    ));
                }
            }
            Environment::Railway => {
                for column in 0..TILE_COLUMNS {
                    cubes.push(Cube::new(
                        Vec3::new(column_x(column), lane_y + 0.12, lane_z),
                        Vec3::new(0.22, 0.12, 1.02),
                        Color::new(104, 70, 47),
                    ));
                }
                for z_offset in [-0.31, 0.31] {
                    cylinders.push(Cylinder::new_x(
                        Vec3::new(0.0, lane_y + 0.24, lane_z + z_offset),
                        TILE_COLUMNS as f32 * TILE_SPACING,
                        0.12,
                        Color::new(151, 157, 157),
                    ));
                }

                if let Some(railway) = &lane.railway {
                    add_railway_signals(&mut cubes, &mut spheres, lane_y, lane_z, railway);
                    if railway.phase == RailwayPhase::Crossing {
                        let progress =
                            (railway.elapsed / crate::game::TRAIN_CROSSING_SECONDS).clamp(0.0, 1.0);
                        let outside_center = WORLD_HALF_WIDTH + TRAIN_LENGTH * 0.5;
                        let center_x = railway.direction.sign()
                            * (-outside_center + 2.0 * outside_center * progress);
                        add_train(
                            &mut cubes,
                            center_x,
                            lane_y,
                            lane_z,
                            railway.direction,
                            WORLD_HALF_WIDTH,
                        );
                    }
                }
            }
        }
    }

    spheres.extend(player_spheres(game));
    Scene {
        cubes,
        spheres,
        cylinders,
    }
}

fn add_railway_signals(
    cubes: &mut Vec<Cube>,
    spheres: &mut Vec<Sphere>,
    lane_y: f32,
    lane_z: f32,
    railway: &RailwayState,
) {
    let warning_on = railway.phase == RailwayPhase::Warning
        && ((railway.elapsed / 0.20).floor() as u32).is_multiple_of(2);
    let light_color = if warning_on {
        Color::new(255, 45, 28)
    } else {
        Color::new(82, 29, 24)
    };

    for x in [-WORLD_HALF_WIDTH + 0.18, WORLD_HALF_WIDTH - 0.18] {
        cubes.push(Cube::new(
            Vec3::new(x, lane_y + 0.57, lane_z + 0.49),
            Vec3::new(0.10, 0.86, 0.10),
            Color::new(55, 58, 56),
        ));
        spheres.push(Sphere::new(
            Vec3::new(x, lane_y + 0.93, lane_z + 0.49),
            0.24,
            light_color,
        ));
    }
}

fn contiguous_runs(columns: &[usize]) -> Vec<(usize, usize)> {
    let Some(&first) = columns.first() else {
        return Vec::new();
    };
    let mut runs = Vec::new();
    let mut start = first;
    let mut end = first;
    for &column in &columns[1..] {
        if column == end + 1 {
            end = column;
        } else {
            runs.push((start, end));
            start = column;
            end = column;
        }
    }
    runs.push((start, end));
    runs
}

fn player_spheres(game: &Game) -> Vec<Sphere> {
    let x = column_x(game.player_column);
    let (lane_y, z) = game.lane_position(game.player_lane);
    let white = if game.game_over {
        Color::new(190, 118, 118)
    } else {
        Color::new(238, 242, 246)
    };

    vec![
        Sphere::new(Vec3::new(x, lane_y + 0.47, z), 0.66, white),
        Sphere::new(Vec3::new(x, lane_y + 0.91, z), 0.50, white),
        Sphere::new(Vec3::new(x, lane_y + 1.25, z), 0.36, white),
    ]
}

fn column_x(column: usize) -> f32 {
    (column as f32 - (TILE_COLUMNS as f32 - 1.0) * 0.5) * TILE_SPACING
}

fn tint(color: Color, amount: u8) -> Color {
    Color::new(
        color.r.saturating_add(amount),
        color.g.saturating_add(amount),
        color.b.saturating_add(amount),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_contains_every_tile_and_the_player() {
        let game = Game::new();
        let scene = build_scene(&game);
        assert!(scene.cubes.len() >= LANE_COUNT * TILE_COLUMNS);
        assert!(scene.spheres.len() >= 3);
        assert!(!scene.cylinders.is_empty());
    }

    #[test]
    fn adjacent_platform_columns_share_one_visual_log() {
        assert_eq!(
            contiguous_runs(&[0, 1, 3, 5, 6]),
            vec![(0, 1), (3, 3), (5, 6)]
        );
    }
}
