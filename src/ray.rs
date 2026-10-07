use crate::math::Vec3;

#[derive(Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inverse_direction: Vec3,
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub distance: f32,
    pub normal: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction,
            inverse_direction: Vec3::new(
                reciprocal_or_infinity(direction.x),
                reciprocal_or_infinity(direction.y),
                reciprocal_or_infinity(direction.z),
            ),
        }
    }
}

fn reciprocal_or_infinity(value: f32) -> f32 {
    if value.abs() < 0.000_001 {
        f32::INFINITY.copysign(value)
    } else {
        value.recip()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_caches_direction_reciprocals_for_bvh_traversal() {
        let ray = Ray::new(Vec3::default(), Vec3::new(2.0, -4.0, 0.0));

        assert!((ray.inverse_direction.x - 0.5).abs() < 0.0001);
        assert!((ray.inverse_direction.y + 0.25).abs() < 0.0001);
        assert!(ray.inverse_direction.z.is_infinite());
    }
}
