use crate::{
    color::Color,
    material::Material,
    math::Vec3,
    ray::{Hit, Ray},
};

pub struct Cylinder {
    pub center: Vec3,
    pub axis: Vec3,
    pub half_length: f32,
    pub radius: f32,
    pub material: Material,
    side_normals: [Vec3; 3],
}

impl Cylinder {
    pub fn new_x_with_material(
        center: Vec3,
        length: f32,
        diameter: f32,
        material: Material,
    ) -> Self {
        let axis = Vec3::new(1.0, 0.0, 0.0);
        Self {
            center,
            axis,
            half_length: length * 0.5,
            radius: diameter * 0.5,
            material,
            side_normals: hexagon_side_normals(axis),
        }
    }

    pub fn new_between(start: Vec3, end: Vec3, diameter: f32, color: Color) -> Self {
        Self::new_between_with_material(start, end, diameter, Material::matte(color))
    }

    pub fn new_between_with_material(
        start: Vec3,
        end: Vec3,
        diameter: f32,
        material: Material,
    ) -> Self {
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
            material,
            side_normals: hexagon_side_normals(axis),
        }
    }

    pub fn rotate_orientation(&mut self, mut rotate: impl FnMut(Vec3) -> Vec3) {
        self.axis = rotate(self.axis).normalize_or_zero();
        for normal in &mut self.side_normals {
            *normal = rotate(*normal).normalize_or_zero();
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let offset = ray.origin - self.center;
        let bounds_squared = self.half_length * self.half_length + self.radius * self.radius;
        let distance_squared = offset.dot(offset);
        let along_ray = offset.dot(ray.direction);
        if (distance_squared > bounds_squared && along_ray > 0.0)
            || along_ray * along_ray - (distance_squared - bounds_squared) < 0.0
        {
            return None;
        }

        let mut entry = f32::NEG_INFINITY;
        let mut exit = f32::INFINITY;
        let mut entry_normal = self.axis;
        let mut exit_normal = self.axis;

        if !clip_slab(
            offset,
            ray.direction,
            self.axis,
            self.half_length,
            &mut entry,
            &mut exit,
            &mut entry_normal,
            &mut exit_normal,
        ) {
            return None;
        }

        // El radio sigue siendo el radio exterior. La distancia a cada cara de
        // un hexagono regular es radio * cos(30 grados).
        let apothem = self.radius * 0.866_025_4;
        for normal in self.side_normals {
            if !clip_slab(
                offset,
                ray.direction,
                normal,
                apothem,
                &mut entry,
                &mut exit,
                &mut entry_normal,
                &mut exit_normal,
            ) {
                return None;
            }
        }

        if entry > 0.001 {
            Some(Hit {
                distance: entry,
                normal: entry_normal,
            })
        } else if exit > 0.001 {
            Some(Hit {
                distance: exit,
                normal: exit_normal,
            })
        } else {
            None
        }
    }
}

fn hexagon_side_normals(axis: Vec3) -> [Vec3; 3] {
    let reference = if axis.y.abs() < 0.999 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let first = axis.cross(reference).normalize_or_zero();
    let second = axis.cross(first).normalize_or_zero();
    const COS_60: f32 = 0.5;
    const SIN_60: f32 = 0.866_025_4;

    [
        first,
        first * COS_60 + second * SIN_60,
        first * -COS_60 + second * SIN_60,
    ]
}

#[allow(clippy::too_many_arguments)]
fn clip_slab(
    offset: Vec3,
    direction: Vec3,
    normal: Vec3,
    extent: f32,
    entry: &mut f32,
    exit: &mut f32,
    entry_normal: &mut Vec3,
    exit_normal: &mut Vec3,
) -> bool {
    let origin_distance = offset.dot(normal);
    let direction_distance = direction.dot(normal);

    if direction_distance.abs() < 0.000_001 {
        return origin_distance.abs() <= extent;
    }

    let mut near = (-extent - origin_distance) / direction_distance;
    let mut far = (extent - origin_distance) / direction_distance;
    let mut near_normal = normal * -1.0;
    let mut far_normal = normal;
    if near > far {
        std::mem::swap(&mut near, &mut far);
        std::mem::swap(&mut near_normal, &mut far_normal);
    }

    if near > *entry {
        *entry = near;
        *entry_normal = near_normal;
    }
    if far < *exit {
        *exit = far;
        *exit_normal = far_normal;
    }

    *entry <= *exit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cylinder() -> Cylinder {
        Cylinder::new_x_with_material(
            Vec3::new(0.0, 0.0, 0.0),
            4.0,
            2.0,
            Material::matte(Color::new(140, 90, 50)),
        )
    }

    #[test]
    fn ray_hits_a_flat_hexagonal_side() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0));
        let hit = test_cylinder().intersect(&ray).unwrap();
        assert!((hit.distance - (3.0 - 0.866_025_4)).abs() < 0.0001);
        assert!((hit.normal.z - 1.0).abs() < 0.0001);
    }

    #[test]
    fn ray_outside_a_hexagonal_corner_misses() {
        let ray = Ray::new(Vec3::new(3.0, 0.8, 0.6), Vec3::new(-1.0, 0.0, 0.0));
        assert!(test_cylinder().intersect(&ray).is_none());
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

        assert!((hit.distance - (3.0 - 0.866_025_4)).abs() < 0.0001);
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
