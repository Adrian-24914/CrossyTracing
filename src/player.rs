use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    game::{Game, TILE_COLUMNS, TILE_SPACING},
    material::Material,
    math::Vec3,
    scene::Scene,
    sphere::Sphere,
    world::Environment,
};

const BIG_WALK_SCALE: f32 = 0.40;
const BIG_WALK_CENTER_OFFSET: f32 = 0.16;
const EYE_LATERAL_OFFSET: f32 = 0.415;
const EYE_HALF_THICKNESS: f32 = 0.022;
const PUPIL_OUTWARD_OFFSET: f32 = 0.035;
const PUPIL_HALF_THICKNESS: f32 = 0.015;

#[derive(Clone, Copy)]
enum CharacterPose {
    Idle,
    Jumping,
}

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
            eye_white: Color::new(255, 255, 255),
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

pub fn add_player(scene: &mut Scene, game: &Game) {
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
    let (jump_x, jump_y, jump_z) = game.player_jump_offset();
    let tile_center = Vec3::new(
        x + jump_x,
        lane_y + surface_height + jump_y,
        lane_z + jump_z,
    );
    let base = tile_center - facing * scaled(BIG_WALK_CENTER_OFFSET);
    let palette = CharacterPalette::big_walk();
    let pose = if game.player_is_jumping() {
        CharacterPose::Jumping
    } else {
        CharacterPose::Idle
    };

    add_contact_shadow(
        scene,
        Vec3::new(x + jump_x, lane_y + surface_height + 0.012, lane_z + jump_z),
    );
    add_big_walk_character(scene, base, facing, &palette, pose);
}

