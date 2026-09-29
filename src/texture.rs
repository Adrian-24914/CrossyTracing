use crate::{color::Color, material::TextureKind, math::Vec3};
use std::{fs, path::Path};

const NORMAL_STRENGTH: f32 = 1.05;
const TEXTURE_ZOOM: f32 = 0.52;
const PARALLAX_DEPTH: f32 = 0.055;
const REFERENCE_GRASS_BRIGHTNESS: f32 = 122.0;

#[derive(Clone, Copy)]
pub struct Surface {
    pub albedo: Color,
    pub normal: Vec3,
    pub ambient_occlusion: f32,
    pub roughness: f32,
}

pub struct Textures {
    ground_grass: TextureSet,
}

impl Textures {
    pub fn load() -> Result<Self, String> {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/textures/ground_grass/runtime");
        Ok(Self {
            ground_grass: TextureSet {
                base_color: Bitmap::load_bmp(&root.join("basecolor.bmp"))?,
                normal: Bitmap::load_bmp(&root.join("normal_ogl.bmp"))?,
                properties: Bitmap::load_bmp(&root.join("properties.bmp"))?,
            },
        })
    }

    pub fn sample(
        &self,
        texture: TextureKind,
        tint: Color,
        u: f32,
        v: f32,
        geometric_normal: Vec3,
        ray_direction: Vec3,
    ) -> Surface {
        match texture {
            TextureKind::None => Surface {
                albedo: tint,
                normal: geometric_normal,
                ambient_occlusion: 1.0,
                roughness: 0.0,
            },
            TextureKind::GroundGrass => {
                self.ground_grass
                    .sample(tint, u, v, geometric_normal, ray_direction)
            }
        }
    }

    #[cfg(test)]
    pub fn flat() -> Self {
        Self {
            ground_grass: TextureSet {
                base_color: Bitmap::solid(Color::new(100, 120, 80)),
                normal: Bitmap::solid(Color::new(128, 128, 255)),
                properties: Bitmap::solid(Color::new(255, 220, 128)),
            },
        }
    }
}

struct TextureSet {
    base_color: Bitmap,
    normal: Bitmap,
    properties: Bitmap,
}

impl TextureSet {
    fn sample(
        &self,
        tint: Color,
        u: f32,
        v: f32,
        geometric_normal: Vec3,
        ray_direction: Vec3,
    ) -> Surface {
        let mapping = Mapping::for_face(zoom_coordinate(u), zoom_coordinate(v), geometric_normal);
        let properties = self.properties.sample(mapping.u, mapping.v);
        let height = properties.b as f32 / 255.0;
        let mapping = parallax_mapping(mapping, geometric_normal, ray_direction, height);
        let base_color = self.base_color.sample(mapping.u, mapping.v);
        let normal = self.normal.sample(mapping.u, mapping.v);

        Surface {
            albedo: adjust_brightness(base_color, tint),
            normal: map_normal(normal, mapping, geometric_normal),
            ambient_occlusion: (0.58 + properties.r as f32 / 255.0 * 0.42) * (0.82 + height * 0.18),
            roughness: properties.g as f32 / 255.0,
        }
    }
}

fn zoom_coordinate(value: f32) -> f32 {
    0.5 + (value - 0.5) * TEXTURE_ZOOM
}

fn parallax_mapping(
    mut mapping: Mapping,
    geometric_normal: Vec3,
    ray_direction: Vec3,
    height: f32,
) -> Mapping {
    let view = ray_direction * -1.0;
    let normal_view = view.dot(geometric_normal).abs().max(0.25);
    let depth = (height - 0.5) * PARALLAX_DEPTH;
    mapping.u = (mapping.u - view.dot(mapping.tangent) / normal_view * depth).clamp(0.0, 0.999_999);
    mapping.v =
        (mapping.v - view.dot(mapping.bitangent) / normal_view * depth).clamp(0.0, 0.999_999);
    mapping
}

#[derive(Clone, Copy)]
struct Mapping {
    u: f32,
    v: f32,
    tangent: Vec3,
    bitangent: Vec3,
}

impl Mapping {
    fn for_face(u: f32, v: f32, normal: Vec3) -> Self {
        let x = normal.x.abs();
        let y = normal.y.abs();
        let z = normal.z.abs();
        let (tangent, bitangent) = if y >= x && y >= z {
            if normal.y >= 0.0 {
                (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0))
            } else {
                (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0))
            }
        } else if x >= z {
            if normal.x >= 0.0 {
                (Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, -1.0, 0.0))
            } else {
                (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0))
            }
        } else if normal.z >= 0.0 {
            (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
        } else {
            (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0))
        };

        Self {
            u,
            v,
            tangent,
            bitangent,
        }
    }
}

fn adjust_brightness(color: Color, tint: Color) -> Color {
    let brightness = tint.r as f32 * 0.21 + tint.g as f32 * 0.72 + tint.b as f32 * 0.07;
    let factor = (brightness / REFERENCE_GRASS_BRIGHTNESS).clamp(0.72, 1.28);
    let scale = |channel: u8| (channel as f32 * factor).clamp(0.0, 255.0) as u8;
    Color::new(scale(color.r), scale(color.g), scale(color.b))
}

