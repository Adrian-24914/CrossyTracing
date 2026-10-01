use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    leaf_cube::LeafCube,
    material::Material,
    math::Vec3,
    ray::{Hit, Ray},
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
    #[cfg(test)]
    pub fn new(kind: ForestObstacleKind, base: Vec3, wood_color: Color) -> Self {
        Self::new_with_foliage(kind, base, wood_color, true)
    }

    pub fn new_with_foliage(
        kind: ForestObstacleKind,
        base: Vec3,
        wood_color: Color,
        has_foliage: bool,
    ) -> Self {
        match kind {
            ForestObstacleKind::Tree => {
                Self::Tree(Tree::with_foliage(base, wood_color, has_foliage))
            }
            ForestObstacleKind::Rock => Self::Rock(rock(base)),
            ForestObstacleKind::FallenLog => Self::FallenLog(fallen_log(base, wood_color)),
            ForestObstacleKind::Bush => Self::Bush(bush(base, wood_color)),
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<(Hit, Material)> {
        match self {
            Self::Tree(tree) => tree.intersect(ray),
            Self::Rock(model) | Self::FallenLog(model) | Self::Bush(model) => model.intersect(ray),
        }
    }

    pub fn intersect_shadow(&self, ray: &Ray) -> bool {
        match self {
            Self::Tree(tree) => tree.intersect_shadow(ray).is_some(),
            Self::Rock(model) | Self::FallenLog(model) | Self::Bush(model) => {
                model.intersect_shadow(ray)
            }
        }
    }

    pub fn foliage_anchors(&self) -> Option<&[FoliageAnchor; 5]> {
        match self {
            Self::Tree(tree) => Some(tree.foliage_anchors()),
            Self::Rock(_) | Self::FallenLog(_) | Self::Bush(_) => None,
        }
    }

    pub(crate) fn bounds(&self) -> (Vec3, f32) {
        match self {
            Self::Tree(tree) => tree.bounds(),
            Self::Rock(model) | Self::FallenLog(model) | Self::Bush(model) => {
                (model.bounds_center, model.bounds_radius)
            }
        }
    }
}

pub(crate) struct CompoundObstacle {
    cubes: Vec<Cube>,
    leaves: Vec<LeafCube>,
    cylinders: Vec<Cylinder>,
    bounds_center: Vec3,
    bounds_radius: f32,
}

impl CompoundObstacle {
    fn intersect(&self, ray: &Ray) -> Option<(Hit, Material)> {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return None;
        }

        let mut closest: Option<(Hit, Material)> = None;
        for cube in &self.cubes {
            keep_closest(&mut closest, cube.intersect(ray), cube.material);
        }
        for leaf in &self.leaves {
            keep_closest(&mut closest, leaf.intersect(ray), leaf.material);
        }
        for cylinder in &self.cylinders {
            keep_closest(&mut closest, cylinder.intersect(ray), cylinder.material);
        }
        closest
    }

    fn intersect_shadow(&self, ray: &Ray) -> bool {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return false;
        }
        // Igual que la copa de los árboles, las hojas del arbusto son parte
        // visual: el tronco conserva la sombra sin multiplicar los rayos de
        // sombra por los cinco cubos de follaje.
        self.cubes.iter().any(|cube| cube.intersect(ray).is_some())
            || self
                .cylinders
                .iter()
                .any(|cylinder| cylinder.intersect(ray).is_some())
    }
}

fn rock(base: Vec3) -> CompoundObstacle {
    let stone = Color::new(112, 119, 116);
    let dark = shade(stone, -18);
    let light = shade(stone, 16);
    let cubes = vec![
        stone_cube(
            base + Vec3::new(0.0, 0.17, 0.0),
            Vec3::new(0.62, 0.34, 0.55),
            stone,
        ),
        stone_cube(
            base + Vec3::new(-0.24, 0.15, 0.12),
            Vec3::new(0.38, 0.30, 0.36),
            dark,
        ),
        stone_cube(
            base + Vec3::new(0.25, 0.13, -0.11),
            Vec3::new(0.40, 0.26, 0.42),
            dark,
        ),
        stone_cube(
            base + Vec3::new(-0.06, 0.39, 0.01),
            Vec3::new(0.34, 0.28, 0.32),
            light,
        ),
        stone_cube(
            base + Vec3::new(0.18, 0.31, 0.18),
            Vec3::new(0.25, 0.22, 0.26),
            stone,
        ),
    ];
    CompoundObstacle {
        cubes,
        leaves: Vec::new(),
        cylinders: Vec::new(),
        bounds_center: base + Vec3::new(0.0, 0.28, 0.0),
        bounds_radius: 0.68,
    }
}

