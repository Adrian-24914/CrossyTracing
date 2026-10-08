use crate::{color::Color, math::Vec3};
#[cfg(not(target_arch = "wasm32"))]
use std::{fs, path::Path};

pub struct Skybox {
    positive_x: Image,
    negative_x: Image,
    positive_y: Image,
    negative_y: Image,
    positive_z: Image,
    negative_z: Image,
}

impl Skybox {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Result<Self, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/skybox/runtime");
        Ok(Self {
            positive_x: Image::load_bmp(&root.join("posx.bmp"))?,
            negative_x: Image::load_bmp(&root.join("negx.bmp"))?,
            positive_y: Image::load_bmp(&root.join("posy.bmp"))?,
            negative_y: Image::load_bmp(&root.join("negy.bmp"))?,
            positive_z: Image::load_bmp(&root.join("posz.bmp"))?,
            negative_z: Image::load_bmp(&root.join("negz.bmp"))?,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            positive_x: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/posx.bmp"))?,
            negative_x: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/negx.bmp"))?,
            positive_y: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/posy.bmp"))?,
            negative_y: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/negy.bmp"))?,
            positive_z: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/posz.bmp"))?,
            negative_z: Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/negz.bmp"))?,
        })
    }

    pub fn sample(&self, direction: Vec3) -> Color {
        let absolute_x = direction.x.abs();
        let absolute_y = direction.y.abs();
        let absolute_z = direction.z.abs();

        let (face, horizontal, vertical, dominant) =
            if absolute_x >= absolute_y && absolute_x >= absolute_z {
                if direction.x >= 0.0 {
                    (&self.positive_x, -direction.z, -direction.y, absolute_x)
                } else {
                    (&self.negative_x, direction.z, -direction.y, absolute_x)
                }
            } else if absolute_y >= absolute_z {
                if direction.y >= 0.0 {
                    (&self.positive_y, direction.x, direction.z, absolute_y)
                } else {
                    (&self.negative_y, direction.x, -direction.z, absolute_y)
                }
            } else if direction.z >= 0.0 {
                (&self.positive_z, direction.x, -direction.y, absolute_z)
            } else {
                (&self.negative_z, -direction.x, -direction.y, absolute_z)
            };

        if dominant <= f32::EPSILON {
            return Color::new(0, 0, 0);
        }
        let u = (horizontal / dominant + 1.0) * 0.5;
        let v = (vertical / dominant + 1.0) * 0.5;
        face.sample(u, v)
    }

    #[cfg(test)]
    pub fn solid(color: Color) -> Self {
        Self {
            positive_x: Image::solid(color),
            negative_x: Image::solid(color),
            positive_y: Image::solid(color),
            negative_y: Image::solid(color),
            positive_z: Image::solid(color),
            negative_z: Image::solid(color),
        }
    }
}

struct Image {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl Image {
    #[cfg(not(target_arch = "wasm32"))]
    fn load_bmp(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Self::from_bmp_bytes(&bytes).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn from_bmp_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 54 || &bytes[..2] != b"BM" {
            return Err("no es un BMP válido".to_string());
        }

        let pixel_offset = read_u32(bytes, 10)? as usize;
        let width = read_i32(bytes, 18)?;
        let signed_height = read_i32(bytes, 22)?;
        let planes = read_u16(bytes, 26)?;
        let bits_per_pixel = read_u16(bytes, 28)?;
        let compression = read_u32(bytes, 30)?;
        if width <= 0
            || signed_height == 0
            || planes != 1
            || !matches!(bits_per_pixel, 24 | 32)
            || compression != 0
        {
            return Err("usa un formato BMP no soportado".to_string());
        }

        let width = width as usize;
        let height = signed_height.unsigned_abs() as usize;
        let bytes_per_pixel = (bits_per_pixel / 8) as usize;
        let row_stride = (width * bytes_per_pixel).div_ceil(4) * 4;
        let required = pixel_offset + row_stride * height;
        if required > bytes.len() {
            return Err("está truncado".to_string());
        }

        let bottom_up = signed_height > 0;
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            let source_y = if bottom_up { height - 1 - y } else { y };
            let row_start = pixel_offset + source_y * row_stride;
            for x in 0..width {
                let pixel = row_start + x * bytes_per_pixel;
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
        let x = u.clamp(0.0, 1.0) * (self.width - 1) as f32;
        let y = v.clamp(0.0, 1.0) * (self.height - 1) as f32;
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let horizontal = x - x0 as f32;
        let vertical = y - y0 as f32;

        let top = mix(self.pixel(x0, y0), self.pixel(x1, y0), horizontal);
        let bottom = mix(self.pixel(x0, y1), self.pixel(x1, y1), horizontal);
        mix(top, bottom, vertical)
    }

    fn pixel(&self, x: usize, y: usize) -> Color {
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

fn mix(first: Color, second: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let channel =
        |first: u8, second: u8| (first as f32 * (1.0 - amount) + second as f32 * amount) as u8;
    Color::new(
        channel(first.r, second.r),
        channel(first.g, second.g),
        channel(first.b, second.b),
    )
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
    fn loads_the_six_runtime_faces() {
        let skybox = Skybox::load().unwrap();
        assert_eq!(skybox.positive_x.width, 512);
        assert_eq!(skybox.negative_x.height, 512);
        assert_eq!(skybox.positive_y.pixels.len(), 512 * 512);
        assert_eq!(skybox.negative_y.pixels.len(), 512 * 512);
        assert_eq!(skybox.positive_z.pixels.len(), 512 * 512);
        assert_eq!(skybox.negative_z.pixels.len(), 512 * 512);
    }

    #[test]
    fn parses_a_face_directly_from_bmp_bytes() {
        let image =
            Image::from_bmp_bytes(include_bytes!("../assets/skybox/runtime/posx.bmp")).unwrap();
        assert_eq!(image.width, 512);
        assert_eq!(image.height, 512);
        assert_eq!(image.pixels.len(), 512 * 512);
    }

    #[test]
    fn cardinal_directions_select_the_expected_faces() {
        let skybox = Skybox::load().unwrap();
        assert_eq!(
            skybox.sample(Vec3::new(1.0, 0.0, 0.0)).to_hex(),
            skybox.positive_x.sample(0.5, 0.5).to_hex()
        );
        assert_eq!(
            skybox.sample(Vec3::new(0.0, -1.0, 0.0)).to_hex(),
            skybox.negative_y.sample(0.5, 0.5).to_hex()
        );
        assert_eq!(
            skybox.sample(Vec3::new(0.0, 0.0, -1.0)).to_hex(),
            skybox.negative_z.sample(0.5, 0.5).to_hex()
        );
    }
}
