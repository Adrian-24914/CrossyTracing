use crate::{
    color::Color,
    cube::Cube,
    cylinder::Cylinder,
    framebuffer::Framebuffer,
    material::{Finish, Material},
    math::Vec3,
    obstacle::ForestProp,
    orbit_camera::{CameraRayGrid, OrbitCamera},
    ray::{Hit, Ray},
    skybox::Skybox,
    sphere::Sphere,
};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

const AMBIENT_LIGHT: f32 = 0.34;
const SKY_FILL_LIGHT: f32 = 0.10;
const EXPOSURE: f32 = 1.06;

#[derive(Clone, Copy)]
pub struct RenderResources<'a> {
    pub skybox: &'a Skybox,
}

#[derive(Clone, Copy)]
enum SceneObject {
    Cube(usize),
    Sphere(usize),
    Cylinder(usize),
    ForestProp(usize),
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn around(center: Vec3, radius: f32) -> Self {
        let extent = Vec3::new(radius, radius, radius);
        Self {
            min: center - extent,
            max: center + extent,
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
}

#[derive(Clone, Copy)]
struct BoundedObject {
    object: SceneObject,
    bounds: Bounds,
}

struct BvhNode {
    bounds: Bounds,
    left: usize,
    right: usize,
    start: usize,
    count: usize,
}

struct TraceScene<'a> {
    cubes: &'a [Cube],
    spheres: &'a [Sphere],
    cylinders: &'a [Cylinder],
    forest_props: &'a [ForestProp],
    objects: Vec<BoundedObject>,
    nodes: Vec<BvhNode>,
    root: Option<usize>,
    shadow_objects: Vec<BoundedObject>,
    shadow_nodes: Vec<BvhNode>,
    shadow_root: Option<usize>,
}

impl<'a> TraceScene<'a> {
    fn new(
        cubes: &'a [Cube],
        spheres: &'a [Sphere],
        cylinders: &'a [Cylinder],
        forest_props: &'a [ForestProp],
    ) -> Self {
        let mut objects =
            Vec::with_capacity(cubes.len() + spheres.len() + cylinders.len() + forest_props.len());
        objects.extend(cubes.iter().enumerate().map(|(index, cube)| BoundedObject {
            object: SceneObject::Cube(index),
            bounds: Bounds {
                min: cube.min,
                max: cube.max,
            },
        }));
        objects.extend(
            spheres
                .iter()
                .enumerate()
                .map(|(index, sphere)| BoundedObject {
                    object: SceneObject::Sphere(index),
                    bounds: Bounds::around(sphere.center, sphere.radius),
                }),
        );
        objects.extend(cylinders.iter().enumerate().map(|(index, cylinder)| {
            let extent = Vec3::new(
                cylinder.axis.x.abs() * cylinder.half_length + cylinder.radius,
                cylinder.axis.y.abs() * cylinder.half_length + cylinder.radius,
                cylinder.axis.z.abs() * cylinder.half_length + cylinder.radius,
            );
            BoundedObject {
                object: SceneObject::Cylinder(index),
                bounds: Bounds {
                    min: cylinder.center - extent,
                    max: cylinder.center + extent,
                },
            }
        }));
        objects.extend(forest_props.iter().enumerate().map(|(index, prop)| {
            let (center, radius) = prop.bounds();
            BoundedObject {
                object: SceneObject::ForestProp(index),
                bounds: Bounds::around(center, radius),
            }
        }));

        let mut nodes = Vec::with_capacity(objects.len() * 2);
        let root = (!objects.is_empty()).then(|| {
            let object_count = objects.len();
            build_bvh_node(&mut objects, &mut nodes, 0, object_count)
        });
        let mut shadow_objects: Vec<_> = forest_props
            .iter()
            .enumerate()
            .map(|(index, prop)| {
                let (center, radius) = prop.bounds();
                BoundedObject {
                    object: SceneObject::ForestProp(index),
                    bounds: Bounds::around(center, radius),
                }
            })
            .collect();
        let mut shadow_nodes = Vec::with_capacity(shadow_objects.len() * 2);
        let shadow_root = (!shadow_objects.is_empty()).then(|| {
            let object_count = shadow_objects.len();
            build_bvh_node(&mut shadow_objects, &mut shadow_nodes, 0, object_count)
        });
        Self {
            cubes,
            spheres,
            cylinders,
            forest_props,
            objects,
            nodes,
            root,
            shadow_objects,
            shadow_nodes,
            shadow_root,
        }
    }
}

fn build_bvh_node(
    objects: &mut [BoundedObject],
    nodes: &mut Vec<BvhNode>,
    start: usize,
    end: usize,
) -> usize {
    let bounds = objects[start..end]
        .iter()
        .skip(1)
        .fold(objects[start].bounds, |bounds, object| {
            bounds.union(object.bounds)
        });
    let node_index = nodes.len();
    nodes.push(BvhNode {
        bounds,
        left: usize::MAX,
        right: usize::MAX,
        start,
        count: end - start,
    });
    if end - start <= 4 {
        return node_index;
    }

    let extent = bounds.max - bounds.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    objects[start..end].sort_unstable_by(|left, right| {
        axis_value(left.bounds.center(), axis).total_cmp(&axis_value(right.bounds.center(), axis))
    });
    let middle = start + (end - start) / 2;
    let left = build_bvh_node(objects, nodes, start, middle);
    let right = build_bvh_node(objects, nodes, middle, end);
    nodes[node_index].left = left;
    nodes[node_index].right = right;
    nodes[node_index].count = 0;
    node_index
}

fn axis_value(value: Vec3, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

pub fn render(
    framebuffer: &mut Framebuffer,
    camera: &OrbitCamera,
    cubes: &[Cube],
    spheres: &[Sphere],
    cylinders: &[Cylinder],
    forest_props: &[ForestProp],
    resources: RenderResources<'_>,
) {
    let light_direction = Vec3::new(-0.45, 0.85, 0.35).normalize_or_zero();
    let width = framebuffer.width;
    let height = framebuffer.height;
    let ray_grid = camera.ray_grid(width, height);
    let trace_scene = TraceScene::new(cubes, spheres, cylinders, forest_props);

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Rayon conserva un pool de hilos entre frames y reparte las filas de
        // manera dinámica. Así, las zonas con agua, reflejos o muchos objetos no
        // dejan a un único hilo trabajando mientras los demás ya terminaron.
        framebuffer
            .color
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                render_row(row, y, ray_grid, &trace_scene, light_direction, resources);
            });
    }

    #[cfg(target_arch = "wasm32")]
    {
        framebuffer
            .color
            .chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                render_row(row, y, ray_grid, &trace_scene, light_direction, resources);
            });
    }
}

