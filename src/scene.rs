use crate::{
    color::Color,
    cube::Cube,
    game::{Game, LANE_COUNT, TILE_COLUMNS, TILE_SPACING},
    math::Vec3,
    sphere::Sphere,
    world::{Environment, ForestSectionKind, LaneKind},
};

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub spheres: Vec<Sphere>,
}

pub fn build_scene(game: &Game) -> Scene {
    let mut cubes = Vec::with_capacity(LANE_COUNT * (TILE_COLUMNS + 3));

    for (lane_index, lane) in game.lanes.iter().enumerate() {
        let (lane_y, lane_z) = game.lane_position(lane_index);
        let base_color = match (lane.environment, lane.section_kind, lane.kind) {
            (Environment::Forest, ForestSectionKind::Clearing, LaneKind::Grass) => {
                Color::new(111, 177, 91)
            }
            (Environment::Forest, ForestSectionKind::Grove, LaneKind::Grass) => {
                Color::new(78, 145, 82)
            }
            (Environment::Forest, ForestSectionKind::Thicket, LaneKind::Grass) => {
                Color::new(58, 119, 72)
            }
            (Environment::Forest, _, LaneKind::Stone) => Color::new(107, 123, 112),
        };

        let obstacle_color = match lane.section_kind {
            ForestSectionKind::Clearing => Color::new(166, 105, 62),
            ForestSectionKind::Grove => Color::new(132, 82, 52),
            ForestSectionKind::Thicket => Color::new(99, 68, 48),
        };

        for column in 0..TILE_COLUMNS {
            let color = if column % 2 == 0 {
                base_color
            } else {
                tint(base_color, 10)
            };
            cubes.push(Cube::new(
                Vec3::new(column_x(column), lane_y - 0.68, lane_z),
                Vec3::new(TILE_SPACING - 0.02, 1.64, TILE_SPACING - 0.02),
                color,
            ));
        }

        for &column in &lane.obstacle_columns {
            cubes.push(Cube::new(
                Vec3::new(column_x(column), lane_y + 0.55, lane_z),
                Vec3::new(0.72, 0.82, 0.72),
                obstacle_color,
            ));
        }
    }

    Scene {
        cubes,
        spheres: player_spheres(game),
    }
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
        assert_eq!(scene.spheres.len(), 3);
    }
}
