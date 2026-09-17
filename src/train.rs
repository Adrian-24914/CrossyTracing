use crate::{color::Color, cube::Cube, math::Vec3, world::TrainDirection};

pub const TRAIN_LENGTH: f32 = 4.2;

pub fn train_center_x(progress: f32, direction: TrainDirection, world_half_width: f32) -> f32 {
    let outside_center = world_half_width + TRAIN_LENGTH * 0.5;
    direction.sign() * (-outside_center + 2.0 * outside_center * progress.clamp(0.0, 1.0))
}

pub fn train_x_bounds(
    progress: f32,
    direction: TrainDirection,
    world_half_width: f32,
) -> Option<(f32, f32)> {
    let center_x = train_center_x(progress, direction, world_half_width);
    let minimum_x = (center_x - TRAIN_LENGTH * 0.5).max(-world_half_width);
    let maximum_x = (center_x + TRAIN_LENGTH * 0.5).min(world_half_width);
    (maximum_x > minimum_x).then_some((minimum_x, maximum_x))
}

pub fn add_train(
    cubes: &mut Vec<Cube>,
    center_x: f32,
    lane_y: f32,
    lane_z: f32,
    direction: TrainDirection,
    world_half_width: f32,
) {
    let facing = direction.sign();
    let mut add_part = |local_x: f32, y: f32, z: f32, size: Vec3, color: Color| {
        add_clipped_cube(
            cubes,
            Vec3::new(center_x + local_x * facing, lane_y + y, lane_z + z),
            size,
            color,
            world_half_width,
        );
    };

    let red = Color::new(174, 53, 45);
    let dark_red = Color::new(112, 42, 39);
    let gold = Color::new(220, 164, 60);
    let charcoal = Color::new(47, 50, 48);
    let window = Color::new(142, 205, 211);

    add_part(
        0.0,
        0.42,
        0.0,
        Vec3::new(TRAIN_LENGTH, 0.22, 0.88),
        charcoal,
    );
    add_part(0.82, 0.72, 0.0, Vec3::new(1.90, 0.64, 0.72), red);
    add_part(-0.22, 0.92, 0.0, Vec3::new(0.94, 1.02, 0.78), dark_red);
    add_part(-0.22, 1.48, 0.0, Vec3::new(1.18, 0.14, 0.94), gold);
    add_part(-0.22, 1.02, 0.405, Vec3::new(0.48, 0.40, 0.05), window);
    add_part(-0.22, 1.02, -0.405, Vec3::new(0.48, 0.40, 0.05), window);
    add_part(1.32, 1.25, 0.0, Vec3::new(0.26, 0.72, 0.30), charcoal);
    add_part(1.32, 1.64, 0.0, Vec3::new(0.48, 0.12, 0.46), charcoal);
    add_part(1.98, 0.54, 0.0, Vec3::new(0.24, 0.20, 1.00), gold);

    add_part(-1.43, 0.88, 0.0, Vec3::new(1.18, 0.78, 0.78), red);
    add_part(-1.43, 1.31, 0.0, Vec3::new(1.34, 0.12, 0.92), gold);

    for wheel_x in [-1.72, -1.13, -0.42, 0.42, 1.14] {
        for wheel_z in [-0.43, 0.43] {
            add_part(
                wheel_x,
                0.30,
                wheel_z,
                Vec3::new(0.34, 0.42, 0.12),
                charcoal,
            );
        }
    }

    add_part(1.86, 0.84, 0.0, Vec3::new(0.12, 0.16, 0.20), gold);
}

fn add_clipped_cube(
    cubes: &mut Vec<Cube>,
    center: Vec3,
    size: Vec3,
    color: Color,
    world_half_width: f32,
) {
    let minimum_x = (center.x - size.x * 0.5).max(-world_half_width);
    let maximum_x = (center.x + size.x * 0.5).min(world_half_width);
    if maximum_x <= minimum_x {
        return;
    }

    cubes.push(Cube::new(
        Vec3::new((minimum_x + maximum_x) * 0.5, center.y, center.z),
        Vec3::new(maximum_x - minimum_x, size.y, size.z),
        color,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORLD_EDGE: f32 = 4.725;

    #[test]
    fn train_geometry_is_always_clipped_to_the_playable_world() {
        for center_x in [-6.825, -4.0, 0.0, 4.0, 6.825] {
            let mut cubes = Vec::new();
            add_train(
                &mut cubes,
                center_x,
                0.0,
                0.0,
                TrainDirection::LeftToRight,
                WORLD_EDGE,
            );
            assert!(cubes.iter().all(|cube| {
                cube.min.x >= -WORLD_EDGE - f32::EPSILON && cube.max.x <= WORLD_EDGE + f32::EPSILON
            }));
        }
    }

    #[test]
    fn train_is_hidden_until_it_reaches_an_edge() {
        let mut cubes = Vec::new();
        add_train(
            &mut cubes,
            -WORLD_EDGE - TRAIN_LENGTH * 0.5,
            0.0,
            0.0,
            TrainDirection::LeftToRight,
            WORLD_EDGE,
        );
        assert!(cubes.is_empty());
    }
}
