use crate::{
    color::Color,
    cylinder::Cylinder,
    leaf_cube::LeafCube,
    material::Material,
    math::Vec3,
    ray::{Hit, Ray},
};

#[derive(Clone, Copy, Debug)]
pub struct FoliageAnchor {
    pub position: Vec3,
    pub suggested_size: f32,
}

pub struct Tree {
    cylinders: Vec<Cylinder>,
    leaves: Vec<LeafCube>,
    foliage_anchors: [FoliageAnchor; 5],
    bounds_center: Vec3,
    bounds_radius: f32,
}

impl Tree {
    #[cfg(test)]
    pub fn new(base: Vec3, color: Color) -> Self {
        Self::with_foliage(base, color, true)
    }

    pub fn with_foliage(base: Vec3, color: Color, has_foliage: bool) -> Self {
        let dark = shade(color, -10);
        let light = shade(color, 12);
        let mut cylinders = Vec::with_capacity(8);

        let trunk_levels = [0.0, 0.55, 1.10, 1.62, 2.12];
        let trunk_diameters = [0.42, 0.34, 0.26, 0.17];
        let trunk_colors = [dark, color, dark, light];
        for index in 0..trunk_diameters.len() {
            cylinders.push(Cylinder::new_between_with_material(
                base + Vec3::new(0.0, trunk_levels[index], 0.0),
                base + Vec3::new(0.0, trunk_levels[index + 1], 0.0),
                trunk_diameters[index],
                wood_material(trunk_colors[index]),
            ));
        }

        let mut foliage_anchors = [FoliageAnchor {
            position: base,
            suggested_size: 0.0,
        }; 5];

        let branch_layouts = [
            (Vec3::new(0.87, 0.0, 0.50), 0.78, 0.68, 0.40),
            (Vec3::new(-0.31, 0.0, 0.95), 1.03, 0.72, 0.52),
            (Vec3::new(-0.93, 0.0, -0.37), 1.28, 0.63, 0.46),
            (Vec3::new(0.42, 0.0, -0.91), 1.51, 0.66, 0.50),
        ];

        for (branch_index, (anchor, layout)) in foliage_anchors[..4]
            .iter_mut()
            .zip(branch_layouts)
            .enumerate()
        {
            let (direction, start_height, horizontal_length, rise) = layout;

            let start = base + Vec3::new(0.0, start_height, 0.0);
            let tip = start + direction * horizontal_length + Vec3::new(0.0, rise, 0.0);

            cylinders.push(Cylinder::new_between_with_material(
                start,
                tip,
                0.15 - branch_index as f32 * 0.01,
                wood_material(color),
            ));
            *anchor = FoliageAnchor {
                position: tip,
                suggested_size: 0.58,
            };
        }

        foliage_anchors[4] = FoliageAnchor {
            position: base + Vec3::new(0.0, trunk_levels[trunk_levels.len() - 1], 0.0),
            suggested_size: 0.62,
        };

        // Cada ancla compone una copa piramidal: los vértices de cubos girados
        // se apoyan en el centro de la cara superior del cubo inferior.
        let leaves = if has_foliage {
            foliage_anchors
                .iter()
                .enumerate()
                .flat_map(|(anchor_index, anchor)| foliage_crown(*anchor, anchor_index))
                .collect()
        } else {
            Vec::new()
        };

        // Los límites anteriores eran deliberadamente amplios. Ajustarlos a
        // cada variante evita abrir la geometría de un árbol cuando un rayo
        // pasa cerca, pero no puede tocar ni su tronco ni su copa.
        let (bounds_center, bounds_radius) = if has_foliage {
            (base + Vec3::new(0.0, 1.72, 0.0), 1.78)
        } else {
            (base + Vec3::new(0.0, 1.08, 0.0), 1.25)
        };

        Self {
            cylinders,
            leaves,
            foliage_anchors,
            bounds_center,
            bounds_radius,
        }
    }

    pub fn foliage_anchors(&self) -> &[FoliageAnchor; 5] {
        &self.foliage_anchors
    }

    pub(crate) fn bounds(&self) -> (Vec3, f32) {
        (self.bounds_center, self.bounds_radius)
    }

    pub fn intersect(&self, ray: &Ray) -> Option<(Hit, Material)> {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return None;
        }

