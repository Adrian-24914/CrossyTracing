use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    math::Vec3,
    ray::{Hit, Ray},
    sphere::Sphere,
    tree::{FoliageAnchor, Tree},
    world::ForestObstacleKind,
};

pub enum ForestProp {
    Tree(Tree),
    Rock(CompoundObstacle),
    FallenLog(CompoundObstacle),
    Bush(CompoundObstacle),
}

impl ForestProp {
    pub fn new(kind: ForestObstacleKind, base: Vec3, wood_color: Color) -> Self {
        match kind {
            ForestObstacleKind::Tree => Self::Tree(Tree::new(base, wood_color)),
            ForestObstacleKind::Rock => Self::Rock(rock(base)),
            ForestObstacleKind::FallenLog => Self::FallenLog(fallen_log(base, wood_color)),
            ForestObstacleKind::Bush => Self::Bush(bush(base)),
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<(Hit, Color)> {
        match self {
            Self::Tree(tree) => tree.intersect(ray),
            Self::Rock(model) | Self::FallenLog(model) | Self::Bush(model) => model.intersect(ray),
        }
    }

    pub fn foliage_anchors(&self) -> Option<&[FoliageAnchor; 5]> {
        match self {
            Self::Tree(tree) => Some(tree.foliage_anchors()),
            Self::Rock(_) | Self::FallenLog(_) | Self::Bush(_) => None,
        }
    }
}

pub(crate) struct CompoundObstacle {
    cubes: Vec<Cube>,
    spheres: Vec<Sphere>,
    cylinders: Vec<Cylinder>,
    bounds_center: Vec3,
    bounds_radius: f32,
}

impl CompoundObstacle {
    fn intersect(&self, ray: &Ray) -> Option<(Hit, Color)> {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return None;
        }

        let mut closest: Option<(Hit, Color)> = None;
        for cube in &self.cubes {
            keep_closest(&mut closest, cube.intersect(ray), cube.color);
        }
        for sphere in &self.spheres {
            keep_closest(&mut closest, sphere.intersect(ray), sphere.color);
        }
        for cylinder in &self.cylinders {
            keep_closest(&mut closest, cylinder.intersect(ray), cylinder.color);
        }
        closest
    }
}

fn rock(base: Vec3) -> CompoundObstacle {
    let stone = Color::new(112, 119, 116);
    let dark = shade(stone, -18);
    let light = shade(stone, 16);
    let cubes = vec![
        Cube::new(
            base + Vec3::new(0.0, 0.17, 0.0),
            Vec3::new(0.62, 0.34, 0.55),
            stone,
        ),
        Cube::new(
            base + Vec3::new(-0.24, 0.15, 0.12),
            Vec3::new(0.38, 0.30, 0.36),
            dark,
        ),
        Cube::new(
            base + Vec3::new(0.25, 0.13, -0.11),
            Vec3::new(0.40, 0.26, 0.42),
            dark,
        ),
        Cube::new(
            base + Vec3::new(-0.06, 0.39, 0.01),
            Vec3::new(0.34, 0.28, 0.32),
            light,
        ),
        Cube::new(
            base + Vec3::new(0.18, 0.31, 0.18),
            Vec3::new(0.25, 0.22, 0.26),
            stone,
        ),
    ];
    CompoundObstacle {
        cubes,
        spheres: Vec::new(),
        cylinders: Vec::new(),
        bounds_center: base + Vec3::new(0.0, 0.28, 0.0),
        bounds_radius: 0.68,
    }
}

fn fallen_log(base: Vec3, color: Color) -> CompoundObstacle {
    let cylinders = vec![
        Cylinder::new_between(
            base + Vec3::new(-0.48, 0.20, -0.10),
            base + Vec3::new(0.48, 0.20, 0.10),
            0.34,
            color,
        ),
        Cylinder::new_between(
            base + Vec3::new(0.04, 0.28, 0.01),
            base + Vec3::new(0.31, 0.53, 0.29),
            0.15,
            shade(color, 12),
        ),
    ];
    CompoundObstacle {
        cubes: Vec::new(),
        spheres: Vec::new(),
        cylinders,
        bounds_center: base + Vec3::new(0.0, 0.30, 0.0),
        bounds_radius: 0.78,
    }
}

fn bush(base: Vec3) -> CompoundObstacle {
    let green = Color::new(57, 132, 70);
    let light = shade(green, 17);
    let dark = shade(green, -14);
    let centers = [
        base + Vec3::new(0.0, 0.34, 0.0),
        base + Vec3::new(-0.29, 0.23, 0.05),
        base + Vec3::new(0.27, 0.29, 0.07),
        base + Vec3::new(-0.08, 0.19, -0.26),
        base + Vec3::new(0.09, 0.50, 0.14),
    ];
    let spheres = vec![
        Sphere::new(centers[0], 0.68, green),
        Sphere::new(centers[1], 0.40, dark),
        Sphere::new(centers[2], 0.54, green),
        Sphere::new(centers[3], 0.32, dark),
        Sphere::new(centers[4], 0.46, light),
    ];
    let joint = base + Vec3::new(0.0, 0.14, 0.0);
    let cylinders = centers[1..]
        .iter()
        .map(|center| Cylinder::new_between(joint, *center, 0.09, dark))
        .collect();
    CompoundObstacle {
        cubes: Vec::new(),
        spheres,
        cylinders,
        bounds_center: base + Vec3::new(0.0, 0.35, 0.0),
        bounds_radius: 0.74,
    }
}

fn keep_closest(closest: &mut Option<(Hit, Color)>, candidate: Option<Hit>, color: Color) {
    let Some(hit) = candidate else {
        return;
    };
    if closest
        .as_ref()
        .is_none_or(|(current, _)| hit.distance < current.distance)
    {
        *closest = Some((hit, color));
    }
}

fn ray_misses_sphere(ray: &Ray, center: Vec3, radius: f32) -> bool {
    let offset = ray.origin - center;
    let distance_squared = offset.dot(offset);
    let along_ray = offset.dot(ray.direction);
    (distance_squared > radius * radius && along_ray > 0.0)
        || along_ray * along_ray - (distance_squared - radius * radius) < 0.0
}

fn shade(color: Color, amount: i16) -> Color {
    let adjust = |channel: u8| (channel as i16 + amount).clamp(0, 255) as u8;
    Color::new(adjust(color.r), adjust(color.g), adjust(color.b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_is_a_group_of_five_cubes() {
        let ForestProp::Rock(model) = ForestProp::new(
            ForestObstacleKind::Rock,
            Vec3::default(),
            Color::new(130, 82, 52),
        ) else {
            panic!("se esperaba una roca");
        };
        assert_eq!(model.cubes.len(), 5);
        assert!(model.spheres.is_empty());
        assert!(model.cylinders.is_empty());
    }

    #[test]
    fn fallen_log_has_a_main_trunk_and_one_branch() {
        let ForestProp::FallenLog(model) = ForestProp::new(
            ForestObstacleKind::FallenLog,
            Vec3::default(),
            Color::new(130, 82, 52),
        ) else {
            panic!("se esperaba un tronco");
        };
        assert_eq!(model.cylinders.len(), 2);
    }

    #[test]
    fn bush_connects_its_five_spheres_with_cylinders() {
        let ForestProp::Bush(model) = ForestProp::new(
            ForestObstacleKind::Bush,
            Vec3::default(),
            Color::new(130, 82, 52),
        ) else {
            panic!("se esperaba un arbusto");
        };
        assert_eq!(model.spheres.len(), 5);
        assert_eq!(model.cylinders.len(), 4);
        let smallest = model
            .spheres
            .iter()
            .map(|sphere| sphere.radius)
            .fold(f32::INFINITY, f32::min);
        let largest = model
            .spheres
            .iter()
            .map(|sphere| sphere.radius)
            .fold(0.0_f32, f32::max);
        assert!(largest - smallest > 0.15);
    }
}
