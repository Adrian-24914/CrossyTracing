use crate::{
    color::Color,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Cylinder {
    pub center: Vec3,
    pub axis: Vec3,
    pub half_length: f32,
    pub radius: f32,
    pub color: Color,
}

impl Cylinder {
    pub fn new_x(center: Vec3, length: f32, diameter: f32, color: Color) -> Self {
        Self {
            center,
            axis: Vec3::new(1.0, 0.0, 0.0),
            half_length: length * 0.5,
            radius: diameter * 0.5,
            color,
        }
    }

    pub fn new_between(start: Vec3, end: Vec3, diameter: f32, color: Color) -> Self {
        let span = end - start;
        let length = span.dot(span).sqrt();
        let axis = if length > 0.000_001 {
            span * (1.0 / length)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };

        Self {
            center: (start + end) * 0.5,
            axis,
            half_length: length * 0.5,
            radius: diameter * 0.5,
            color,
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let offset = ray.origin - self.center;
        let mut closest = self.intersect_side(ray, offset);
        let ray_axis = ray.direction.dot(self.axis);
        let offset_axis = offset.dot(self.axis);

        if ray_axis.abs() > 0.000_001 {
            for cap in [-self.half_length, self.half_length] {
                let distance = (cap - offset_axis) / ray_axis;
                if distance <= 0.001 || closest.as_ref().is_some_and(|hit| distance >= hit.distance)
                {
                    continue;
                }
                let point = offset + ray.direction * distance;
                let radial = point - self.axis * cap;
                if radial.dot(radial) <= self.radius * self.radius {
                    closest = Some(Hit {
                        distance,
                        normal: self.axis * cap.signum(),
                    });
                }
            }
        }

        closest
    }

    fn intersect_side(&self, ray: &Ray, offset: Vec3) -> Option<Hit> {
        let ray_axis = ray.direction.dot(self.axis);
        let offset_axis = offset.dot(self.axis);
        let radial_direction = ray.direction - self.axis * ray_axis;
        let radial_offset = offset - self.axis * offset_axis;
        let a = radial_direction.dot(radial_direction);
        if a < 0.000_001 {
            return None;
        }
        let half_b = radial_offset.dot(radial_direction);
        let c = radial_offset.dot(radial_offset) - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return None;
        }

        let square_root = discriminant.sqrt();
        for distance in [(-half_b - square_root) / a, (-half_b + square_root) / a] {
            if distance <= 0.001 {
                continue;
            }
            let axial_distance = offset_axis + ray_axis * distance;
            if axial_distance.abs() <= self.half_length {
                let point = offset + ray.direction * distance;
                let radial = point - self.axis * axial_distance;
                return Some(Hit {
                    distance,
                    normal: radial * (1.0 / self.radius),
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

    #[test]
    fn new_between_orients_a_vertical_cylinder() {
        let cylinder = Cylinder::new_between(
            Vec3::new(0.0, -2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            2.0,
            Color::new(140, 90, 50),
        );
        let ray = Ray::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = cylinder.intersect(&ray).unwrap();

        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.normal.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn new_between_orients_caps_along_the_segment() {
        let cylinder = Cylinder::new_between(
            Vec3::new(0.0, -2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            2.0,
            Color::new(140, 90, 50),
        );
        let ray = Ray::new(Vec3::new(0.0, 4.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let hit = cylinder.intersect(&ray).unwrap();

        assert!((hit.distance - 2.0).abs() < 0.0001);
        assert!((hit.normal.y - 1.0).abs() < 0.0001);
    }
}
