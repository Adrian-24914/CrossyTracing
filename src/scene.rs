use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    game::{Game, LANE_COUNT, TILE_COLUMNS, TILE_SPACING, WORLD_HALF_WIDTH},
    math::Vec3,
    sphere::Sphere,
    train::{add_train, train_center_x},
    world::{Environment, ForestSectionKind, LaneKind, RailwayPhase, RailwayState, SectionKind},
};

pub struct Scene {
    pub cubes: Vec<Cube>,
    pub spheres: Vec<Sphere>,
    pub cylinders: Vec<Cylinder>,
}

const BIG_WALK_SCALE: f32 = 0.40;
const BIG_WALK_CENTER_OFFSET: f32 = 0.16;
const EYE_LATERAL_OFFSET: f32 = 0.415;
const EYE_HALF_THICKNESS: f32 = 0.022;
const PUPIL_OUTWARD_OFFSET: f32 = 0.035;
const PUPIL_HALF_THICKNESS: f32 = 0.015;

pub struct CharacterPalette {
    pub head: Color,
    pub beak: Color,
    pub eye_white: Color,
    pub pupil: Color,
    pub torso: Color,
    pub hips: Color,
    pub arms: Color,
    pub hands: Color,
    pub legs: Color,
    pub feet: Color,
}

impl CharacterPalette {
    fn big_walk() -> Self {
        Self {
            head: Color::new(181, 70, 60),
            beak: Color::new(170, 61, 51),
            eye_white: Color::new(240, 240, 230),
            pupil: Color::new(12, 12, 12),
            torso: Color::new(218, 166, 48),
            hips: Color::new(39, 44, 91),
            arms: Color::new(218, 166, 48),
            hands: Color::new(218, 166, 48),
            legs: Color::new(31, 36, 76),
            feet: Color::new(31, 36, 76),
        }
    }
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
                        let center_x =
                            train_center_x(progress, railway.direction, WORLD_HALF_WIDTH);
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

