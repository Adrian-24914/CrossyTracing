use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    game::{Game, LANE_COUNT, TILE_COLUMNS, TILE_SPACING, WORLD_HALF_WIDTH},
    material::Material,
    math::Vec3,
    obstacle::ForestProp,
    player::add_player,
    sphere::Sphere,
    train::{add_train, train_center_x},
    world::{
        Environment, ForestSectionKind, GrassBlade, LaneKind, RailwayPhase, RailwayState,
        SectionKind,
    },
};

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub spheres: Vec<Sphere>,
    pub cylinders: Vec<Cylinder>,
    pub forest_props: Vec<ForestProp>,
}

pub fn build_scene(game: &Game) -> Scene {
    let mut cubes = Vec::with_capacity(LANE_COUNT * (TILE_COLUMNS + 3));
    let mut spheres = Vec::new();
    let mut cylinders = Vec::new();
    let mut forest_props = Vec::new();
    add_river_surfaces(&mut cubes, game);

    for (lane_index, lane) in game.lanes.iter().enumerate() {
        let (lane_y, lane_z) = game.lane_position(lane_index);
        let base_color = match (lane.section_kind, lane.kind) {
            (SectionKind::Forest(ForestSectionKind::Clearing), LaneKind::Grass) => {
                Color::new(92, 151, 76)
            }
            (SectionKind::Forest(ForestSectionKind::Grove), LaneKind::Grass) => {
                Color::new(64, 126, 70)
            }
            (SectionKind::Forest(ForestSectionKind::Thicket), LaneKind::Grass) => {
                Color::new(48, 103, 61)
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

        if lane.environment != Environment::River {
            for column in 0..TILE_COLUMNS {
                let color = if lane.kind == LaneKind::Grass {
                    grass_tile_color(base_color, lane.grass_tone_shifts[column])
                } else {
                    base_color
                };
                cubes.push(Cube::with_material(
                    Vec3::new(column_x(column), tile_y, lane_z),
                    Vec3::new(TILE_SPACING - 0.02, tile_height, TILE_SPACING - 0.02),
                    Material::matte(color),
                ));
                if lane.kind == LaneKind::Grass {
                    add_grass_blades(
                        &mut cubes,
                        column_x(column),
                        tile_y + tile_height * 0.5,
                        lane_z,
                        color,
                        &lane.grass_blades[column],
                    );
                }
            }
        }

        match lane.environment {
            Environment::Forest => {
                for obstacle in &lane.obstacles {
                    let prop = ForestProp::new(
                        obstacle.kind,
                        Vec3::new(column_x(obstacle.column), lane_y + 0.14, lane_z),
                        obstacle_color,
                    );
                    debug_assert!(prop.foliage_anchors().is_none_or(|anchors| {
                        anchors
                            .iter()
                            .all(|anchor| anchor.position.y > lane_y && anchor.suggested_size > 0.0)
                    }));
                    forest_props.push(prop);
                }
            }
            Environment::River => {
                for (start, end) in contiguous_runs(&lane.platform_columns) {
                    let start_x = column_x(start);
                    let end_x = column_x(end);
                    cylinders.push(Cylinder::new_x_with_material(
                        Vec3::new((start_x + end_x) * 0.5, lane_y + 0.13, lane_z),
                        end_x - start_x + TILE_SPACING * 0.82,
                        0.48,
                        Material::matte(Color::new(137, 84, 48)),
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
                    cylinders.push(Cylinder::new_x_with_material(
                        Vec3::new(0.0, lane_y + 0.24, lane_z + z_offset),
                        TILE_COLUMNS as f32 * TILE_SPACING,
                        0.12,
                        Material::glossy(Color::new(151, 157, 157), 0.32, 24.0),
                    ));
                }

                if let Some(railway) = &lane.railway {
                    add_railway_signals(&mut cubes, &mut spheres, lane_y, lane_z, railway);
                    if railway.phase == RailwayPhase::Crossing {
                        let progress =
                            (railway.elapsed / crate::game::TRAIN_CROSSING_SECONDS).clamp(0.0, 1.0);
                        let center_x =
                            train_center_x(progress, railway.direction, WORLD_HALF_WIDTH);
                        add_train(
                            &mut cubes,
                            &mut cylinders,
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

    let mut scene = Scene {
        cubes,
        spheres,
        cylinders,
        forest_props,
    };
    add_player(&mut scene, game);
    scene
}

fn add_river_surfaces(cubes: &mut Vec<Cube>, game: &Game) {
    let mut start = 0;
    while start < game.lanes.len() {
        if game.lanes[start].environment != Environment::River {
            start += 1;
            continue;
        }

        let (surface_y, start_z) = game.lane_position(start);
        let mut end = start;
        while end + 1 < game.lanes.len() && game.lanes[end + 1].environment == Environment::River {
            let (next_y, _) = game.lane_position(end + 1);
            if (next_y - surface_y).abs() > 0.0001 {
                break;
            }
            end += 1;
        }

        let (_, end_z) = game.lane_position(end);
        cubes.push(Cube::with_material(
            Vec3::new(0.0, surface_y - 0.80, (start_z + end_z) * 0.5),
            Vec3::new(
                TILE_COLUMNS as f32 * TILE_SPACING - 0.02,
                1.38,
                end_z - start_z + TILE_SPACING - 0.02,
            ),
            Material::translucent_matte(Color::new(52, 129, 164), 0.42),
        ));
        start = end + 1;
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
        let material = if warning_on {
            Material::unlit(light_color)
        } else {
            Material::matte(light_color)
        };
        spheres.push(Sphere::with_material(
            Vec3::new(x, lane_y + 0.93, lane_z + 0.49),
            0.24,
            material,
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

fn column_x(column: usize) -> f32 {
    (column as f32 - (TILE_COLUMNS as f32 - 1.0) * 0.5) * TILE_SPACING
}

fn darken(color: Color, amount: u8) -> Color {
    Color::new(
        color.r.saturating_sub(amount),
        color.g.saturating_sub(amount),
        color.b.saturating_sub(amount),
    )
}

fn grass_tile_color(base: Color, tone_shift: u8) -> Color {
    darken(base, tone_shift)
}

fn add_grass_blades(
    cubes: &mut Vec<Cube>,
    tile_x: f32,
    ground_top: f32,
    tile_z: f32,
    tile_color: Color,
    blades: &[GrassBlade],
) {
    for (index, blade) in blades.iter().enumerate() {
        let color = darken(tile_color, 4 + index as u8);
        cubes.push(Cube::new(
            Vec3::new(
                tile_x + blade.local_x,
                ground_top + blade.height * 0.5,
                tile_z + blade.local_z,
            ),
            Vec3::new(blade.width, blade.height, blade.width),
            color,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_contains_every_ground_row_and_the_player() {
        let game = Game::new();
        let scene = build_scene(&game);
        let expected_forest_props: usize = game.lanes.iter().map(|lane| lane.obstacles.len()).sum();
        let expected_ground_cubes: usize = game
            .lanes
            .iter()
            .map(|lane| {
                if lane.environment == Environment::River {
                    1
                } else {
                    TILE_COLUMNS
                }
            })
            .sum();
        assert!(scene.cubes.len() >= expected_ground_cubes);
        assert!(scene.spheres.len() >= 7);
        assert!(!scene.cylinders.is_empty());
        assert_eq!(scene.forest_props.len(), expected_forest_props);
    }

    #[test]
    fn adjacent_platform_columns_share_one_visual_log() {
        assert_eq!(
            contiguous_runs(&[0, 1, 3, 5, 6]),
            vec![(0, 1), (3, 3), (5, 6)]
        );
    }

    #[test]
    fn river_tiles_use_translucent_material() {
        let mut game = Game::new();
        game.lanes[0].environment = Environment::River;
        game.lanes[0].section_kind = SectionKind::River;
        game.lanes[0].kind = LaneKind::Water;
        game.lanes[0].obstacles.clear();
        let scene = build_scene(&game);

        let river = &scene.cubes[0];
        assert!((river.material.transparency - 0.42).abs() < 0.0001);
        assert!(
            ((river.max.x - river.min.x) - (TILE_COLUMNS as f32 * TILE_SPACING - 0.02)).abs()
                < 0.0001
        );
    }

    #[test]
    fn river_logs_use_matte_wood_material() {
        let mut game = Game::new();
        game.lanes[0].environment = Environment::River;
        game.lanes[0].section_kind = SectionKind::River;
        game.lanes[0].kind = LaneKind::Water;
        game.lanes[0].obstacles.clear();
        game.lanes[0].platform_columns = vec![0, 1, 3];
        let scene = build_scene(&game);

        assert!(
            scene
                .cylinders
                .iter()
                .filter(|cylinder| {
                    cylinder.material.albedo.to_hex() == 0x895430
                        && cylinder.material.finish == crate::material::Finish::Matte
                })
                .count()
                >= 2
        );
    }

    #[test]
    fn grass_and_stone_tiles_keep_flat_matte_materials() {
        let mut game = Game::new();
        for lane in &mut game.lanes {
            lane.environment = Environment::Forest;
            lane.section_kind = SectionKind::Forest(ForestSectionKind::Clearing);
            lane.kind = LaneKind::Stone;
            lane.obstacles.clear();
            lane.platform_columns.clear();
            lane.railway = None;
        }
        game.lanes[0].kind = LaneKind::Grass;

        let scene = build_scene(&game);
        let ground_tiles = scene
            .cubes
            .iter()
            .filter(|cube| {
                cube.material.finish == crate::material::Finish::Matte
                    && cube.max.y - cube.min.y > 1.0
            })
            .count();

        assert_eq!(ground_tiles, LANE_COUNT * TILE_COLUMNS);
    }

    #[test]
    fn grass_tiles_use_three_or_four_vertical_decorative_blades() {
        let mut cubes = Vec::new();
        let blades = vec![
            GrassBlade {
                local_x: -0.22,
                local_z: 0.17,
                width: 0.04,
                height: 0.14,
            },
            GrassBlade {
                local_x: 0.18,
                local_z: -0.25,
                width: 0.05,
                height: 0.22,
            },
            GrassBlade {
                local_x: 0.04,
                local_z: 0.31,
                width: 0.06,
                height: 0.18,
            },
        ];
        add_grass_blades(&mut cubes, 0.0, 0.14, 0.0, Color::new(64, 126, 70), &blades);
        let mut rebuilt = Vec::new();
        add_grass_blades(
            &mut rebuilt,
            0.0,
            0.14,
            0.0,
            Color::new(64, 126, 70),
            &blades,
        );

        assert!((3..=4).contains(&cubes.len()));
        assert!(cubes.iter().all(|blade| {
            let size = blade.max - blade.min;
            size.y <= 0.24 && size.y >= 0.10 && (size.x - size.z).abs() < 0.0001 && size.y > size.x
        }));
        assert!(((cubes[0].min.x + cubes[0].max.x) * 0.5 + 0.22).abs() < 0.0001);
        assert!((cubes[1].min.z - rebuilt[1].min.z).abs() < 0.0001);
    }

    #[test]
    fn adjacent_river_rows_share_one_continuous_surface() {
        let mut game = Game::new();
        for lane in &mut game.lanes {
            lane.environment = Environment::Forest;
            lane.section_kind = SectionKind::Forest(ForestSectionKind::Clearing);
            lane.kind = LaneKind::Grass;
            lane.obstacles.clear();
            lane.platform_columns.clear();
            lane.railway = None;
        }
        for lane in &mut game.lanes[..2] {
            lane.environment = Environment::River;
            lane.section_kind = SectionKind::River;
            lane.kind = LaneKind::Water;
        }

        let scene = build_scene(&game);
        let water: Vec<_> = scene
            .cubes
            .iter()
            .filter(|cube| cube.material.transparency > 0.0)
            .collect();

        assert_eq!(water.len(), 1);
        assert!(((water[0].max.z - water[0].min.z) - (TILE_SPACING * 2.0 - 0.02)).abs() < 0.0001);
    }
}
