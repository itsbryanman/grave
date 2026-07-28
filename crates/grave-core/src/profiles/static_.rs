use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

use super::{clamp_u8, RotContext};

const STATIC_CHARS: [char; 9] = ['▚', '▞', '░', '▒', '#', '$', '%', '&', '@'];
const BRIGHTNESS_WAVE: [u16; 40] = [
    256, 265, 273, 279, 281, 279, 273, 265, 256, 247, 239, 231, 225, 221, 218, 221, 225, 231, 239,
    247, 256, 265, 273, 279, 281, 279, 273, 265, 256, 247, 239, 231, 225, 221, 218, 221, 225, 231,
    239, 247,
];

pub(super) fn rot_image(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) {
    if width == 0 || height == 0 || rgba.len() < 4 {
        return;
    }

    let pixel_count = (width as usize).saturating_mul(height as usize);
    let salt_count = ((pixel_count as u64 * context.q as u64 * 15) / 1_000_000) as usize;
    for _ in 0..salt_count {
        let pixel = (rng.next_u32() as usize) % pixel_count;
        let value = if rng.next_u32() & 1 == 0 { 0 } else { 255 };
        let base = pixel * 4;
        rgba[base] = value;
        rgba[base + 1] = value;
        rgba[base + 2] = value;
    }

    let block_size = 16usize;
    let blocks_w = (width as usize).div_ceil(block_size);
    let blocks_h = (height as usize).div_ceil(block_size);
    let block_count = (context.q / 700) as usize;
    let block_reference = rgba.to_vec();
    for _ in 0..block_count {
        let block_x = (rng.next_u32() as usize) % blocks_w;
        let block_y = (rng.next_u32() as usize) % blocks_h;
        let x0 = block_x * block_size;
        let y0 = block_y * block_size;
        let x1 = (x0 + block_size).min(width as usize);
        let y1 = (y0 + block_size).min(height as usize);
        match rng.next_u32() % 3 {
            0 => shift_red_channel(rgba, &block_reference, width as usize, x0, x1, y0, y1),
            1 => fill_block_gray(rgba, width as usize, x0, x1, y0, y1, rng),
            _ => copy_other_block(
                rgba,
                &block_reference,
                width as usize,
                height as usize,
                x0,
                x1,
                y0,
                y1,
                block_size,
                rng,
            ),
        }
    }

    let tear_count = (context.q / 2_000) as usize;
    for _ in 0..tear_count {
        let band_y = (rng.next_u32() as usize) % height as usize;
        let band_height = 3 + (rng.next_u32() as usize % 8);
        let magnitude = 4 + (rng.next_u32() as usize % 27);
        let shift = if rng.next_u32() & 1 == 0 {
            magnitude as isize
        } else {
            -(magnitude as isize)
        };
        shift_band_rows(
            rgba,
            width as usize,
            height as usize,
            band_y,
            band_height,
            shift,
        );
    }

    if context.q >= 7_000 {
        let phase = (rng.next_u32() as usize) % BRIGHTNESS_WAVE.len();
        for row in 0..height as usize {
            let multiplier = BRIGHTNESS_WAVE[(row + phase) % BRIGHTNESS_WAVE.len()] as i32;
            for col in 0..width as usize {
                let base = (row * width as usize + col) * 4;
                rgba[base] = clamp_u8((rgba[base] as i32 * multiplier) / 256);
                rgba[base + 1] = clamp_u8((rgba[base + 1] as i32 * multiplier) / 256);
                rgba[base + 2] = clamp_u8((rgba[base + 2] as i32 * multiplier) / 256);
            }
        }
    }
}