        let mut closest: Option<(Hit, Material)> = None;
        for cylinder in &self.cylinders {
            let Some(hit) = cylinder.intersect(ray) else {
                continue;
            };
            if closest
                .as_ref()
                .is_none_or(|(current, _)| hit.distance < current.distance)
            {
                closest = Some((hit, cylinder.material));
            }
        }
        for leaf in &self.leaves {
            let Some(hit) = leaf.intersect(ray) else {
                continue;
            };
            if closest
                .as_ref()
                .is_none_or(|(current, _)| hit.distance < current.distance)
            {
                closest = Some((hit, leaf.material));
            }
        }
        closest
    }

    /// El follaje se dibuja, pero no se usa como oclusor: evita sombras densas
    /// y ruidosas de cubos semitransparentes sobre el diorama.
    pub fn intersect_shadow(&self, ray: &Ray) -> Option<Hit> {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return None;
        }
        self.cylinders
            .iter()
            .find_map(|cylinder| cylinder.intersect(ray))
    }
}

fn foliage_crown(anchor: FoliageAnchor, anchor_index: usize) -> Vec<LeafCube> {
    let levels = if anchor_index == 4 { 2 } else { 1 };
    let diagonal_half = 3.0_f32.sqrt() * 0.5;
    let mut leaves = Vec::with_capacity(levels);
    let mut center = anchor.position;
    let mut previous_size = 0.0;
    for level in 0..levels {
        // Copas pequeñas: dejan visible el tronco cilíndrico y sus ramas.
        let size = anchor.suggested_size * (0.94 - level as f32 * 0.21);
        if level > 0 {
            center.y += (previous_size + size) * diagonal_half;
        }
        leaves.push(LeafCube::new(
            center,
            Vec3::new(size, size, size),
            anchor_index as f32 * 1.22 + 0.36,
            0.615_48,
            std::f32::consts::FRAC_PI_4,
            foliage_material(anchor_index + level),
        ));
        previous_size = size;
    }
    leaves
}

fn foliage_material(anchor_index: usize) -> Material {
    let color = match anchor_index % 3 {
        0 => Color::new(55, 119, 61),
        1 => Color::new(73, 142, 68),
        _ => Color::new(46, 101, 57),
    };
    Material::translucent_matte(color, 0.25)
}

fn wood_material(color: Color) -> Material {
    Material::matte(color)
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
    fn tree_uses_only_tapered_cylinder_sequences() {
        let tree = Tree::new(Vec3::default(), Color::new(120, 76, 44));

        assert_eq!(tree.cylinders.len(), 8);
        assert_eq!(tree.leaves.len(), 6);
        assert_eq!(tree.foliage_anchors.len(), 5);
        assert!(tree.cylinders[..4]
            .windows(2)
            .all(|segments| segments[1].radius < segments[0].radius));
        assert_eq!(tree.cylinders[4..].len(), 4);
        assert!(tree
            .cylinders
            .iter()
            .all(|cylinder| cylinder.material.finish == crate::material::Finish::Matte));
        assert!(tree
            .leaves
            .iter()
            .all(|leaf| (leaf.material.transparency - 0.25).abs() < 0.0001));
    }

    #[test]
    fn branch_layout_is_fixed_and_exposes_sprite_anchors() {
        let first = Tree::new(Vec3::default(), Color::new(120, 76, 44));
        let second = Tree::new(Vec3::default(), Color::new(120, 76, 44));

        for (left, right) in first.foliage_anchors.iter().zip(second.foliage_anchors) {
            assert!((left.position.x - right.position.x).abs() < 0.0001);
            assert!((left.position.y - right.position.y).abs() < 0.0001);
            assert!((left.position.z - right.position.z).abs() < 0.0001);
            assert!(left.suggested_size > 0.0);
        }
    }

    #[test]
    fn foliage_is_kept_or_removed_from_the_generated_tree() {
        let leafy = Tree::with_foliage(Vec3::default(), Color::new(120, 76, 44), true);
        let bare = Tree::with_foliage(Vec3::default(), Color::new(120, 76, 44), false);

        assert_eq!(leafy.leaves.len(), 6);
        assert!(bare.leaves.is_empty());
    }

    #[test]
    fn tree_bounds_reject_a_distant_ray() {
        let tree = Tree::new(Vec3::default(), Color::new(120, 76, 44));
        let ray = Ray::new(Vec3::new(20.0, 1.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

        assert!(tree.intersect(&ray).is_none());
    }

    #[test]
    fn tree_bounds_keep_visible_trunk_hits() {
        let tree = Tree::new(Vec3::default(), Color::new(120, 76, 44));
        let ray = Ray::new(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.0, 0.0, -1.0));

        assert!(tree.intersect(&ray).is_some());
        assert!(tree.intersect_shadow(&ray).is_some());
    }
}
