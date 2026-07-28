use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

use super::RotContext;

pub(super) fn rot_image(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) {
    if width == 0 || height == 0 || rgba.is_empty() {
        return;
    }

    let reference = rgba.to_vec();
    let ghost_count = context.open_count.min(12) as usize;
    let ghost_alpha = 10 + ((context.q * 15 + 5_000) / 10_000) as u16;
    for _ in 0..ghost_count {
        let dx = random_offset(rng);
        let dy = random_offset(rng);
        let scale_pct = 99 + (rng.next_u32() % 3) as i32;
        composite_ghost(
            rgba,
            &reference,
            width as usize,
            height as usize,
            dx,
            dy,
            scale_pct,
            ghost_alpha,
        );
    }

    apply_contrast_crater(rgba, context.q);
    apply_sepia_shift(rgba, context.q);
    apply_vignette_and_hotspot(rgba, width as usize, height as usize, context.q, rng);
}

pub(super) fn rot_text(text: &str, context: &RotContext, rng: &mut ChaCha8Rng) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let first_line = lines.first().copied().unwrap_or("");
    let echo_count = (context.open_count / 3).min(8) as usize;

    let mut rendered = String::new();
    for _ in 0..echo_count {
        rendered.push('⌈');
        rendered.push(' ');
        rendered.push_str(first_line);
        rendered.push(' ');
        rendered.push('⌉');
        rendered.push('\n');
    }

    let lowercase_threshold = (context.q / 2).max(500);
    let fade_threshold = context.q.saturating_sub(5_500);
    for ch in text.chars() {
        if ch.is_whitespace() {
            rendered.push(ch);
            continue;
        }

        let sample = rng.next_u32() % 10_000;
        if fade_threshold > 0 && sample < fade_threshold {
            rendered.push('·');
        } else if sample < lowercase_threshold {
            rendered.extend(ch.to_lowercase());
        } else {
            rendered.push(ch);
        }
    }

    rendered
}

fn random_offset(rng: &mut ChaCha8Rng) -> i32 {
    let magnitude = 2 + (rng.next_u32() % 13) as i32;
    if rng.next_u32() & 1 == 0 {
        magnitude
    } else {
        -magnitude
    }
}

#[allow(clippy::too_many_arguments)]
fn composite_ghost(
    rgba: &mut [u8],
    reference: &[u8],
    width: usize,
    height: usize,
    dx: i32,
    dy: i32,
    scale_pct: i32,
    alpha: u16,
) {
    let center_x = (width / 2) as i32;
    let center_y = (height / 2) as i32;

    for y in 0..height {
        for x in 0..width {
            let shifted_x = x as i32 - dx - center_x;
            let shifted_y = y as i32 - dy - center_y;
            let src_x = center_x + shifted_x * 100 / scale_pct;
            let src_y = center_y + shifted_y * 100 / scale_pct;
            if src_x < 0 || src_y < 0 || src_x >= width as i32 || src_y >= height as i32 {
                continue;
            }

            let dest = (y * width + x) * 4;
            let src = (src_y as usize * width + src_x as usize) * 4;
            for channel in 0..3 {
                let current = rgba[dest + channel] as u16;
                let ghost = reference[src + channel] as u16;
                rgba[dest + channel] =
                    ((current * (256 - alpha) + ghost * alpha + 128) / 256) as u8;
            }
        }
    }
}

fn apply_contrast_crater(rgba: &mut [u8], q: u32) {
    let crater_strength = (q * 128 / 10_000) as u16;
    if crater_strength == 0 {
        return;
    }

    let mut total_luma = 0u64;
    let mut pixels = 0u64;
    for pixel in rgba.chunks_exact(4) {
        total_luma += luminance(pixel) as u64;
        pixels += 1;
    }
    if pixels == 0 {
        return;
    }
    let mean_luma = (total_luma / pixels) as u16;

    for pixel in rgba.chunks_exact_mut(4) {
        for channel in 0..3 {
            let value = pixel[channel] as u16;
            pixel[channel] =
                ((value * (256 - crater_strength) + mean_luma * crater_strength + 128) / 256) as u8;
        }
    }
}

fn apply_sepia_shift(rgba: &mut [u8], q: u32) {
    let sepia_strength = (q * 256 / 10_000) as u16;
    if sepia_strength == 0 {
        return;
    }

    for pixel in rgba.chunks_exact_mut(4) {
        let green_target = pixel[1] as u16 * 94 / 100;
        let blue_target = pixel[2] as u16 * 82 / 100;
        pixel[1] = ((pixel[1] as u16 * (256 - sepia_strength)
            + green_target * sepia_strength
            + 128)
            / 256) as u8;
        pixel[2] = ((pixel[2] as u16 * (256 - sepia_strength) + blue_target * sepia_strength + 128)
            / 256) as u8;
    }
}

fn apply_vignette_and_hotspot(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    q: u32,
    rng: &mut ChaCha8Rng,
) {
    let strength = q * 6_000 / 10_000;
    if strength == 0 {
        return;
    }

    let center_x = (width as i64 - 1) / 2;
    let center_y = (height as i64 - 1) / 2;
    let max_dist_sq = center_x.pow(2) + center_y.pow(2);
    if max_dist_sq == 0 {
        return;
    }

    let hotspot_x = (rng.next_u32() as usize % width) as i64;
    let hotspot_y = (rng.next_u32() as usize % height) as i64;
    let hotspot_radius = (width.max(height) / 5).max(12) as i64;
    let hotspot_radius_sq = hotspot_radius.pow(2);
    let hotspot_strength = 1_200 + q / 20;

    for y in 0..height {
        for x in 0..width {
            let dx = x as i64 - center_x;
            let dy = y as i64 - center_y;
            let dist_sq = dx * dx + dy * dy;
            let darken = (strength as u64 * dist_sq as u64 / max_dist_sq as u64) as u32;

            let hx = x as i64 - hotspot_x;
            let hy = y as i64 - hotspot_y;
            let hotspot_dist_sq = hx * hx + hy * hy;
            let brighten = if hotspot_dist_sq < hotspot_radius_sq {
                (hotspot_strength as u64 * (hotspot_radius_sq - hotspot_dist_sq) as u64
                    / hotspot_radius_sq as u64) as u32
            } else {
                0
            };

            let base = (y * width + x) * 4;
            for channel in 0..3 {
                let darkened = rgba[base + channel] as u32 * (10_000 - darken) / 10_000;
                let brightened = darkened + ((255 - darkened) * brighten / 10_000);
                rgba[base + channel] = brightened.min(255) as u8;
            }
        }
    }
}

fn luminance(pixel: &[u8]) -> u16 {
    (pixel[0] as u16 * 54 + pixel[1] as u16 * 183 + pixel[2] as u16 * 19) / 256
}