fn render_row(
    row: &mut [u32],
    y: usize,
    ray_grid: CameraRayGrid,
    trace_scene: &TraceScene<'_>,
    light_direction: Vec3,
    resources: RenderResources<'_>,
) {
    let mut rays = ray_grid.row(y);
    for pixel in row {
        let ray = rays.next();
        let color = trace_primary(&ray, trace_scene, light_direction, resources);
        *pixel = color.to_hex();
    }
}

fn closest_hit(
    ray: &Ray,
    scene: &TraceScene<'_>,
    minimum_distance: f32,
) -> Option<(Hit, Material)> {
    closest_hit_filtered(ray, scene, minimum_distance, false)
}

fn closest_hit_filtered(
    ray: &Ray,
    scene: &TraceScene<'_>,
    minimum_distance: f32,
    opaque_only: bool,
) -> Option<(Hit, Material)> {
    let mut closest: Option<(Hit, Material)> = None;
    let Some(root) = scene.root else {
        return None;
    };
    let mut stack = [0_usize; 64];
    let mut stack_length = 1;
    stack[0] = root;

    while stack_length > 0 {
        stack_length -= 1;
        let node = &scene.nodes[stack[stack_length]];
        let maximum_distance = closest
            .as_ref()
            .map(|(hit, _)| hit.distance)
            .unwrap_or(f32::INFINITY);
        if ray_bounds_near(ray, node.bounds, maximum_distance).is_none() {
            continue;
        }

        if node.count > 0 {
            for bounded in &scene.objects[node.start..node.start + node.count] {
                let Some((hit, material)) =
                    intersect_object(ray, scene, bounded.object, opaque_only)
                else {
                    continue;
                };
                if hit.distance > minimum_distance
                    && closest
                        .as_ref()
                        .is_none_or(|(current, _)| hit.distance < current.distance)
                {
                    closest = Some((hit, material));
                }
            }
            continue;
        }

        let left = &scene.nodes[node.left];
        let right = &scene.nodes[node.right];
        let left_near = ray_bounds_near(ray, left.bounds, maximum_distance);
        let right_near = ray_bounds_near(ray, right.bounds, maximum_distance);
        match (left_near, right_near) {
            (Some(left_distance), Some(right_distance)) => {
                let (near, far) = if left_distance <= right_distance {
                    (node.left, node.right)
                } else {
                    (node.right, node.left)
                };
                stack[stack_length] = far;
                stack[stack_length + 1] = near;
                stack_length += 2;
            }
            (Some(_), None) => {
                stack[stack_length] = node.left;
                stack_length += 1;
            }
            (None, Some(_)) => {
                stack[stack_length] = node.right;
                stack_length += 1;
            }
            (None, None) => {}
        }
    }
    closest
}

