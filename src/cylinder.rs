use crate::{
    color::Color,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Cylinder {
    pub center: Vec3,
    pub half_length: f32,
    pub radius: f32,
    pub color: Color,
}

impl Cylinder {
    pub fn new_x(center: Vec3, length: f32, diameter: f32, color: Color) -> Self {
        Self {
            center,
            half_length: length * 0.5,
            radius: diameter * 0.5,
            color,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let offset = ray.origin - self.center;
        let mut closest = self.intersect_side(ray, offset);

        if ray.direction.x.abs() > 0.000_001 {
            for cap in [-self.half_length, self.half_length] {
                let distance = (cap - offset.x) / ray.direction.x;
                if distance <= 0.001 || closest.as_ref().is_some_and(|hit| distance >= hit.distance)
                {
                    continue;
                }
                let y = offset.y + ray.direction.y * distance;
                let z = offset.z + ray.direction.z * distance;
                if y * y + z * z <= self.radius * self.radius {
                    closest = Some(Hit {
                        distance,
                        normal: Vec3::new(cap.signum(), 0.0, 0.0),
                    });
                }
            }
        }

        closest
    }

    fn intersect_side(&self, ray: &Ray, offset: Vec3) -> Option<Hit> {
        let a = ray.direction.y * ray.direction.y + ray.direction.z * ray.direction.z;
        if a < 0.000_001 {
            return None;
        }
        let half_b = offset.y * ray.direction.y + offset.z * ray.direction.z;
        let c = offset.y * offset.y + offset.z * offset.z - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let square_root = discriminant.sqrt();
        for distance in [(-half_b - square_root) / a, (-half_b + square_root) / a] {
            if distance <= 0.001 {
                continue;
            }
            let x = offset.x + ray.direction.x * distance;
            if x.abs() <= self.half_length {
                let y = offset.y + ray.direction.y * distance;
                let z = offset.z + ray.direction.z * distance;
                return Some(Hit {
                    distance,
                    normal: Vec3::new(0.0, y / self.radius, z / self.radius),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cylinder() -> Cylinder {
        Cylinder::new_x(Vec3::new(0.0, 0.0, 0.0), 4.0, 2.0, Color::new(140, 90, 50))
    }

    #[test]
    fn ray_hits_the_round_side() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = test_cylinder().intersect(&ray).unwrap();
        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.normal.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn ray_hits_the_flat_cap() {
        let ray = Ray::new(Vec3::new(4.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0));
        let hit = test_cylinder().intersect(&ray).unwrap();
        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.normal.x - 1.0).abs() < 0.0001);
    }
}
