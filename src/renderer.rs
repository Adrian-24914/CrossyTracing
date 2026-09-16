use crate::{
    color::Color,
    cube::Cube,
    framebuffer::Framebuffer,
    math::Vec3,
    orbit_camera::OrbitCamera,
    ray::{Hit, Ray},
    sphere::Sphere,
};

const AMBIENT_LIGHT: f32 = 0.22;

pub fn render(
    framebuffer: &mut Framebuffer,
    camera: &OrbitCamera,
    cubes: &[Cube],
    spheres: &[Sphere],
) {
    let light_direction = Vec3::new(-0.45, 0.85, 0.35).normalize();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let direction = camera.ray_direction(x, y, framebuffer.width, framebuffer.height);
            let ray = Ray::new(camera.eye, direction);
            let color = match closest_hit(&ray, cubes, spheres) {
                Some((hit, color)) => shade(hit, color, light_direction),
                None => sky_color(direction),
            };
            framebuffer.set_pixel(x, y, color.to_hex());
        }
    }
}

fn closest_hit(ray: &Ray, cubes: &[Cube], spheres: &[Sphere]) -> Option<(Hit, Color)> {
    let mut closest: Option<(Hit, Color)> = None;
    for cube in cubes {
        let Some(hit) = cube.intersect(ray) else {
            continue;
        };
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, cube.color));
        }
    }
    for sphere in spheres {
        let Some(hit) = sphere.intersect(ray) else {
            continue;
        };
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, sphere.color));
        }
    }
    closest
}

fn shade(hit: Hit, color: Color, light_direction: Vec3) -> Color {
    let diffuse = hit.normal.dot(light_direction).max(0.0);
    color.lit(AMBIENT_LIGHT + diffuse * (1.0 - AMBIENT_LIGHT))
}

fn sky_color(direction: Vec3) -> Color {
    let blend = (direction.y * 0.5 + 0.5).clamp(0.0, 1.0);
    Color::new(
        (35.0 + 65.0 * blend) as u8,
        (45.0 + 90.0 * blend) as u8,
        (65.0 + 125.0 * blend) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_4;

    #[test]
    fn cube_changes_pixels_in_front_of_the_camera() {
        let mut framebuffer = Framebuffer::new(80, 60);
        let camera = OrbitCamera::new(
            Vec3::new(4.0, 4.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            FRAC_PI_4,
        );
        let cubes = [Cube::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
            Color::new(220, 80, 60),
        )];

        render(&mut framebuffer, &camera, &cubes, &[]);

        let visible_pixels = framebuffer
            .color
            .iter()
            .enumerate()
            .filter(|(index, pixel)| {
                let x = *index % framebuffer.width;
                let y = *index / framebuffer.width;
                let direction = camera.ray_direction(x, y, framebuffer.width, framebuffer.height);
                **pixel != sky_color(direction).to_hex()
            })
            .count();
        assert!(visible_pixels > 100, "el cubo debería aparecer en cámara");
    }
}
