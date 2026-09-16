use crate::{
    color::Color,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub color: Color,
}

impl Cube {
    pub fn new(center: Vec3, size: Vec3, color: Color) -> Self {
        let half_size = size * 0.5;
        Self {
            min: center - half_size,
            max: center + half_size,
            color,
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

        Some(Hit {
            distance: near,
            normal,
        })
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
}
