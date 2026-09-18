use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    framebuffer::Framebuffer,
    material::{Finish, Material},
    math::Vec3,
    obstacle::ForestProp,
    orbit_camera::OrbitCamera,
    ray::{Hit, Ray},
    skybox::Skybox,
    sphere::Sphere,
};
use std::thread;

const AMBIENT_LIGHT: f32 = 0.22;

pub fn render(
    framebuffer: &mut Framebuffer,
    camera: &OrbitCamera,
    cubes: &[Cube],
    spheres: &[Sphere],
    cylinders: &[Cylinder],
    forest_props: &[ForestProp],
    skybox: &Skybox,
) {
    let light_direction = Vec3::new(-0.45, 0.85, 0.35).normalize();
    let width = framebuffer.width;
    let height = framebuffer.height;
    let thread_count = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(height);
    let rows_per_thread = height.div_ceil(thread_count);

    thread::scope(|scope| {
        for (chunk_index, pixels) in framebuffer
            .color
            .chunks_mut(width * rows_per_thread)
            .enumerate()
        {
            let start_y = chunk_index * rows_per_thread;
            scope.spawn(move || {
                for (local_y, row) in pixels.chunks_mut(width).enumerate() {
                    let y = start_y + local_y;
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let direction = camera.ray_direction(x, y, width, height);
                        let ray = Ray::new(camera.eye, direction);
                        let color = trace_primary(
                            &ray,
                            cubes,
                            spheres,
                            cylinders,
                            forest_props,
                            light_direction,
                            skybox,
                        );
                        *pixel = color.to_hex();
                    }
                }
            });
        }
    });
}

fn closest_hit(
    ray: &Ray,
    cubes: &[Cube],
    spheres: &[Sphere],
    cylinders: &[Cylinder],
    forest_props: &[ForestProp],
    minimum_distance: f32,
) -> Option<(Hit, Material)> {
    let mut closest: Option<(Hit, Material)> = None;
    for cube in cubes {
        let Some(hit) = cube.intersect(ray) else {
            continue;
        };
        if hit.distance <= minimum_distance {
            continue;
        }
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, cube.material));
        }
    }
    for sphere in spheres {
        let Some(hit) = sphere.intersect(ray) else {
            continue;
        };
        if hit.distance <= minimum_distance {
            continue;
        }
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, sphere.material));
        }
    }
    for cylinder in cylinders {
        let Some(hit) = cylinder.intersect(ray) else {
            continue;
        };
        if hit.distance <= minimum_distance {
            continue;
        }
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, cylinder.material));
        }
    }
    for prop in forest_props {
        let Some((hit, material)) = prop.intersect(ray) else {
            continue;
        };
        if hit.distance <= minimum_distance {
            continue;
        }
        if closest
            .as_ref()
            .is_none_or(|(current, _)| hit.distance < current.distance)
        {
            closest = Some((hit, material));
        }
    }
    closest
}

fn trace_primary(
    ray: &Ray,
    cubes: &[Cube],
    spheres: &[Sphere],
    cylinders: &[Cylinder],
    forest_props: &[ForestProp],
    light_direction: Vec3,
    skybox: &Skybox,
) -> Color {
    let Some((hit, material)) = closest_hit(ray, cubes, spheres, cylinders, forest_props, 0.001)
    else {
        return skybox.sample(ray.direction);
    };
    let mut surface = shade(hit, material, light_direction, ray.direction * -1.0);
    if material.transparency > 0.0 {
        let behind = closest_hit(
            ray,
            cubes,
            spheres,
            cylinders,
            forest_props,
            hit.distance + 0.001,
        )
        .map(|(behind_hit, behind_material)| {
            shade(
                behind_hit,
                behind_material,
                light_direction,
                ray.direction * -1.0,
            )
        })
        .unwrap_or_else(|| skybox.sample(ray.direction));
        surface = blend(surface, behind, 1.0 - material.transparency);
    }

    if material.reflectivity > 0.0 {
        let hit_point = ray.origin + ray.direction * hit.distance;
        let reflected_direction =
            (ray.direction - hit.normal * (2.0 * ray.direction.dot(hit.normal))).normalize();
        let reflected_ray = Ray::new(hit_point + hit.normal * 0.002, reflected_direction);
        let reflected = closest_hit(
            &reflected_ray,
            cubes,
            spheres,
            cylinders,
            forest_props,
            0.001,
        )
        .map(|(reflected_hit, reflected_material)| {
            shade(
                reflected_hit,
                reflected_material,
                light_direction,
                reflected_direction * -1.0,
            )
        })
        .unwrap_or_else(|| skybox.sample(reflected_direction));
        surface = blend(reflected, surface, material.reflectivity);
    }

    surface
}

