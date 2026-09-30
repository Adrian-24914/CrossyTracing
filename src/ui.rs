use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub struct UiAssets {
    intro: Option<Bitmap>,
    death: Option<Bitmap>,
}

impl UiAssets {
    pub fn load() -> Self {
        let project = Path::new(env!("CARGO_MANIFEST_DIR"));
        let ui_root = project.join("assets/ui");
        ensure_runtime_bitmap(project, &ui_root, "intro", 480, 300);
        ensure_runtime_bitmap(project, &ui_root, "death", 600, 120);
        let root = ui_root.join("runtime");
        Self {
            intro: Bitmap::load_bmp(&root.join("intro.bmp")),
            death: Bitmap::load_bmp(&root.join("death.bmp")),
        }
    }
}

fn ensure_runtime_bitmap(project: &Path, ui_root: &Path, name: &str, width: u32, height: u32) {
    let Some(source) = find_source(ui_root, name) else {
        return;
    };
    let destination = ui_root.join("runtime").join(format!("{name}.bmp"));
    if is_current(&source, &destination) {
        return;
    }
    if let Some(parent) = destination.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("No se pudo crear {}: {error}", parent.display());
            return;
        }
    }

    let converter = project.join("tools/convert_ui_image.ps1");
    let converted = ["powershell.exe", "pwsh.exe"].into_iter().any(|shell| {
        Command::new(shell)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&converter)
            .arg("-Source")
            .arg(&source)
            .arg("-Destination")
            .arg(&destination)
            .arg("-MaximumWidth")
            .arg(width.to_string())
            .arg("-MaximumHeight")
            .arg(height.to_string())
            .status()
            .is_ok_and(|status| status.success())
    });
    if !converted {
        eprintln!("No se pudo convertir {} a BMP", source.display());
    }
}

fn find_source(root: &Path, name: &str) -> Option<PathBuf> {
    ["png", "jpg", "jpeg", "bmp"]
        .into_iter()
        .map(|extension| root.join(format!("{name}.{extension}")))
        .find(|path| path.is_file())
}

fn is_current(source: &Path, destination: &Path) -> bool {
    let Ok(source_time) = fs::metadata(source).and_then(|metadata| metadata.modified()) else {
        return false;
    };
    let is_alpha_bitmap = fs::read(destination)
        .ok()
        .is_some_and(|bytes| read_u16(&bytes, 28) == Some(32));
    is_alpha_bitmap
        && fs::metadata(destination)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|destination_time| destination_time >= source_time)
}

pub fn blur(pixels: &mut [u32], width: usize, height: usize) {
    let mut source = pixels.to_vec();
    for _ in 0..3 {
        for y in 0..height {
            for x in 0..width {
                let mut red = 0_u32;
                let mut green = 0_u32;
                let mut blue = 0_u32;
                let mut samples = 0_u32;
                for sample_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for sample_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        let color = source[sample_y * width + sample_x];
                        red += color >> 16 & 0xFF;
                        green += color >> 8 & 0xFF;
                        blue += color & 0xFF;
                        samples += 1;
                    }
                }
                pixels[y * width + x] =
                    (red / samples) << 16 | (green / samples) << 8 | (blue / samples);
            }
        }
        source.copy_from_slice(pixels);
    }
}

pub fn draw_intro(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    assets: &UiAssets,
    progress: f32,
) {
    let progress = progress.clamp(0.0, 1.0);
    if let Some(image) = &assets.intro {
        let center_y = (height as i32 - image.height as i32) / 2;
        let exit_distance = center_y + image.height as i32;
        let eased = progress * progress * (3.0 - 2.0 * progress);
        draw_image_centered(
            pixels,
            width,
            height,
            image,
            center_y - (exit_distance as f32 * eased) as i32,
            1.0,
        );
    } else {
        let center_y = height as i32 / 2 - 44;
        let exit_distance = center_y + 58 + 7 * 7;
        let eased = progress * progress * (3.0 - 2.0 * progress);
        let y = center_y - (exit_distance as f32 * eased) as i32;
        draw_text_centered(pixels, width, height, "CROSSY", y, 9, 0xF3E6C8, 1.0);
        draw_text_centered(pixels, width, height, "TRACING", y + 58, 7, 0x9CD49B, 1.0);
    }
}

