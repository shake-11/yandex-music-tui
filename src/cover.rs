//! Album art, decoded once and drawn portably as half-block cells.
//!
//! Kitty and Sixel protocols are optional in terminals, and half-block art
//! degrades predictably everywhere: two vertical pixels per character, true
//! colour foreground and background. A generated abstract cover also means the
//! now-playing screen is never empty when the network is down.

use std::hash::{Hash, Hasher};

/// A small RGB image kept in memory for the currently playing track.
#[derive(Debug, Clone)]
pub struct CoverImage {
    pub width: u32,
    pub height: u32,
    pixels: Vec<[u8; 3]>,
}

impl CoverImage {
    /// Decode JPEG/PNG/WebP bytes and shrink to a size useful for a terminal.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory(bytes)
            .map_err(|err| format!("cover image could not be decoded: {err}"))?;
        let image = image.thumbnail(512, 512).to_rgb8();
        let (width, height) = image.dimensions();
        let pixels = image.pixels().map(|pixel| pixel.0).collect();
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Sample an image coordinate, clamped to the image bounds.
    #[must_use]
    pub fn sample(&self, x: u32, y: u32) -> [u8; 3] {
        if self.width == 0 || self.height == 0 {
            return [0, 0, 0];
        }
        let x = x.min(self.width - 1);
        let y = y.min(self.height - 1);
        self.pixels[(y * self.width + x) as usize]
    }

    /// Generate deterministic art for a track or album title.
    ///
    /// The result is intentionally abstract: gradients, a soft halo, and vinyl
    /// grooves in a restrained palette. It is not an attempt to replace real
    /// cover art, only a graceful fallback that still looks designed.
    #[must_use]
    pub fn procedural(seed: &str) -> Self {
        const SIZE: u32 = 320;

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();
        let palette = palette((hash % 4) as usize);
        let mut pixels = Vec::with_capacity((SIZE * SIZE) as usize);

        let phase = ((hash >> 8) & 0xffff) as f32 / 65535.0 * std::f32::consts::TAU;
        let cx = 0.32 + ((hash >> 16) & 0xff) as f32 / 255.0 * 0.30;
        let cy = 0.30 + ((hash >> 24) & 0xff) as f32 / 255.0 * 0.26;

        for y in 0..SIZE {
            for x in 0..SIZE {
                let u = x as f32 / SIZE as f32;
                let v = y as f32 / SIZE as f32;
                let dx = u - cx;
                let dy = v - cy;
                let distance = (dx * dx + dy * dy).sqrt();

                // Deep vertical gradient.
                let mix = (v * 0.62 + u * 0.10).clamp(0.0, 1.0);
                let mut colour = blend(palette.base, palette.shade, mix);

                // A large, soft coloured sun.
                let halo = (1.0 - (distance * 2.35).clamp(0.0, 1.0)).powf(2.1);
                colour = add(colour, scale(palette.accent, halo * 0.72));

                // Wide diagonal bands cut gently across the composition.
                let wave = ((u * 4.1 + v * 2.2 + phase).sin() * 0.5 + 0.5).powf(7.0);
                let edge = (1.0 - (v - 0.74).abs() * 8.0).clamp(0.0, 1.0);
                colour = add(colour, scale(palette.secondary, wave * edge * 0.42));

                // Fine rings, like a record or a topographic map.
                let ring = (1.0 - (((distance * 22.0).fract() - 0.5).abs() * 9.0)).clamp(0.0, 1.0);
                colour = add(colour, scale(palette.highlight, ring * 0.045));

                // Typographic-feeling bar near the lower edge.
                if (0.12..0.74).contains(&u) && (0.82..0.855).contains(&v) {
                    colour = blend(colour, palette.highlight, 0.74);
                }

                // Deterministic grain keeps large gradients from banding.
                let noise = {
                    let mut n = (x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263))
                        .wrapping_add(hash as u32);
                    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
                    ((n ^ (n >> 16)) & 0xff) as i16 - 127
                };
                let noise = (noise as f32 / 127.0 * 3.0) as i16;
                colour = [
                    (colour[0] as i16 + noise).clamp(0, 255) as u8,
                    (colour[1] as i16 + noise).clamp(0, 255) as u8,
                    (colour[2] as i16 + noise).clamp(0, 255) as u8,
                ];

                pixels.push(colour);
            }
        }

        Self {
            width: SIZE,
            height: SIZE,
            pixels,
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    base: [u8; 3],
    shade: [u8; 3],
    accent: [u8; 3],
    secondary: [u8; 3],
    highlight: [u8; 3],
}

fn palette(index: usize) -> Palette {
    match index {
        0 => Palette {
            base: [12, 14, 23],
            shade: [34, 20, 34],
            accent: [255, 191, 82],
            secondary: [223, 88, 93],
            highlight: [247, 239, 214],
        },
        1 => Palette {
            base: [8, 22, 22],
            shade: [13, 44, 42],
            accent: [103, 229, 192],
            secondary: [183, 241, 112],
            highlight: [238, 255, 225],
        },
        2 => Palette {
            base: [16, 12, 13],
            shade: [42, 25, 20],
            accent: [255, 210, 70],
            secondary: [173, 71, 92],
            highlight: [255, 246, 218],
        },
        _ => Palette {
            base: [12, 14, 26],
            shade: [21, 31, 55],
            accent: [255, 219, 98],
            secondary: [95, 159, 255],
            highlight: [224, 237, 255],
        },
    }
}

fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

fn add(a: [u8; 3], b: [f32; 3]) -> [u8; 3] {
    [
        (a[0] as f32 + b[0]).clamp(0.0, 255.0) as u8,
        (a[1] as f32 + b[1]).clamp(0.0, 255.0) as u8,
        (a[2] as f32 + b[2]).clamp(0.0, 255.0) as u8,
    ]
}

fn scale(colour: [u8; 3], factor: f32) -> [f32; 3] {
    [
        colour[0] as f32 * factor,
        colour[1] as f32 * factor,
        colour[2] as f32 * factor,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_art_is_deterministic_and_in_bounds() {
        let a = CoverImage::procedural("night drive");
        let b = CoverImage::procedural("night drive");
        assert_eq!(a.width, b.width);
        assert_eq!(a.sample(0, 0), b.sample(0, 0));
        assert_eq!(
            a.sample(u32::MAX, u32::MAX),
            a.sample(a.width - 1, a.height - 1)
        );
    }

    #[test]
    fn different_seeds_generally_differ() {
        let a = CoverImage::procedural("one");
        let b = CoverImage::procedural("two");
        assert_ne!(a.sample(20, 20), b.sample(20, 20));
    }
}
