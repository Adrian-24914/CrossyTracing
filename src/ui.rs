use std::{fs, path::Path};

pub struct UiAssets {
    intro: Option<Bitmap>,
    death: Option<Bitmap>,
}

/// Efecto de maqueta ligero: mantiene una franja nítida y mezcla las zonas
/// lejanas con pocas muestras desenfocadas. No reserva memoria durante el frame.
pub struct DioramaEffect {
    width: usize,
    height: usize,
    radius: usize,
    horizontal: Vec<u32>,
    strengths: Vec<f32>,
    top_end: usize,
    bottom_start: usize,
}

impl DioramaEffect {
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0);
        let strengths: Vec<_> = (0..height).map(|y| diorama_strength(y, height)).collect();
        let middle = height / 2;
        let top_end = (0..middle).rev().find(|&y| strengths[y] > 0.0).unwrap_or(0);
        let bottom_start = (middle..height)
            .find(|&y| strengths[y] > 0.0)
            .unwrap_or(height - 1);

        Self {
            width,
            height,
            // El desenfoque se limita a los extremos: la franja central queda
            // nitida para conservar al personaje y producir el efecto maqueta.
            radius: 4,
            horizontal: vec![0; width * height],
            strengths,
            top_end,
            bottom_start,
        }
    }

    pub fn apply(&mut self, pixels: &mut [u32]) {
        assert_eq!(pixels.len(), self.width * self.height);
        let top_needed_end = (self.top_end + self.radius).min(self.height - 1);
        let bottom_needed_start = self.bottom_start.saturating_sub(self.radius);

        for y in 0..=top_needed_end {
            blur_horizontal_row(pixels, &mut self.horizontal, self.width, y, self.radius);
        }
        for y in bottom_needed_start.max(top_needed_end + 1)..self.height {
            blur_horizontal_row(pixels, &mut self.horizontal, self.width, y, self.radius);
        }

        apply_diorama_band(
            pixels,
            &self.horizontal,
            &self.strengths,
            self.width,
            self.height,
            0,
            self.top_end,
            self.radius,
        );
        apply_diorama_band(
            pixels,
            &self.horizontal,
            &self.strengths,
            self.width,
            self.height,
            self.bottom_start,
            self.height - 1,
            self.radius,
        );
    }
}

fn diorama_strength(y: usize, height: usize) -> f32 {
    let normalized_y = (y as f32 + 0.5) / height as f32;
    let distance = (normalized_y - 0.54).abs();
    let transition = ((distance - 0.16) / 0.22).clamp(0.0, 1.0);
    let smooth = transition * transition * (3.0 - 2.0 * transition);
    smooth * 0.72
}

fn blur_horizontal_row(source: &[u32], target: &mut [u32], width: usize, y: usize, radius: usize) {
    let row = y * width;
    let mut red = 0_u32;
    let mut green = 0_u32;
    let mut blue = 0_u32;
    let mut samples = 0_u32;
    for x in 0..=radius.min(width - 1) {
        add_channels(source[row + x], &mut red, &mut green, &mut blue);
        samples += 1;
    }

    for x in 0..width {
        target[row + x] = average_color(red, green, blue, samples);
        if x >= radius {
            subtract_channels(source[row + x - radius], &mut red, &mut green, &mut blue);
            samples -= 1;
        }
        let incoming = x + radius + 1;
        if incoming < width {
            add_channels(source[row + incoming], &mut red, &mut green, &mut blue);
            samples += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_diorama_band(
    pixels: &mut [u32],
    horizontal: &[u32],
    strengths: &[f32],
    width: usize,
    height: usize,
    start: usize,
    end: usize,
    radius: usize,
) {
    for y in start..=end {
        let strength = strengths[y];
        if strength <= 0.0 {
            continue;
        }
        let vertical_offset = ((radius as f32 * strength / 0.72).round() as usize).max(1);
        let upper_row = y.saturating_sub(vertical_offset) * width;
        let center_row = y * width;
        let lower_row = (y + vertical_offset).min(height - 1) * width;
        for x in 0..width {
            let upper = horizontal[upper_row + x];
            let center = horizontal[center_row + x];
            let lower = horizontal[lower_row + x];
            let red = (upper >> 16 & 0xFF) + (center >> 16 & 0xFF) + (lower >> 16 & 0xFF);
            let green = (upper >> 8 & 0xFF) + (center >> 8 & 0xFF) + (lower >> 8 & 0xFF);
            let blue = (upper & 0xFF) + (center & 0xFF) + (lower & 0xFF);
            let index = center_row + x;
            pixels[index] = blend(pixels[index], average_color(red, green, blue, 3), strength);
        }
    }
}

fn add_channels(color: u32, red: &mut u32, green: &mut u32, blue: &mut u32) {
    *red += color >> 16 & 0xFF;
    *green += color >> 8 & 0xFF;
    *blue += color & 0xFF;
}

fn subtract_channels(color: u32, red: &mut u32, green: &mut u32, blue: &mut u32) {
    *red -= color >> 16 & 0xFF;
    *green -= color >> 8 & 0xFF;
    *blue -= color & 0xFF;
}

fn average_color(red: u32, green: u32, blue: u32, samples: u32) -> u32 {
    (red / samples) << 16 | (green / samples) << 8 | blue / samples
}

impl UiAssets {
    pub fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/ui/runtime");
        Self {
            intro: Bitmap::load_bmp(&root.join("intro.bmp")),
            death: Bitmap::load_bmp(&root.join("death.bmp")),
        }
    }
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
        Self::from_bmp_bytes(&bytes)
    }

    fn from_bmp_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 54 || &bytes[..2] != b"BM" {
            return None;
        }
        let pixel_offset = read_u32(bytes, 10)? as usize;
        let width = read_i32(bytes, 18)?;
        let signed_height = read_i32(bytes, 22)?;
        if width <= 0
            || signed_height == 0
            || !matches!(read_u16(bytes, 28)?, 24 | 32)
            || read_u32(bytes, 30)? != 0
        {
            return None;
        }
        let width = width as usize;
        let height = signed_height.unsigned_abs() as usize;
        let bits_per_pixel = read_u16(bytes, 28)? as usize;
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
    fn parses_intro_directly_from_bmp_bytes() {
        let bitmap =
            Bitmap::from_bmp_bytes(include_bytes!("../assets/ui/runtime/intro.bmp")).unwrap();
        assert_eq!(bitmap.width, 480);
        assert_eq!(bitmap.height, 300);
        assert_eq!(bitmap.pixels.len(), 480 * 300);
    }

    #[test]
    fn blur_averages_adjacent_pixels() {
        let mut pixels = vec![0; 9];
        pixels[4] = 0xFFFFFF;
        blur(&mut pixels, 3, 3);
        assert!(pixels[0] > 0 && pixels[4] < 0xFFFFFF);
    }

    #[test]
    fn diorama_effect_keeps_the_focus_band_sharp_and_blurs_the_edges() {
        let width = 80;
        let height = 60;
        let mut pixels: Vec<_> = (0..width * height)
            .map(|index| if index % 2 == 0 { 0xFFFFFF } else { 0 })
            .collect();
        let original = pixels.clone();
        let mut effect = DioramaEffect::new(width, height);

        effect.apply(&mut pixels);

        let focus_y = (height as f32 * 0.54) as usize;
        assert_eq!(
            &pixels[focus_y * width..(focus_y + 1) * width],
            &original[focus_y * width..(focus_y + 1) * width]
        );
        assert_ne!(&pixels[..width], &original[..width]);
        assert_ne!(
            &pixels[(height - 1) * width..],
            &original[(height - 1) * width..]
        );
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