fn shade(hit: Hit, material: Material, light_direction: Vec3, view_direction: Vec3) -> Color {
    if material.finish == Finish::Unlit {
        return material.albedo;
    }

    let diffuse = hit.normal.dot(light_direction).max(0.0);
    let base = material
        .albedo
        .lit(AMBIENT_LIGHT + diffuse * (1.0 - AMBIENT_LIGHT));
    if material.finish != Finish::Glossy {
        return base;
    }

    let reflected_light = hit.normal * (2.0 * hit.normal.dot(light_direction)) - light_direction;
    let highlight = reflected_light
        .dot(view_direction)
        .max(0.0)
        .powf(material.shininess)
        * material.specular_strength;
    blend(Color::new(255, 255, 255), base, highlight)
}

fn blend(front: Color, behind: Color, opacity: f32) -> Color {
    let opacity = opacity.clamp(0.0, 1.0);
    let mix =
        |front: u8, behind: u8| (front as f32 * opacity + behind as f32 * (1.0 - opacity)) as u8;
    Color::new(
        mix(front.r, behind.r),
        mix(front.g, behind.g),
        mix(front.b, behind.b),
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
        let skybox = Skybox::solid(Color::new(50, 80, 120));

        render(&mut framebuffer, &camera, &cubes, &[], &[], &[], &skybox);

        let visible_pixels = framebuffer
            .color
            .iter()
            .enumerate()
            .filter(|(index, pixel)| {
                let x = *index % framebuffer.width;
                let y = *index / framebuffer.width;
                let direction = camera.ray_direction(x, y, framebuffer.width, framebuffer.height);
                **pixel != skybox.sample(direction).to_hex()
            })
            .count();
        assert!(visible_pixels > 100, "el cubo debería aparecer en cámara");
    }

    #[test]
    fn unlit_white_ignores_the_light_direction() {
        let hit = Hit {
            distance: 1.0,
            normal: Vec3::new(0.0, -1.0, 0.0),
        };
        let color = shade(
            hit,
            Material::unlit(Color::new(255, 255, 255)),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );

        assert_eq!(color.to_hex(), 0xFFFFFF);
    }

    #[test]
    fn translucent_surface_blends_with_the_object_behind_it() {
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0));
        let cubes = [
            Cube::with_material(
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(1.0, 1.0, 1.0),
                Material::translucent_matte(Color::new(0, 0, 255), 0.5),
            ),
            Cube::new(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 1.0),
                Color::new(255, 0, 0),
            ),
        ];
        let skybox = Skybox::solid(Color::new(50, 80, 120));

        let color = trace_primary(
            &ray,
            &cubes,
            &[],
            &[],
            &[],
            Vec3::new(0.0, 0.0, 1.0),
            &skybox,
        );

        assert_eq!(color.to_hex(), 0x7F007F);
    }

    #[test]
    fn reflective_surface_mixes_in_the_reflected_environment() {
        let ray = Ray::new(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let cubes = [Cube::with_material(
            Vec3::default(),
            Vec3::new(4.0, 0.2, 4.0),
            Material::reflective_glossy(Color::new(200, 0, 0), 0.0, 24.0, 0.5),
        )];
        let skybox = Skybox::solid(Color::new(100, 135, 190));

        let color = trace_primary(
            &ray,
            &cubes,
            &[],
            &[],
            &[],
            Vec3::new(0.0, 1.0, 0.0),
            &skybox,
        );

        assert!(color.r < 200);
        assert!(color.g > 0);
        assert!(color.b > 0);
    }
}
