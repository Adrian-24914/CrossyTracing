use crate::{
    color::Color,
    cylinder::Cylinder,
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
    foliage_anchors: [FoliageAnchor; 5],
    bounds_center: Vec3,
    bounds_radius: f32,
}

impl Tree {
    pub fn new(base: Vec3, color: Color) -> Self {
        let dark = shade(color, -10);
        let light = shade(color, 12);
        let mut cylinders = Vec::with_capacity(8);

        let trunk_levels = [0.0, 0.55, 1.10, 1.62, 2.12];
        let trunk_diameters = [0.42, 0.34, 0.26, 0.17];
        let trunk_colors = [dark, color, dark, light];
        for index in 0..trunk_diameters.len() {
            cylinders.push(Cylinder::new_between(
                base + Vec3::new(0.0, trunk_levels[index], 0.0),
                base + Vec3::new(0.0, trunk_levels[index + 1], 0.0),
                trunk_diameters[index],
                trunk_colors[index],
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

            cylinders.push(Cylinder::new_between(
                start,
                tip,
                0.15 - branch_index as f32 * 0.01,
                color,
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

        Self {
            cylinders,
            foliage_anchors,
            bounds_center: base + Vec3::new(0.0, 1.06, 0.0),
            bounds_radius: 1.4,
        }
    }

    pub fn foliage_anchors(&self) -> &[FoliageAnchor; 5] {
        &self.foliage_anchors
    }

    pub fn intersect(&self, ray: &Ray) -> Option<(Hit, Color)> {
        if ray_misses_sphere(ray, self.bounds_center, self.bounds_radius) {
            return None;
        }

        let mut closest: Option<(Hit, Color)> = None;
        for cylinder in &self.cylinders {
            let Some(hit) = cylinder.intersect(ray) else {
                continue;
            };
            if closest
                .as_ref()
                .is_none_or(|(current, _)| hit.distance < current.distance)
            {
                closest = Some((hit, cylinder.color));
            }
        }
        closest
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
    fn tree_uses_only_tapered_cylinder_sequences() {
        let tree = Tree::new(Vec3::default(), Color::new(120, 76, 44));

        assert_eq!(tree.cylinders.len(), 8);
        assert_eq!(tree.foliage_anchors.len(), 5);
        assert!(tree.cylinders[..4]
            .windows(2)
            .all(|segments| segments[1].radius < segments[0].radius));
        assert_eq!(tree.cylinders[4..].len(), 4);
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
    }
}