fn add_contact_shadow(scene: &mut Scene, center: Vec3) {
    scene.cylinders.push(Cylinder::new_between_with_material(
        center - Vec3::new(0.0, 0.012, 0.0),
        center + Vec3::new(0.0, 0.012, 0.0),
        scaled(1.85),
        Material::translucent_matte(Color::new(18, 28, 21), 0.48),
    ));
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

fn add_big_walk_character(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_left_leg(scene, base, facing, palette, pose);
    add_right_leg(scene, base, facing, palette, pose);
    add_left_foot(scene, base, facing, palette, pose);
    add_right_foot(scene, base, facing, palette, pose);
    add_hips(scene, base, facing, palette);
    add_torso(scene, base, facing, palette);
    add_left_arm(scene, base, facing, palette, pose);
    add_right_arm(scene, base, facing, palette, pose);
    add_left_hand(scene, base, facing, palette, pose);
    add_right_hand(scene, base, facing, palette, pose);
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
    scene.cylinders.push(Cylinder::new_between_with_material(
        center - right * scaled(EYE_HALF_THICKNESS),
        center + right * scaled(EYE_HALF_THICKNESS),
        scaled(0.43),
        Material::unlit(palette.eye_white),
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
    scene.cylinders.push(Cylinder::new_between_with_material(
        center - right * scaled(PUPIL_HALF_THICKNESS),
        center + right * scaled(PUPIL_HALF_THICKNESS),
        scaled(0.26),
        Material::glossy(palette.pupil, 0.55, 36.0),
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

fn arm_points(side: f32, pose: CharacterPose) -> (Vec3, Vec3, Vec3) {
    let shoulder = Vec3::new(0.30 * side, 2.70, 0.0);
    match pose {
        CharacterPose::Idle => {
            let wrist_y = if side > 0.0 { 1.78 } else { 1.95 };
            (
                shoulder,
                Vec3::new(0.62 * side, 2.35, 0.0),
                Vec3::new(0.92 * side, wrist_y, 0.0),
            )
        }
        CharacterPose::Jumping => (
            shoulder,
            Vec3::new(0.58 * side, 2.96, 0.04),
            Vec3::new(0.84 * side, 3.18, 0.10),
        ),
    }
}

fn add_arm_for_side(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    side: f32,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    let (shoulder, elbow, wrist) = arm_points(side, pose);
    add_arm(scene, base, facing, shoulder, elbow, wrist, palette);
}

fn add_right_arm(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_arm_for_side(scene, base, facing, 1.0, palette, pose);
}

fn add_left_arm(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_arm_for_side(scene, base, facing, -1.0, palette, pose);
}

fn add_hand(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    side: f32,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    let (_, _, wrist) = arm_points(side, pose);
    scene.spheres.push(Sphere::new(
        transform_character_local(base, wrist, facing),
        scaled(0.25),
        palette.hands,
    ));
}

fn add_right_hand(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_hand(scene, base, facing, 1.0, palette, pose);
}

fn add_left_hand(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_hand(scene, base, facing, -1.0, palette, pose);
}

fn leg_points(side: f32, pose: CharacterPose) -> (Vec3, Vec3, Vec3) {
    let hip = Vec3::new(0.22 * side, 1.25, 0.0);
    match pose {
        CharacterPose::Idle => (
            hip,
            Vec3::new(0.22 * side, 0.735, 0.0),
            Vec3::new(0.22 * side, 0.22, 0.0),
        ),
        CharacterPose::Jumping => (
            hip,
            Vec3::new(0.42 * side, 0.92, 0.18),
            Vec3::new(0.30 * side, 0.62, -0.12),
        ),
    }
}

fn add_leg(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    side: f32,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    let (hip, knee, ankle) = leg_points(side, pose);
    let hip = transform_character_local(base, hip, facing);
    let knee = transform_character_local(base, knee, facing);
    let ankle = transform_character_local(base, ankle, facing);
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

fn add_right_leg(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_leg(scene, base, facing, 1.0, palette, pose);
}

fn add_left_leg(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_leg(scene, base, facing, -1.0, palette, pose);
}

fn foot_center(side: f32, pose: CharacterPose) -> Vec3 {
    match pose {
        CharacterPose::Idle => Vec3::new(0.22 * side, 0.12, 0.16),
        CharacterPose::Jumping => Vec3::new(0.30 * side, 0.52, 0.04),
    }
}

fn add_foot(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    side: f32,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    let center = transform_character_local(base, foot_center(side, pose), facing);
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

fn add_right_foot(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_foot(scene, base, facing, 1.0, palette, pose);
}

fn add_left_foot(
    scene: &mut Scene,
    base: Vec3,
    facing: Vec3,
    palette: &CharacterPalette,
    pose: CharacterPose,
) {
    add_foot(scene, base, facing, -1.0, palette, pose);
}

fn column_x(column: usize) -> f32 {
    (column as f32 - (TILE_COLUMNS as f32 - 1.0) * 0.5) * TILE_SPACING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Finish;

    fn empty_scene() -> Scene {
        Scene {
            cubes: Vec::new(),
            spheres: Vec::new(),
            cylinders: Vec::new(),
            forest_props: Vec::new(),
        }
    }

    #[test]
    fn big_walk_character_has_all_independent_parts() {
        let mut scene = empty_scene();
        add_big_walk_character(
            &mut scene,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            &CharacterPalette::big_walk(),
            CharacterPose::Idle,
        );

        assert_eq!(scene.cubes.len(), 2);
        assert_eq!(scene.spheres.len(), 14);
        assert_eq!(scene.cylinders.len(), 14);
    }

    #[test]
    fn eye_whites_are_unlit_while_the_body_stays_matte() {
        let mut scene = empty_scene();
        add_big_walk_character(
            &mut scene,
            Vec3::default(),
            Vec3::new(0.0, 0.0, -1.0),
            &CharacterPalette::big_walk(),
            CharacterPose::Idle,
        );

        let unlit: Vec<_> = scene
            .cylinders
            .iter()
            .filter(|part| part.material.finish == Finish::Unlit)
            .collect();
        let glossy_count = scene
            .cylinders
            .iter()
            .filter(|part| part.material.finish == Finish::Glossy)
            .count();
        assert_eq!(unlit.len(), 2);
        assert!(unlit
            .iter()
            .all(|part| part.material.albedo.to_hex() == 0xFFFFFF));
        assert_eq!(glossy_count, 2);
        assert!(scene
            .cubes
            .iter()
            .all(|part| part.material.finish == Finish::Matte));
        assert!(scene
            .spheres
            .iter()
            .all(|part| part.material.finish == Finish::Matte));
    }

    #[test]
    fn leg_segments_are_equal_and_share_a_rounded_knee() {
        let mut scene = empty_scene();
        add_leg(
            &mut scene,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            1.0,
            &CharacterPalette::big_walk(),
            CharacterPose::Idle,
        );

        assert_eq!(scene.cylinders.len(), 2);
        assert_eq!(scene.spheres.len(), 1);
        assert!((scene.cylinders[0].half_length - scene.cylinders[1].half_length).abs() < 0.0001);
        assert!((scene.cylinders[0].radius - scene.spheres[0].radius).abs() < 0.0001);
    }

    #[test]
    fn jumping_pose_raises_the_hands_and_tucks_the_legs() {
        let (_, _, idle_wrist) = arm_points(1.0, CharacterPose::Idle);
        let (_, _, jumping_wrist) = arm_points(1.0, CharacterPose::Jumping);
        let (_, _, idle_ankle) = leg_points(1.0, CharacterPose::Idle);
        let (_, _, jumping_ankle) = leg_points(1.0, CharacterPose::Jumping);
        let idle_foot = foot_center(1.0, CharacterPose::Idle);
        let jumping_foot = foot_center(1.0, CharacterPose::Jumping);

        assert!(jumping_wrist.y > idle_wrist.y);
        assert!(jumping_ankle.y > idle_ankle.y);
        assert!(jumping_foot.y > idle_foot.y);
    }

    #[test]
    fn contact_shadow_is_a_thin_translucent_disc() {
        let mut scene = empty_scene();
        add_contact_shadow(&mut scene, Vec3::new(1.0, 0.2, -2.0));

        assert_eq!(scene.cylinders.len(), 1);
        let shadow = &scene.cylinders[0];
        assert!(shadow.axis.y > 0.99);
        assert!(shadow.half_length < shadow.radius * 0.05);
        assert!((shadow.material.transparency - 0.48).abs() < 0.0001);
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
}