    let mut scene = Scene {
        cubes,
        spheres,
        cylinders,
    };
    add_player(&mut scene, game);
    scene
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

fn add_player(scene: &mut Scene, game: &Game) {
    let x = column_x(game.player_column);
    let (lane_y, lane_z) = game.lane_position(game.player_lane);
    let lane = &game.lanes[game.player_lane];
    let surface_height = match lane.environment {
        Environment::Forest => 0.14,
        Environment::River if lane.supports_player(game.player_column) => 0.37,
        Environment::River => -0.11,
        Environment::Railway => 0.30,
    };
    let facing = Vec3::new(0.0, 0.0, -1.0);
    let tile_center = Vec3::new(x, lane_y + surface_height, lane_z);
    let base = tile_center - facing * scaled(BIG_WALK_CENTER_OFFSET);
    let palette = CharacterPalette::big_walk();

    add_big_walk_character(scene, base, facing, &palette);
}

fn transform_local(base: Vec3, local: Vec3, facing: Vec3) -> Vec3 {
    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    let facing = facing.normalize();
    let right = facing.cross(&UP).normalize();
    base + right * local.x + UP * local.y + facing * local.z
}

fn transform_character_local(base: Vec3, local: Vec3, facing: Vec3) -> Vec3 {
    transform_local(base, local * BIG_WALK_SCALE, facing)
}

fn scaled(value: f32) -> f32 {
    value * BIG_WALK_SCALE
}

fn add_big_walk_character(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_left_leg(scene, base, facing, palette);
    add_right_leg(scene, base, facing, palette);
    add_left_foot(scene, base, facing, palette);
    add_right_foot(scene, base, facing, palette);
    add_hips(scene, base, facing, palette);
    add_torso(scene, base, facing, palette);
    add_left_arm(scene, base, facing, palette);
    add_right_arm(scene, base, facing, palette);
    add_left_hand(scene, base, facing, palette);
    add_right_hand(scene, base, facing, palette);
    add_head(scene, base, facing, palette);
    add_left_eye(scene, base, facing, palette);
    add_right_eye(scene, base, facing, palette);
    add_left_pupil(scene, base, facing, palette);
    add_right_pupil(scene, base, facing, palette);
    add_beak(scene, base, facing, palette);
}

fn add_head(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    scene.spheres.push(Sphere::new(
        transform_character_local(base, Vec3::new(0.0, 3.41, 0.0), facing),
        scaled(0.87),
        palette.head,
    ));
}

fn add_beak(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    let start = transform_character_local(base, Vec3::new(0.0, 3.41, 0.34), facing);
    let middle = transform_character_local(base, Vec3::new(0.0, 3.41, 0.56), facing);
    let end = transform_character_local(base, Vec3::new(0.0, 3.41, 0.75), facing);
    let diameter = scaled(0.23);
    scene
        .cylinders
        .push(Cylinder::new_between(start, middle, diameter, palette.beak));
    scene
        .cylinders
        .push(Cylinder::new_between(middle, end, diameter, palette.beak));
    scene.spheres.push(Sphere::new(end, diameter, palette.beak));
}

fn eye_center(base: Vec3, facing: Vec3, side: f32) -> Vec3 {
    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    let head_center = transform_character_local(base, Vec3::new(0.0, 3.41, 0.0), facing);
    let right = facing.normalize().cross(&UP).normalize();
    head_center + right * scaled(EYE_LATERAL_OFFSET * side)
}

fn add_eye(scene: &mut Scene, base: Vec3, facing: Vec3, side: f32, palette: &CharacterPalette) {
    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    let right = facing.normalize().cross(&UP).normalize();
    let center = eye_center(base, facing, side);
    scene.cylinders.push(Cylinder::new_between(
        center - right * scaled(EYE_HALF_THICKNESS),
        center + right * scaled(EYE_HALF_THICKNESS),
        scaled(0.43),
        palette.eye_white,
    ));
}

fn add_right_eye(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_eye(scene, base, facing, 1.0, palette);
}

fn add_left_eye(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_eye(scene, base, facing, -1.0, palette);
}

fn add_pupil(scene: &mut Scene, base: Vec3, facing: Vec3, side: f32, palette: &CharacterPalette) {
    const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    let right = facing.normalize().cross(&UP).normalize();
    let center = eye_center(base, facing, side) + right * scaled(PUPIL_OUTWARD_OFFSET * side);
    scene.cylinders.push(Cylinder::new_between(
        center - right * scaled(PUPIL_HALF_THICKNESS),
        center + right * scaled(PUPIL_HALF_THICKNESS),
        scaled(0.26),
        palette.pupil,
    ));
}

fn add_right_pupil(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_pupil(scene, base, facing, 1.0, palette);
}

fn add_left_pupil(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_pupil(scene, base, facing, -1.0, palette);
}

fn add_torso(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    scene.spheres.push(Sphere::new(
        transform_character_local(base, Vec3::new(0.0, 2.66, 0.0), facing),
        scaled(0.62),
        palette.torso,
    ));
}

fn add_hips(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    scene.spheres.push(Sphere::new(
        transform_character_local(base, Vec3::new(0.0, 1.76, 0.0), facing),
        scaled(1.20),
        palette.hips,
    ));
}

fn add_arm(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    shoulder: Vec3,
    elbow: Vec3,
    wrist: Vec3,
    palette: &CharacterPalette,
) {
    let shoulder = transform_character_local(base, shoulder, facing);
    let elbow = transform_character_local(base, elbow, facing);
    let wrist = transform_character_local(base, wrist, facing);
    let joint_diameter = scaled(0.15);
    scene.cylinders.push(Cylinder::new_between(
        shoulder,
        elbow,
        joint_diameter,
        palette.arms,
    ));
    scene.cylinders.push(Cylinder::new_between(
        elbow,
        wrist,
        joint_diameter,
        palette.arms,
    ));
    scene
        .spheres
        .push(Sphere::new(shoulder, joint_diameter, palette.arms));
    scene
        .spheres
        .push(Sphere::new(elbow, joint_diameter, palette.arms));
}

fn add_right_arm(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_arm(
        scene,
        base,
        facing,
        Vec3::new(0.30, 2.70, 0.0),
        Vec3::new(0.62, 2.35, 0.0),
        Vec3::new(0.92, 1.78, 0.0),
        palette,
    );
}

fn add_left_arm(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_arm(
        scene,
        base,
        facing,
        Vec3::new(-0.30, 2.70, 0.0),
        Vec3::new(-0.62, 2.35, 0.0),
        Vec3::new(-0.92, 1.95, 0.0),
        palette,
    );
}

fn add_right_hand(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    scene.spheres.push(Sphere::new(
        transform_character_local(base, Vec3::new(0.92, 1.78, 0.0), facing),
        scaled(0.25),
        palette.hands,
    ));
}

fn add_left_hand(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    scene.spheres.push(Sphere::new(
        transform_character_local(base, Vec3::new(-0.92, 1.95, 0.0), facing),
        scaled(0.25),
        palette.hands,
    ));
}

fn add_leg(scene: &mut Scene, base: Vec3, facing: Vec3, side: f32, palette: &CharacterPalette) {
    let hip = transform_character_local(base, Vec3::new(0.22 * side, 1.25, 0.0), facing);
    let knee = transform_character_local(base, Vec3::new(0.22 * side, 0.735, 0.0), facing);
    let ankle = transform_character_local(base, Vec3::new(0.22 * side, 0.22, 0.0), facing);
    let joint_diameter = scaled(0.16);
    scene.cylinders.push(Cylinder::new_between(
        hip,
        knee,
        joint_diameter,
        palette.legs,
    ));
    scene.cylinders.push(Cylinder::new_between(
        knee,
        ankle,
        joint_diameter,
        palette.legs,
    ));
    scene
        .spheres
        .push(Sphere::new(knee, joint_diameter, palette.legs));
}

fn add_right_leg(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_leg(scene, base, facing, 1.0, palette);
}

fn add_left_leg(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_leg(scene, base, facing, -1.0, palette);
}

fn add_foot(scene: &mut Scene, base: Vec3, facing: Vec3, side: f32, palette: &CharacterPalette) {
    let center = transform_character_local(base, Vec3::new(0.22 * side, 0.12, 0.16), facing);
    let facing = facing.normalize();
    let size = if facing.x.abs() > facing.z.abs() {
        Vec3::new(0.42, 0.18, 0.28)
    } else {
        Vec3::new(0.28, 0.18, 0.42)
    } * BIG_WALK_SCALE;
    scene.cubes.push(Cube::new(center, size, palette.feet));
    scene.spheres.push(Sphere::new(
        center + facing * scaled(0.18),
        scaled(0.28),
        palette.feet,
    ));
}

fn add_right_foot(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_foot(scene, base, facing, 1.0, palette);
}

fn add_left_foot(scene: &mut Scene, base: Vec3, facing: Vec3, palette: &CharacterPalette) {
    add_foot(scene, base, facing, -1.0, palette);
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
        assert!(scene.spheres.len() >= 7);
        assert!(!scene.cylinders.is_empty());
    }

