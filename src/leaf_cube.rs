use crate::{
    material::Material,
    math::Vec3,
    ray::{Hit, Ray},
};

/// Cubo orientado para el follaje low-poly. Su tamaño sigue siendo el de un cubo,
/// pero sus ejes locales permiten que cada hoja apunte en una dirección distinta.
pub struct LeafCube {
    center: Vec3,
    half_size: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    pub material: Material,
}

impl LeafCube {
    pub fn new(
        center: Vec3,
        size: Vec3,
        yaw: f32,
        tilt: f32,
        roll: f32,
        material: Material,
    ) -> Self {
        let forward = Vec3::new(yaw.cos() * tilt.cos(), tilt.sin(), yaw.sin() * tilt.cos());
        let base_right = Vec3::new(-yaw.sin(), 0.0, yaw.cos());
        let base_up = base_right.cross(&forward).normalize();
        let right = base_right * roll.cos() + base_up * roll.sin();
        let up = base_up * roll.cos() - base_right * roll.sin();
        Self {
            center,
            half_size: size * 0.5,
            right,
            up,
            forward,
            material,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let offset = ray.origin - self.center;
        let local_origin = Vec3::new(
            offset.dot(self.right),
            offset.dot(self.up),
            offset.dot(self.forward),
        );
        let local_direction = Vec3::new(
            ray.direction.dot(self.right),
            ray.direction.dot(self.up),
            ray.direction.dot(self.forward),
        );
        let mut near = 0.001;
        let mut far = f32::INFINITY;
        let mut normal = Vec3::default();
        for (origin, direction, half_size, axis) in [
            (
                local_origin.x,
                local_direction.x,
                self.half_size.x,
                self.right,
            ),
            (local_origin.y, local_direction.y, self.half_size.y, self.up),
            (
                local_origin.z,
                local_direction.z,
                self.half_size.z,
                self.forward,
            ),
        ] {
            if direction.abs() < 0.000_001 {
                if origin < -half_size || origin > half_size {
                    return None;
                }
                continue;
            }
            let first = (-half_size - origin) / direction;
            let second = (half_size - origin) / direction;
            let axis_near = first.min(second);
            let axis_far = first.max(second);
            if axis_near > near {
                near = axis_near;
                normal = if direction > 0.0 { axis * -1.0 } else { axis };
            }
            far = far.min(axis_far);
            if near > far {
                return None;
            }
        }
        Some(Hit {
            distance: near,
            normal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{color::Color, material::Material};

    #[test]
    fn rotated_leaf_cube_is_hit_with_a_world_space_normal() {
        let leaf = LeafCube::new(
            Vec3::default(),
            Vec3::new(2.0, 0.2, 1.0),
            0.6,
            0.3,
            0.4,
            Material::translucent_matte(Color::new(80, 150, 72), 0.42),
        );
        let ray = Ray::new(Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = leaf.intersect(&ray).expect("la hoja rotada debe tocarse");

        assert!(hit.distance > 0.0);
        assert!(hit.normal.y > 0.0);
    }
}
