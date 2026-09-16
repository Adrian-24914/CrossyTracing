use crate::{
    color::Color,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: Color,
}

impl Sphere {
    pub fn new(center: Vec3, diameter: f32, color: Color) -> Self {
        Self {
            center,
            radius: diameter * 0.5,
            color,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let offset = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let half_b = offset.dot(ray.direction);
        let c = offset.dot(offset) - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let square_root = discriminant.sqrt();
        let mut distance = (-half_b - square_root) / a;
        if distance <= 0.001 {
            distance = (-half_b + square_root) / a;
            if distance <= 0.001 {
                return None;
            }
        }

        let point = ray.origin + ray.direction * distance;
        Some(Hit {
            distance,
            normal: (point - self.center) * (1.0 / self.radius),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_the_sphere() {
        let sphere = Sphere::new(Vec3::new(0.0, 0.0, 0.0), 2.0, Color::new(255, 255, 255));
        let ray = Ray::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0));

        let hit = sphere
            .intersect(&ray)
            .expect("el rayo debería tocar la esfera");
        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.normal.z - 1.0).abs() < 0.0001);
    }
}
