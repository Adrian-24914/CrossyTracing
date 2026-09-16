use crate::{
    color::Color,
    cube::Cube,
    game::{Game, LaneKind, LANE_COUNT, TILE_COLUMNS, TILE_SPACING},
    math::Vec3,
    sphere::Sphere,
};

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub spheres: Vec<Sphere>,
}

pub fn build_scene(game: &Game) -> Scene {
    let mut cubes = Vec::with_capacity(LANE_COUNT * (TILE_COLUMNS + 1));

    for (lane_index, lane) in game.lanes.iter().enumerate() {
        let lane_z = game.lane_z(lane_index);
        let base_color = match lane.kind {
            LaneKind::Grass => Color::new(92, 170, 92),
            LaneKind::Stone => Color::new(107, 123, 138),
        };

        for column in 0..TILE_COLUMNS {
            let color = if column % 2 == 0 {
                base_color
            } else {
                tint(base_color, 10)
            };
            cubes.push(Cube::new(
                Vec3::new(column_x(column), -0.68, lane_z),
                Vec3::new(TILE_SPACING - 0.02, 1.64, TILE_SPACING - 0.02),
                color,
            ));
        }

        if let Some(column) = lane.obstacle_column {
            cubes.push(Cube::new(
                Vec3::new(column_x(column), 0.55, lane_z),
                Vec3::new(0.72, 0.82, 0.72),
                Color::new(217, 106, 67),
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
    let z = game.lane_z(game.player_lane);
    let white = Color::new(238, 242, 246);

    vec![
        Sphere::new(Vec3::new(x, 0.47, z), 0.66, white),
        Sphere::new(Vec3::new(x, 0.91, z), 0.50, white),
        Sphere::new(Vec3::new(x, 1.25, z), 0.36, white),
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