fn intersect_object(
    ray: &Ray,
    scene: &TraceScene<'_>,
    object: SceneObject,
    opaque_only: bool,
) -> Option<(Hit, Material)> {
    let (hit, material) = match object {
        SceneObject::Cube(index) => {
            let object = &scene.cubes[index];
            (object.intersect(ray)?, object.material)
        }
        SceneObject::Sphere(index) => {
            let object = &scene.spheres[index];
            (object.intersect(ray)?, object.material)
        }
        SceneObject::Cylinder(index) => {
            let object = &scene.cylinders[index];
            (object.intersect(ray)?, object.material)
        }
        SceneObject::ForestProp(index) => scene.forest_props[index].intersect(ray)?,
    };
    (!opaque_only || material.transparency <= 0.0).then_some((hit, material))
}

fn ray_bounds_near(ray: &Ray, bounds: Bounds, maximum_distance: f32) -> Option<f32> {
    let mut near: f32 = 0.001;
    let mut far = maximum_distance;
    for (origin, direction, inverse, minimum, maximum) in [
        (
            ray.origin.x,
            ray.direction.x,
            ray.inverse_direction.x,
            bounds.min.x,
            bounds.max.x,
        ),
        (
            ray.origin.y,
            ray.direction.y,
            ray.inverse_direction.y,
            bounds.min.y,
            bounds.max.y,
        ),
        (
            ray.origin.z,
            ray.direction.z,
            ray.inverse_direction.z,
            bounds.min.z,
            bounds.max.z,
        ),
    ] {
        if direction.abs() < 0.000_001 {
            if origin < minimum || origin > maximum {
                return None;
            }
            continue;
        }
        let first = (minimum - origin) * inverse;
        let second = (maximum - origin) * inverse;
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near > far {
            return None;
        }
    }
    Some(near)
}

fn trace_primary(
    ray: &Ray,
    scene: &TraceScene<'_>,
    light_direction: Vec3,
    resources: RenderResources<'_>,
) -> Color {
    let Some((hit, material)) = closest_hit(ray, scene, 0.001) else {
        return resources.skybox.sample(ray.direction);
    };
    let hit_point = ray.origin + ray.direction * hit.distance;
    let shadowed = material.finish != Finish::Unlit
        && material.transparency <= 0.0
        // La sombra proyectada se aprecia sobre las superficies horizontales
        // del diorama. Evitar caras laterales ahorra rayos secundarios sin
        // perder la lectura visual de árboles, troncos y obstáculos.
        && hit.normal.y > 0.90
        && hit.normal.dot(light_direction) > 0.0
        && is_shadowed(hit_point, hit.normal, light_direction, scene);
    let mut surface = shade_with_shadow(ray, hit, material, light_direction, shadowed);
    if material.transparency > 0.0 {
        let behind = if material.refraction_index > 1.0 {
            refracted_color(ray, hit, material, scene, light_direction, resources)
        } else {
            closest_hit(ray, scene, hit.distance + 0.001)
                .map(|(behind_hit, behind_material)| {
                    shade(ray, behind_hit, behind_material, light_direction)
                })
                .unwrap_or_else(|| resources.skybox.sample(ray.direction))
        };
        surface = blend(surface, behind, 1.0 - material.transparency);
    }

    if material.reflectivity > 0.0 {
        let reflected_direction = (ray.direction
            - hit.normal * (2.0 * ray.direction.dot(hit.normal)))
        .normalize_or_zero();
        let reflected_ray = Ray::new(hit_point + hit.normal * 0.002, reflected_direction);
        let reflected = closest_hit(&reflected_ray, scene, 0.001)
            .map(|(reflected_hit, reflected_material)| {
                shade(
                    &reflected_ray,
                    reflected_hit,
                    reflected_material,
                    light_direction,
                )
            })
            .unwrap_or_else(|| resources.skybox.sample(reflected_direction));
        surface = blend(reflected, surface, material.reflectivity);
    }

    surface
}