    #[test]
    fn big_walk_character_has_all_independent_parts() {
        let mut scene = Scene {
            cubes: Vec::new(),
            spheres: Vec::new(),
            cylinders: Vec::new(),
        };
        add_big_walk_character(
            &mut scene,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            &CharacterPalette::big_walk(),
        );

        assert_eq!(scene.cubes.len(), 2);
        assert_eq!(scene.spheres.len(), 14);
        assert_eq!(scene.cylinders.len(), 14);
    }

    #[test]
    fn leg_segments_are_equal_and_share_a_rounded_knee() {
        let mut scene = Scene {
            cubes: Vec::new(),
            spheres: Vec::new(),
            cylinders: Vec::new(),
        };
        add_leg(
            &mut scene,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            1.0,
            &CharacterPalette::big_walk(),
        );

        assert_eq!(scene.cylinders.len(), 2);
        assert_eq!(scene.spheres.len(), 1);
        assert!((scene.cylinders[0].half_length - scene.cylinders[1].half_length).abs() < 0.0001);
        assert!((scene.cylinders[0].radius - scene.spheres[0].radius).abs() < 0.0001);
    }

    #[test]
    fn local_front_follows_the_character_facing() {
        let point = transform_local(
            Vec3::new(2.0, 1.0, 3.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, -1.0),
        );

        assert!((point.x - 2.0).abs() < 0.0001);
        assert!((point.y - 1.0).abs() < 0.0001);
        assert!((point.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn eye_discs_protrude_from_the_head_and_overlap() {
        let head_radius = 0.87 * 0.5;
        let eye_outer_face = EYE_LATERAL_OFFSET + EYE_HALF_THICKNESS;
        let pupil_inner_face = EYE_LATERAL_OFFSET + PUPIL_OUTWARD_OFFSET - PUPIL_HALF_THICKNESS;

        assert!(eye_outer_face > head_radius);
        assert!(pupil_inner_face <= eye_outer_face);
    }

    #[test]
    fn adjacent_platform_columns_share_one_visual_log() {
        assert_eq!(
            contiguous_runs(&[0, 1, 3, 5, 6]),
            vec![(0, 1), (3, 3), (5, 6)]
        );
    }
}
