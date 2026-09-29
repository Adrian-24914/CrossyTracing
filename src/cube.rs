use crate::{
    color::Color,
    material::Material,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

impl Cube {
    pub fn new(center: Vec3, size: Vec3, color: Color) -> Self {
        Self::with_material(center, size, Material::matte(color))
    }

    pub fn with_material(center: Vec3, size: Vec3, material: Material) -> Self {
        let half_size = size * 0.5;
        Self {
            min: center - half_size,
            max: center + half_size,
            material,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let mut near = 0.001;
        let mut far = f32::INFINITY;
        let mut normal = Vec3::default();

        if !intersect_axis(
            ray.origin.x,
            ray.direction.x,
            self.min.x,
            self.max.x,
            Vec3::new(1.0, 0.0, 0.0),
            &mut near,
            &mut far,
            &mut normal,
        ) || !intersect_axis(
            ray.origin.y,
            ray.direction.y,
            self.min.y,
            self.max.y,
            Vec3::new(0.0, 1.0, 0.0),
            &mut near,
            &mut far,
            &mut normal,
        ) || !intersect_axis(
            ray.origin.z,
            ray.direction.z,
            self.min.z,
            self.max.z,
            Vec3::new(0.0, 0.0, 1.0),
            &mut near,
            &mut far,
            &mut normal,
        ) {
            return None;
        }

        let point = ray.origin + ray.direction * near;
        let (u, v) = face_uv(point, normal, self.min, self.max);
        Some(Hit {
            distance: near,
            normal,
            u,
            v,
        })
    }
}

fn face_uv(point: Vec3, normal: Vec3, minimum: Vec3, maximum: Vec3) -> (f32, f32) {
    let x = ((point.x - minimum.x) / (maximum.x - minimum.x)).clamp(0.0, 1.0);
    let y = ((point.y - minimum.y) / (maximum.y - minimum.y)).clamp(0.0, 1.0);
    let z = ((point.z - minimum.z) / (maximum.z - minimum.z)).clamp(0.0, 1.0);

    if normal.y > 0.5 {
        (x, 1.0 - z)
    } else if normal.y < -0.5 {
        (x, z)
    } else if normal.x > 0.5 {
        (z, 1.0 - y)
    } else if normal.x < -0.5 {
        (1.0 - z, 1.0 - y)
    } else if normal.z > 0.5 {
        (x, y)
    } else {
        (1.0 - x, y)
    }
}

fn intersect_axis(
    origin: f32,
    direction: f32,
    minimum: f32,
    maximum: f32,
    axis: Vec3,
    near: &mut f32,
    far: &mut f32,
    normal: &mut Vec3,
) -> bool {
    if direction.abs() < 0.000_001 {
        return origin >= minimum && origin <= maximum;
    }

    let inverse_direction = 1.0 / direction;
    let first = (minimum - origin) * inverse_direction;
    let second = (maximum - origin) * inverse_direction;
    let axis_near = first.min(second);
    let axis_far = first.max(second);

    if axis_near > *near {
        *near = axis_near;
        *normal = if direction > 0.0 { axis * -1.0 } else { axis };
    }
    *far = far.min(axis_far);
    *near <= *far
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_the_front_face() {
        let cube = Cube::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            Color::new(255, 255, 255),
        );
        let ray = Ray::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0));

        let hit = cube.intersect(&ray).expect("el rayo debería tocar el cubo");
        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert_eq!(hit.normal.z, 1.0);
    }

    #[test]
    fn parallel_ray_misses_the_cube() {
        let cube = Cube::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            Color::new(255, 255, 255),
        );
        let ray = Ray::new(Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));

        assert!(cube.intersect(&ray).is_none());
    }

    #[test]
    fn texture_coordinates_move_with_the_cube() {
        let first = Cube::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 2.0),
            Color::new(255, 255, 255),
        );
        let moved = Cube::new(
            Vec3::new(0.0, 0.0, 0.73),
            Vec3::new(2.0, 1.0, 2.0),
            Color::new(255, 255, 255),
        );
        let first_ray = Ray::new(Vec3::new(0.25, 2.0, 0.18), Vec3::new(0.0, -1.0, 0.0));
        let moved_ray = Ray::new(Vec3::new(0.25, 2.0, 0.91), Vec3::new(0.0, -1.0, 0.0));

        let first_hit = first.intersect(&first_ray).unwrap();
        let moved_hit = moved.intersect(&moved_ray).unwrap();

        assert!((first_hit.u - moved_hit.u).abs() < 0.0001);
        assert!((first_hit.v - moved_hit.v).abs() < 0.0001);
    }
}