fn refracted_color(
    ray: &Ray,
    hit: Hit,
    material: Material,
    scene: &TraceScene<'_>,
    light_direction: Vec3,
    resources: RenderResources<'_>,
) -> Color {
    let hit_point = ray.origin + ray.direction * hit.distance;
    let Some(direction) = refract(ray.direction, hit.normal, material.refraction_index) else {
        return resources.skybox.sample(ray.direction);
    };
    let refracted_ray = Ray::new(hit_point + direction * 0.002, direction);
    // El agua ya es una superficie visual delgada. Buscar directamente el
    // primer objeto opaco conserva la desviacion de Snell y evita volver a
    // recorrer la misma interfaz dos veces por pixel.
    closest_hit_filtered(&refracted_ray, scene, 0.001, true)
        .map(|(behind_hit, behind_material)| {
            shade(&refracted_ray, behind_hit, behind_material, light_direction)
        })
        .unwrap_or_else(|| resources.skybox.sample(direction))
}

/// Snell: desvía el rayo al entrar o salir de un material transparente.
fn refract(direction: Vec3, normal: Vec3, material_index: f32) -> Option<Vec3> {
    let mut interface_normal = normal;
    let (from_index, to_index) = if direction.dot(normal) < 0.0 {
        (1.0, material_index)
    } else {
        interface_normal = normal * -1.0;
        (material_index, 1.0)
    };
    let ratio = from_index / to_index;
    let cosine = (-direction.dot(interface_normal)).clamp(0.0, 1.0);
    let perpendicular_squared = ratio * ratio * (1.0 - cosine * cosine);
    if perpendicular_squared > 1.0 {
        return None;
    }
    Some(
        (direction * ratio
            + interface_normal * (ratio * cosine - (1.0 - perpendicular_squared).sqrt()))
        .normalize_or_zero(),
    )
}

fn shade(ray: &Ray, hit: Hit, material: Material, light_direction: Vec3) -> Color {
    shade_with_shadow(ray, hit, material, light_direction, false)
}