pub fn draw_death(pixels: &mut [u32], width: usize, height: usize, assets: &UiAssets, fade: f32) {
    let fade = fade.clamp(0.0, 1.0);
    let band_height = (height as f32 * 0.24) as usize;
    let top = (height - band_height) / 2;
    for y in top..top + band_height {
        for x in 0..width {
            let index = y * width + x;
            pixels[index] = blend(pixels[index], 0x16070A, 0.72);
        }
    }

    if let Some(image) = &assets.death {
        draw_image_centered(
            pixels,
            width,
            height,
            image,
            (height as i32 - image.height as i32) / 2,
            fade,
        );
    } else {
        draw_text_centered(
            pixels,
            width,
            height,
            "YOU DIED",
            height as i32 / 2 - 26,
            8,
            0xB52B2B,
            0.94 * fade,
        );
    }
    draw_text_centered(
        pixels,
        width,
        height,
        "PRESS R TO RESTART",
        (top + band_height + 18) as i32,
        3,
        0xE6D7CB,
        0.78 * fade,
    );
}

fn draw_image_centered(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    image: &Bitmap,
    top: i32,
    opacity: f32,
) {
    let left = (width as i32 - image.width as i32) / 2;
    for image_y in 0..image.height {
        let target_y = top + image_y as i32;
        if !(0..height as i32).contains(&target_y) {
            continue;
        }
        for image_x in 0..image.width {
            let target_x = left + image_x as i32;
            if !(0..width as i32).contains(&target_x) {
                continue;
            }
            let source = image.pixels[image_y * image.width + image_x];
            let alpha = (source >> 24) as f32 / 255.0;
            if alpha > 0.0 {
                let index = target_y as usize * width + target_x as usize;
                pixels[index] = blend(pixels[index], source & 0x00FF_FFFF, opacity * alpha);
            }
        }
    }
}

fn draw_text_centered(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    text: &str,
    top: i32,
    scale: usize,
    color: u32,
    opacity: f32,
) {
    let text_width = text.chars().count() * 6 * scale;
    let left = (width as i32 - text_width as i32) / 2;
    for (character_index, character) in text.chars().enumerate() {
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                for offset_y in 0..scale {
                    for offset_x in 0..scale {
                        let x =
                            left + (character_index * 6 * scale + column * scale + offset_x) as i32;
                        let y = top + (row * scale + offset_y) as i32;
                        if (0..width as i32).contains(&x) && (0..height as i32).contains(&y) {
                            let index = y as usize * width + x as usize;
                            pixels[index] = blend(pixels[index], color, opacity);
                        }
                    }
                }
            }
        }
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        ' ' => [0; 7],
        _ => [0; 7],
    }
}

fn blend(background: u32, foreground: u32, opacity: f32) -> u32 {
    let opacity = opacity.clamp(0.0, 1.0);
    let channel = |shift| {
        (((background >> shift & 0xFF_u32) as f32 * (1.0 - opacity)
            + (foreground >> shift & 0xFF_u32) as f32 * opacity) as u32)
            << shift
    };
    channel(16) | channel(8) | channel(0)
}

struct Bitmap {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Bitmap {
    fn load_bmp(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        if bytes.len() < 54 || &bytes[..2] != b"BM" {
            return None;
        }
        let pixel_offset = read_u32(&bytes, 10)? as usize;
        let width = read_i32(&bytes, 18)?;
        let signed_height = read_i32(&bytes, 22)?;
        if width <= 0
            || signed_height == 0
            || !matches!(read_u16(&bytes, 28)?, 24 | 32)
            || read_u32(&bytes, 30)? != 0
        {
            return None;
        }
        let width = width as usize;
        let height = signed_height.unsigned_abs() as usize;
        let bits_per_pixel = read_u16(&bytes, 28)? as usize;
        let bytes_per_pixel = bits_per_pixel / 8;
        let stride = (width * bytes_per_pixel).div_ceil(4) * 4;
        if pixel_offset + stride * height > bytes.len() {
            return None;
        }
        let bottom_up = signed_height > 0;
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            let source_y = if bottom_up { height - 1 - y } else { y };
            let start = pixel_offset + source_y * stride;
            for x in 0..width {
                let pixel = start + x * bytes_per_pixel;
                let alpha = if bytes_per_pixel == 4 {
                    bytes[pixel + 3]
                } else {
                    0xFF
                };
                pixels.push(
                    (alpha as u32) << 24
                        | (bytes[pixel + 2] as u32) << 16
                        | (bytes[pixel + 1] as u32) << 8
                        | bytes[pixel] as u32,
                );
            }
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_averages_adjacent_pixels() {
        let mut pixels = vec![0; 9];
        pixels[4] = 0xFFFFFF;
        blur(&mut pixels, 3, 3);
        assert!(pixels[0] > 0 && pixels[4] < 0xFFFFFF);
    }

    #[test]
    fn blend_respects_full_opacity() {
        assert_eq!(blend(0x123456, 0xABCDEF, 1.0), 0xABCDEF);
    }

    #[test]
    fn glyph_supports_the_restart_prompt() {
        assert_ne!(glyph('P'), [0; 7]);
    }
}