pub(super) fn rot_text(text: &str, context: &RotContext, rng: &mut ChaCha8Rng) -> String {
    let ends_with_newline = text.ends_with('\n');
    let lines: Vec<String> = text.split('\n').map(|line| line.to_string()).collect();
    let duplicate_budget = ((context.q / 2_500) as usize).min(lines.len().max(1));
    let mut duplicate_counts = vec![0usize; lines.len()];

    for _ in 0..duplicate_budget {
        if lines.is_empty() {
            break;
        }
        let index = (rng.next_u32() as usize) % lines.len();
        duplicate_counts[index] = duplicate_counts[index].saturating_add(1);
    }

    let substitution_threshold = context.q / 5;
    let truncation_threshold = if context.q >= 8_000 {
        (context.q - 8_000) / 2
    } else {
        0
    };

    let mut rendered = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        rendered.push(substitute_chars(line, substitution_threshold, rng));
        for _ in 0..duplicate_counts[index] {
            let mut echoed = substitute_chars(line, 3_000, rng);
            if truncation_threshold > 0
                && !echoed.trim().is_empty()
                && rng.next_u32() % 10_000 < truncation_threshold
            {
                truncate_for_carrier_loss(&mut echoed);
            }
            rendered.push(echoed);
        }
    }

    for line in &mut rendered {
        if truncation_threshold > 0
            && !line.trim().is_empty()
            && rng.next_u32() % 10_000 < truncation_threshold
        {
            truncate_for_carrier_loss(line);
        }
    }

    let mut joined = rendered.join("\n");
    if ends_with_newline {
        joined.push('\n');
    }
    joined
}

fn substitute_chars(line: &str, threshold: u32, rng: &mut ChaCha8Rng) -> String {
    let mut output = String::with_capacity(line.len());
    for ch in line.chars() {
        if ch.is_whitespace() || threshold == 0 {
            output.push(ch);
        } else if rng.next_u32() % 10_000 < threshold {
            output.push(STATIC_CHARS[(rng.next_u32() as usize) % STATIC_CHARS.len()]);
        } else {
            output.push(ch);
        }
    }
    output
}

fn truncate_for_carrier_loss(line: &mut String) {
    let keep = line.chars().count().max(4) / 2;
    let mut truncated = line.chars().take(keep).collect::<String>();
    while truncated.ends_with(char::is_whitespace) {
        truncated.pop();
    }
    if !truncated.is_empty() {
        truncated.push(' ');
    }
    truncated.push_str("___ CARRIER LOST");
    *line = truncated;
}

fn shift_red_channel(
    rgba: &mut [u8],
    reference: &[u8],
    width: usize,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
) {
    let span = x1.saturating_sub(x0);
    if span == 0 {
        return;
    }
    let shift = 6.min(span.saturating_sub(1));
    for y in y0..y1 {
        for x in x0..x1 {
            let src_x = x0 + ((x - x0 + shift) % span);
            let dest = (y * width + x) * 4;
            let src = (y * width + src_x) * 4;
            rgba[dest] = reference[src];
        }
    }
}

fn fill_block_gray(
    rgba: &mut [u8],
    width: usize,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
    rng: &mut ChaCha8Rng,
) {
    let gray = 72 + (rng.next_u32() % 96) as u8;
    for y in y0..y1 {
        for x in x0..x1 {
            let base = (y * width + x) * 4;
            rgba[base] = gray;
            rgba[base + 1] = gray;
            rgba[base + 2] = gray;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn copy_other_block(
    rgba: &mut [u8],
    reference: &[u8],
    width: usize,
    height: usize,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
    block_size: usize,
    rng: &mut ChaCha8Rng,
) {
    let blocks_w = width.div_ceil(block_size);
    let blocks_h = height.div_ceil(block_size);
    let src_block_x = (rng.next_u32() as usize) % blocks_w;
    let src_block_y = (rng.next_u32() as usize) % blocks_h;
    let src_x0 = src_block_x * block_size;
    let src_y0 = src_block_y * block_size;

    for y in y0..y1 {
        for x in x0..x1 {
            let source_x = src_x0 + (x - x0);
            let source_y = src_y0 + (y - y0);
            if source_x >= width || source_y >= height {
                continue;
            }
            let dest = (y * width + x) * 4;
            let src = (source_y * width + source_x) * 4;
            rgba[dest..dest + 3].copy_from_slice(&reference[src..src + 3]);
        }
    }
}

fn shift_band_rows(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    band_y: usize,
    band_height: usize,
    shift: isize,
) {
    if width == 0 {
        return;
    }

    let row_bytes = width * 4;
    for row in band_y..(band_y + band_height).min(height) {
        let mut source = vec![0u8; row_bytes];
        let start = row * row_bytes;
        source.copy_from_slice(&rgba[start..start + row_bytes]);
        for x in 0..width {
            let src_x = ((x as isize - shift).rem_euclid(width as isize)) as usize;
            let dest = start + x * 4;
            let src = src_x * 4;
            rgba[dest..dest + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
}