fn stone_cube(center: Vec3, size: Vec3, color: Color) -> Cube {
    Cube::with_material(center, size, Material::matte(color))
}

fn fallen_log(base: Vec3, color: Color) -> CompoundObstacle {
    let cylinders = vec![
        Cylinder::new_between_with_material(
            base + Vec3::new(-0.48, 0.20, -0.10),
            base + Vec3::new(0.48, 0.20, 0.10),
            0.34,
            wood_material(color),
        ),
        Cylinder::new_between_with_material(
            base + Vec3::new(0.04, 0.28, 0.01),
            base + Vec3::new(0.31, 0.53, 0.29),
            0.15,
            wood_material(shade(color, 12)),
        ),
    ];
    CompoundObstacle {
        cubes: Vec::new(),
        leaves: Vec::new(),
        cylinders,
        bounds_center: base + Vec3::new(0.0, 0.30, 0.0),
        bounds_radius: 0.78,
    }
}

fn wood_material(color: Color) -> Material {
    Material::matte(color)
}

fn bush(base: Vec3, wood_color: Color) -> CompoundObstacle {
    let green = Color::new(57, 132, 70);
    let light = shade(green, 17);
    let dark = shade(green, -14);
    // Cinco cubos solapados alrededor del tronco: conservan la silueta de
    // nube del arbusto anterior, sin abrir huecos entre sus hojas.
    let centers = [
        base + Vec3::new(0.0, 0.62, 0.0),
        base + Vec3::new(-0.17, 0.57, 0.04),
        base + Vec3::new(0.18, 0.59, 0.05),
        base + Vec3::new(-0.05, 0.54, -0.16),
        base + Vec3::new(0.04, 0.75, 0.09),
    ];
    let leaves = vec![
        bush_leaf(centers[0], 0.42, 0.18, 0.31, 0.42, green),
        bush_leaf(centers[1], 0.30, 1.07, 0.22, 0.58, dark),
        bush_leaf(centers[2], 0.34, 2.16, 0.37, 0.29, green),
        bush_leaf(centers[3], 0.27, 3.38, 0.26, 0.66, dark),
        bush_leaf(centers[4], 0.31, 4.71, 0.34, 0.47, light),
    ];
    let cylinders = vec![Cylinder::new_between_with_material(
        base + Vec3::new(0.0, 0.02, 0.0),
        base + Vec3::new(0.0, 0.60, 0.0),
        0.16,
        wood_material(shade(wood_color, -8)),
    )];
    CompoundObstacle {
        cubes: Vec::new(),
        leaves,
        cylinders,
        bounds_center: base + Vec3::new(0.0, 0.61, 0.0),
        bounds_radius: 0.70,
    }
}

fn bush_leaf(center: Vec3, size: f32, yaw: f32, tilt: f32, roll: f32, color: Color) -> LeafCube {
    LeafCube::new(
        center,
        Vec3::new(size, size, size),
        yaw,
        tilt,
        roll,
        Material::matte(color),
    )
}

fn keep_closest(closest: &mut Option<(Hit, Material)>, candidate: Option<Hit>, material: Material) {
    let Some(hit) = candidate else {
        return;
    };
    if closest
        .as_ref()
        .is_none_or(|(current, _)| hit.distance < current.distance)
    {
        *closest = Some((hit, material));
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
        assert!(model
            .cubes
            .iter()
            .all(|cube| cube.material.finish == crate::material::Finish::Matte));
        assert!(model.leaves.is_empty());
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
        assert!(model.leaves.is_empty());
        assert!(model
            .cylinders
            .iter()
            .all(|cylinder| cylinder.material.finish == crate::material::Finish::Matte));
    }

    #[test]
    fn bush_uses_five_leaf_cubes_and_one_hexagonal_trunk() {
        let ForestProp::Bush(model) = ForestProp::new(
            ForestObstacleKind::Bush,
            Vec3::default(),
            Color::new(130, 82, 52),
        ) else {
            panic!("se esperaba un arbusto");
        };
        assert!(model.cubes.is_empty());
        assert_eq!(model.leaves.len(), 5);
        assert_eq!(model.cylinders.len(), 1);
        assert!(model
            .leaves
            .iter()
            .all(|leaf| leaf.material.finish == crate::material::Finish::Matte));
    }
}