fn shade_with_shadow(
    ray: &Ray,
    hit: Hit,
    material: Material,
    light_direction: Vec3,
    shadowed: bool,
) -> Color {
    if material.finish == Finish::Unlit {
        return material.albedo;
    }

    let diffuse = if shadowed {
        0.0
    } else {
        cell_shade_light(hit.normal.dot(light_direction).max(0.0))
    };
    let sky_visibility = (hit.normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
    let ambient = AMBIENT_LIGHT + SKY_FILL_LIGHT * sky_visibility;
    let base = material
        .albedo
        .lit_with_exposure(ambient + diffuse * (1.0 - AMBIENT_LIGHT), EXPOSURE);
    if material.finish != Finish::Glossy {
        return base;
    }

    let reflected_light = hit.normal * (2.0 * hit.normal.dot(light_direction)) - light_direction;
    let highlight = reflected_light
        .dot(ray.direction * -1.0)
        .max(0.0)
        .powf(material.shininess)
        * material.specular_strength;
    blend(Color::new(255, 255, 255), base, highlight)
}

fn cell_shade_light(diffuse: f32) -> f32 {
    match diffuse {
        value if value < 0.16 => 0.0,
        value if value < 0.42 => 0.32,
        value if value < 0.72 => 0.66,
        _ => 1.0,
    }
}

fn is_shadowed(
    hit_point: Vec3,
    normal: Vec3,
    light_direction: Vec3,
    scene: &TraceScene<'_>,
) -> bool {
    let shadow_ray = Ray::new(hit_point + normal * 0.002, light_direction);
    forest_shadow_hit(&shadow_ray, scene)
}

fn forest_shadow_hit(ray: &Ray, scene: &TraceScene<'_>) -> bool {
    let Some(root) = scene.shadow_root else {
        return false;
    };
    let mut stack = [0_usize; 64];
    let mut stack_length = 1;
    stack[0] = root;

    while stack_length > 0 {
        stack_length -= 1;
        let node = &scene.shadow_nodes[stack[stack_length]];
        if ray_bounds_near(ray, node.bounds, f32::INFINITY).is_none() {
            continue;
        }

        if node.count > 0 {
            for bounded in &scene.shadow_objects[node.start..node.start + node.count] {
                let SceneObject::ForestProp(index) = bounded.object else {
                    continue;
                };
                if scene.forest_props[index].intersect_shadow(ray) {
                    return true;
                }
            }
            continue;
        }

        stack[stack_length] = node.left;
        stack[stack_length + 1] = node.right;
        stack_length += 2;
    }
    false
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
    use std::{
        f32::consts::FRAC_PI_4,
        time::{Duration, Instant},
    };

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
        let resources = RenderResources { skybox: &skybox };

        render(&mut framebuffer, &camera, &cubes, &[], &[], &[], resources);

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
        let ray = Ray::new(Vec3::default(), Vec3::new(0.0, 0.0, 1.0));
        let color = shade(
            &ray,
            hit,
            Material::unlit(Color::new(255, 255, 255)),
            Vec3::new(0.0, 1.0, 0.0),
        );

        assert_eq!(color.to_hex(), 0xFFFFFF);
    }

    #[test]
    fn ambient_and_sky_fill_keep_shadowed_surfaces_readable() {
        let hit = Hit {
            distance: 1.0,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        let ray = Ray::new(Vec3::default(), Vec3::new(0.0, -1.0, 0.0));
        let color = shade(
            &ray,
            hit,
            Material::matte(Color::new(100, 120, 80)),
            Vec3::new(0.0, -1.0, 0.0),
        );

        assert!(color.r >= 45 && color.g >= 55 && color.b >= 35);
        assert!(color.r < 100 && color.g < 120 && color.b < 80);
    }

    #[test]
    fn cell_shading_uses_four_distinct_light_bands() {
        assert_eq!(cell_shade_light(0.0), 0.0);
        assert_eq!(cell_shade_light(0.20), 0.32);
        assert_eq!(cell_shade_light(0.60), 0.66);
        assert_eq!(cell_shade_light(0.90), 1.0);
    }

    #[test]
    fn occluders_cast_a_shadow_toward_the_light() {
        let forest_props = [ForestProp::new(
            crate::world::ForestObstacleKind::Tree,
            Vec3::default(),
            Color::new(80, 80, 80),
        )];
        let hit = Hit {
            distance: 1.0,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        let ray = Ray::new(Vec3::default(), Vec3::new(0.0, -1.0, 0.0));
        let light = Vec3::new(0.0, 1.0, 0.0);
        let scene = TraceScene::new(&[], &[], &[], &forest_props);

        assert!(is_shadowed(Vec3::default(), hit.normal, light, &scene,));
        let lit = shade_with_shadow(
            &ray,
            hit,
            Material::matte(Color::new(100, 120, 80)),
            light,
            false,
        );
        let shadowed = shade_with_shadow(
            &ray,
            hit,
            Material::matte(Color::new(100, 120, 80)),
            light,
            true,
        );
        assert!(shadowed.r < lit.r && shadowed.g < lit.g && shadowed.b < lit.b);
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
        let resources = RenderResources { skybox: &skybox };
        let scene = TraceScene::new(&cubes, &[], &[], &[]);

        let color = trace_primary(&ray, &scene, Vec3::new(0.0, 0.0, 1.0), resources);

        assert_eq!(color.to_hex(), 0x7F007F);
    }

    #[test]
    fn water_refraction_bends_a_ray_toward_the_surface_normal() {
        let incoming = Vec3::new(0.6, -0.8, 0.0);
        let refracted = refract(incoming, Vec3::new(0.0, 1.0, 0.0), 1.333).unwrap();

        assert!(refracted.x.abs() < incoming.x.abs());
        assert!(refracted.y < incoming.y);
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
        let resources = RenderResources { skybox: &skybox };
        let scene = TraceScene::new(&cubes, &[], &[], &[]);

        let color = trace_primary(&ray, &scene, Vec3::new(0.0, 1.0, 0.0), resources);

        assert!(color.r < 200);
        assert!(color.g > 0);
        assert!(color.b > 0);
    }

    #[test]
    #[ignore = "medición manual de rendimiento a la resolución interna"]
    fn measures_quality_render_buffer_options() {
        let game = crate::game::Game::new();
        let scene = crate::scene::build_scene(&game);
        let camera = OrbitCamera::new(Vec3::new(8.5, 8.5, 11.5), Vec3::default(), FRAC_PI_4);
        let skybox = Skybox::solid(Color::new(120, 170, 220));
        let resources = RenderResources { skybox: &skybox };
        for preset in crate::display::DisplayPreset::ALL {
            let (width, height) = preset.render_size();
            let mut framebuffer = Framebuffer::new(width, height);
            let mut diorama_effect = crate::ui::DioramaEffect::new(width, height);

            // El primer frame estabiliza el pool y las cachés. Medir varios
            // frames permite detectar picos, no solo un promedio favorable.
            render(
                &mut framebuffer,
                &camera,
                &scene.cubes,
                &scene.spheres,
                &scene.cylinders,
                &scene.forest_props,
                resources,
            );
            diorama_effect.apply(&mut framebuffer.color);
            let mut samples = Vec::with_capacity(12);
            for _ in 0..12 {
                let started = Instant::now();
                render(
                    &mut framebuffer,
                    &camera,
                    &scene.cubes,
                    &scene.spheres,
                    &scene.cylinders,
                    &scene.forest_props,
                    resources,
                );
                diorama_effect.apply(&mut framebuffer.color);
                samples.push(started.elapsed());
            }
            samples.sort_unstable();
            let average = samples.iter().sum::<Duration>() / samples.len() as u32;
            let percentile_95 = samples[samples.len() * 95 / 100];
            println!(
                "buffer {} {width}x{height}: avg {:.1} ms ({:.1} FPS), p95 {:.1} ms",
                preset.label(),
                average.as_secs_f32() * 1_000.0,
                1.0 / average.as_secs_f32(),
                percentile_95.as_secs_f32() * 1_000.0,
            );
            assert!(average < Duration::from_secs(1));
        }

        for (label, cubes, spheres, cylinders, forest_props) in [
            (
                "completa",
                scene.cubes.as_slice(),
                scene.spheres.as_slice(),
                scene.cylinders.as_slice(),
                scene.forest_props.as_slice(),
            ),
            (
                "sin props",
                scene.cubes.as_slice(),
                scene.spheres.as_slice(),
                scene.cylinders.as_slice(),
                &[],
            ),
            ("solo cubos", scene.cubes.as_slice(), &[], &[], &[]),
            (
                "sin cubos",
                &[],
                scene.spheres.as_slice(),
                scene.cylinders.as_slice(),
                scene.forest_props.as_slice(),
            ),
        ] {
            let mut framebuffer = Framebuffer::new(800, 600);
            render(
                &mut framebuffer,
                &camera,
                cubes,
                spheres,
                cylinders,
                forest_props,
                resources,
            );
            let started = Instant::now();
            for _ in 0..2 {
                render(
                    &mut framebuffer,
                    &camera,
                    cubes,
                    spheres,
                    cylinders,
                    forest_props,
                    resources,
                );
            }
            let average = started.elapsed() / 2;
            println!(
                "{label} 800x600: {:.1} ms ({:.1} FPS)",
                average.as_secs_f32() * 1_000.0,
                1.0 / average.as_secs_f32()
            );
        }
    }
}