fn map_normal(sample: Color, mapping: Mapping, geometric_normal: Vec3) -> Vec3 {
    let tangent_x = (sample.r as f32 / 255.0 * 2.0 - 1.0) * NORMAL_STRENGTH;
    let tangent_y = (sample.g as f32 / 255.0 * 2.0 - 1.0) * NORMAL_STRENGTH;
    let tangent_z = (sample.b as f32 / 255.0 * 2.0 - 1.0).max(0.15);
    (mapping.tangent * tangent_x + mapping.bitangent * tangent_y + geometric_normal * tangent_z)
        .normalize()
}

struct Bitmap {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl Bitmap {
    fn load_bmp(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        if bytes.len() < 54 || &bytes[..2] != b"BM" {
            return Err(format!("{} no es un BMP válido", path.display()));
        }

        let pixel_offset = read_u32(&bytes, 10)? as usize;
        let width = read_i32(&bytes, 18)?;
        let signed_height = read_i32(&bytes, 22)?;
        let planes = read_u16(&bytes, 26)?;
        let bits_per_pixel = read_u16(&bytes, 28)?;
        let compression = read_u32(&bytes, 30)?;
        if width <= 0
            || signed_height == 0
            || planes != 1
            || bits_per_pixel != 24
            || compression != 0
        {
            return Err(format!(
                "{} usa un formato BMP no soportado",
                path.display()
            ));
        }

        let width = width as usize;
        let height = signed_height.unsigned_abs() as usize;
        let row_stride = (width * 3).div_ceil(4) * 4;
        let required = pixel_offset + row_stride * height;
        if required > bytes.len() {
            return Err(format!("{} está truncado", path.display()));
        }

        let bottom_up = signed_height > 0;
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            let source_y = if bottom_up { height - 1 - y } else { y };
            let row_start = pixel_offset + source_y * row_stride;
            for x in 0..width {
                let pixel = row_start + x * 3;
                pixels.push(Color::new(bytes[pixel + 2], bytes[pixel + 1], bytes[pixel]));
            }
        }

        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    fn sample(&self, u: f32, v: f32) -> Color {
        let x = (u * self.width as f32) as usize % self.width;
        let y = (v * self.height as f32) as usize % self.height;
        self.pixels[y * self.width + x]
    }

    #[cfg(test)]
    fn solid(color: Color) -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![color],
        }
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "encabezado BMP incompleto".to_string())?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "encabezado BMP incompleto".to_string())?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_i32(bytes: &[u8], offset: usize) -> Result<i32, String> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "encabezado BMP incompleto".to_string())?;
    Ok(i32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_three_runtime_maps() {
        let textures = Textures::load().unwrap();
        assert_eq!(textures.ground_grass.base_color.width, 256);
        assert_eq!(textures.ground_grass.normal.height, 256);
        assert_eq!(textures.ground_grass.properties.pixels.len(), 256 * 256);
    }

    #[test]
    fn local_coordinates_sample_the_same_texel_after_movement() {
        let textures = Textures::load().unwrap();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let first = textures.sample(
            TextureKind::GroundGrass,
            Color::new(78, 145, 82),
            0.31,
            0.42,
            normal,
            Vec3::new(0.0, -1.0, 0.0),
        );
        let moved_cube = textures.sample(
            TextureKind::GroundGrass,
            Color::new(78, 145, 82),
            0.31,
            0.42,
            normal,
            Vec3::new(0.0, -1.0, 0.0),
        );

        assert_eq!(first.albedo.to_hex(), moved_cube.albedo.to_hex());
        assert!((first.normal.x - moved_cube.normal.x).abs() < 0.0001);
        assert!(first.normal.dot(normal) > 0.5);
        assert!((0.58..=1.0).contains(&first.ambient_occlusion));
        assert!((first.roughness - moved_cube.roughness).abs() < 0.0001);
    }

    #[test]
    fn untextured_surface_keeps_its_original_values() {
        let textures = Textures::flat();
        let albedo = Color::new(120, 80, 40);
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let surface = textures.sample(
            TextureKind::None,
            albedo,
            0.0,
            0.0,
            normal,
            Vec3::new(0.0, -1.0, 0.0),
        );

        assert_eq!(surface.albedo.to_hex(), albedo.to_hex());
        assert_eq!(surface.normal.y, 1.0);
        assert_eq!(surface.ambient_occlusion, 1.0);
        assert_eq!(surface.roughness, 0.0);
    }

    #[test]
    fn normal_map_visibly_tilts_the_geometric_normal() {
        let texture = TextureSet {
            base_color: Bitmap::solid(Color::new(100, 120, 80)),
            normal: Bitmap::solid(Color::new(255, 128, 255)),
            properties: Bitmap::solid(Color::new(255, 220, 128)),
        };
        let geometric_normal = Vec3::new(0.0, 1.0, 0.0);
        let surface = texture.sample(
            Color::new(78, 145, 82),
            0.5,
            0.5,
            geometric_normal,
            Vec3::new(0.0, -1.0, 0.0),
        );

        assert!(surface.normal.x > 0.6);
        assert!(surface.normal.dot(geometric_normal) < 0.8);
    }

    #[test]
    fn texture_zoom_uses_a_larger_detail_region() {
        assert!((zoom_coordinate(0.0) - 0.24).abs() < 0.0001);
        assert!((zoom_coordinate(1.0) - 0.76).abs() < 0.0001);
    }
}
