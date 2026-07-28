use std::collections::VecDeque;

use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

use super::{clamp_u8, has_dead_neighbor, RotContext};

pub(super) fn rot_image(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) {
    if width == 0 || height == 0 {
        return;
    }

    let block_size = 32usize;
    let blocks_w = ((width as usize) + block_size - 1) / block_size;
    let blocks_h = ((height as usize) + block_size - 1) / block_size;
    let total_blocks = blocks_w.saturating_mul(blocks_h);
    if total_blocks == 0 {
        return;
    }

    let mut dead = vec![false; total_blocks];
    let target_dead = ((total_blocks as u64 * context.q as u64 * 45) / 1_000_000) as usize;
    for _ in 0..target_dead {
        if let Some(index) = choose_clustered_block(&dead, blocks_w, blocks_h, rng) {
            dead[index] = true;
        }
    }

    if context.q >= 6_000 {
        let extra_rows = 1 + ((context.q - 6_000) / 1_500) as usize;
        for _ in 0..extra_rows.min(blocks_h) {
            let row = (rng.next_u32() as usize) % blocks_h;
            for x in 0..blocks_w {
                dead[row * blocks_w + x] = true;
            }
        }
    }

    paint_dead_blocks(
        rgba,
        width as usize,
        height as usize,
        block_size,
        blocks_w,
        &dead,
    );

    if context.q >= 9_000 {
        let clusters = dead_clusters(&dead, blocks_w, blocks_h);
        for cluster in clusters.iter().take(2) {
            let anchor = cluster_anchor(cluster, blocks_w);
            let offset = (anchor.1 * blocks_w + anchor.0) as u32 * 0x1000;
            let label = format!("! REGION UNRECOVERABLE (0X{offset:08X})");
            let px = anchor.0 * block_size + 2;
            let py = anchor.1 * block_size + 2;
            draw_label(rgba, width as usize, height as usize, px, py, &label);
        }
    }
}

pub(super) fn rot_text(text: &str, context: &RotContext, rng: &mut ChaCha8Rng) -> String {
    let paragraphs: Vec<&str> = if text.is_empty() {
        vec![""]
    } else {
        text.split("\n\n").collect()
    };
    let mut paragraph_lost = vec![false; paragraphs.len()];

    if context.q >= 9_000 && !paragraphs.is_empty() {
        let target =
            (((paragraphs.len() as u64) * (context.q - 9_000) as u64) / 2_000).max(1) as usize;
        choose_clustered_1d(
            paragraphs.len(),
            target.min(paragraphs.len()),
            rng,
            &mut paragraph_lost,
        );
    }

    let mut rendered = Vec::with_capacity(paragraphs.len());
    let mut lost_sections = 0usize;
    for (index, paragraph) in paragraphs.iter().enumerate() {
        if paragraph_lost[index] {
            lost_sections += 1;
            rendered.push("[SECTION UNRECOVERABLE]".to_string());
            continue;
        }

        let sentences = split_sentences(paragraph);
        if sentences.is_empty() {
            rendered.push((*paragraph).to_string());
            continue;
        }

        let mut dead_sentences = vec![false; sentences.len()];
        let target = ((sentences.len() as u64 * context.q as u64 * 35) / 1_000_000) as usize;
        choose_clustered_1d(
            sentences.len(),
            target.min(sentences.len()),
            rng,
            &mut dead_sentences,
        );

        let mut rebuilt = String::new();
        for (sentence_index, sentence) in sentences.iter().enumerate() {
            if dead_sentences[sentence_index] {
                rebuilt.push_str(&format!("[LOST: {} bytes]", sentence.as_bytes().len()));
            } else {
                rebuilt.push_str(sentence);
            }
        }
        rendered.push(rebuilt);
    }

    let mut body = rendered.join("\n\n");
    if lost_sections > 0 {
        body.push_str(&format!(
            "\n\n{} of {} sections could not be read.",
            lost_sections,
            paragraphs.len()
        ));
    }
    body
}

fn choose_clustered_block(
    dead: &[bool],
    blocks_w: usize,
    blocks_h: usize,
    rng: &mut ChaCha8Rng,
) -> Option<usize> {
    let mut total_weight = 0u32;
    for index in 0..dead.len() {
        if dead[index] {
            continue;
        }
        let x = index % blocks_w;
        let y = index / blocks_w;
        total_weight += if has_dead_neighbor(dead, x, y, blocks_w, blocks_h) {
            4
        } else {
            1
        };
    }

    if total_weight == 0 {
        return None;
    }

    let mut pick = rng.next_u32() % total_weight;
    for index in 0..dead.len() {
        if dead[index] {
            continue;
        }
        let x = index % blocks_w;
        let y = index / blocks_w;
        let weight = if has_dead_neighbor(dead, x, y, blocks_w, blocks_h) {
            4
        } else {
            1
        };
        if pick < weight {
            return Some(index);
        }
        pick -= weight;
    }
    None
}

fn paint_dead_blocks(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    block_size: usize,
    blocks_w: usize,
    dead: &[bool],
) {
    for (index, is_dead) in dead.iter().enumerate() {
        if !is_dead {
            continue;
        }
        let block_x = index % blocks_w;
        let block_y = index / blocks_w;
        let x0 = block_x * block_size;
        let y0 = block_y * block_size;
        let x1 = (x0 + block_size).min(width);
        let y1 = (y0 + block_size).min(height);

        for y in y0..y1 {
            for x in x0..x1 {
                let border = x == x0 || y == y0 || x + 1 == x1 || y + 1 == y1;
                let value = if border { 0x2b } else { 0x3c };
                let base = (y * width + x) * 4;
                rgba[base] = value;
                rgba[base + 1] = value;
                rgba[base + 2] = value;
            }
        }
    }
}

fn dead_clusters(dead: &[bool], blocks_w: usize, blocks_h: usize) -> Vec<Vec<usize>> {
    let mut seen = vec![false; dead.len()];
    let mut clusters = Vec::new();

    for index in 0..dead.len() {
        if !dead[index] || seen[index] {
            continue;
        }
        let mut cluster = Vec::new();
        let mut queue = VecDeque::from([index]);
        seen[index] = true;

        while let Some(current) = queue.pop_front() {
            cluster.push(current);
            let x = current % blocks_w;
            let y = current / blocks_w;
            for (nx, ny) in neighbors_4(x, y, blocks_w, blocks_h) {
                let next = ny * blocks_w + nx;
                if dead[next] && !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }

        clusters.push(cluster);
    }

    clusters.sort_by(|left, right| right.len().cmp(&left.len()));
    clusters
}

fn neighbors_4(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors.into_iter()
}

fn cluster_anchor(cluster: &[usize], blocks_w: usize) -> (usize, usize) {
    let mut best = (usize::MAX, usize::MAX);
    for index in cluster {
        let x = index % blocks_w;
        let y = index / blocks_w;
        if (y, x) < (best.1, best.0) {
            best = (x, y);
        }
    }
    best
}

fn choose_clustered_1d(count: usize, target: usize, rng: &mut ChaCha8Rng, chosen: &mut [bool]) {
    for _ in 0..target {
        let mut total_weight = 0u32;
        for index in 0..count {
            if chosen[index] {
                continue;
            }
            total_weight += if adjacent_dead(chosen, index) { 4 } else { 1 };
        }
        if total_weight == 0 {
            break;
        }

        let mut pick = rng.next_u32() % total_weight;
        for index in 0..count {
            if chosen[index] {
                continue;
            }
            let weight = if adjacent_dead(chosen, index) { 4 } else { 1 };
            if pick < weight {
                chosen[index] = true;
                break;
            }
            pick -= weight;
        }
    }
}

fn adjacent_dead(chosen: &[bool], index: usize) -> bool {
    (index > 0 && chosen[index - 1]) || (index + 1 < chosen.len() && chosen[index + 1])
}

fn split_sentences(paragraph: &str) -> Vec<String> {
    if paragraph.is_empty() {
        return vec![String::new()];
    }

    let mut parts = Vec::new();
    let mut start = 0usize;
    for (index, ch) in paragraph.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let end = index + ch.len_utf8();
            parts.push(paragraph[start..end].to_string());
            start = end;
        }
    }
    if start < paragraph.len() {
        parts.push(paragraph[start..].to_string());
    }
    if parts.is_empty() {
        parts.push(paragraph.to_string());
    }
    parts
}

fn draw_label(rgba: &mut [u8], width: usize, height: usize, x: usize, y: usize, label: &str) {
    let label_width = label.chars().count() * 6 + 4;
    let label_height = 11usize;
    for py in y..(y + label_height).min(height) {
        for px in x..(x + label_width).min(width) {
            let base = (py * width + px) * 4;
            rgba[base] = clamp_u8((rgba[base] as i32 * 40) / 100);
            rgba[base + 1] = clamp_u8((rgba[base + 1] as i32 * 40) / 100);
            rgba[base + 2] = clamp_u8((rgba[base + 2] as i32 * 40) / 100);
        }
    }

    let mut cursor_x = x + 2;
    for ch in label.chars() {
        draw_glyph(rgba, width, height, cursor_x, y + 2, ch);
        cursor_x += 6;
    }
}

fn draw_glyph(rgba: &mut [u8], width: usize, height: usize, x: usize, y: usize, ch: char) {
    let glyph = glyph_rows(ch);
    for (row, mask) in glyph.iter().enumerate() {
        for col in 0..5usize {
            if (mask >> (4 - col)) & 1 == 0 {
                continue;
            }
            let px = x + col;
            let py = y + row;
            if px >= width || py >= height {
                continue;
            }
            let base = (py * width + px) * 4;
            rgba[base] = 235;
            rgba[base + 1] = 235;
            rgba[base + 2] = 235;
        }
    }
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0F, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0F],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'G' => [0x0F, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x0A, 0x0A, 0x04],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        ' ' => [0x00; 7],
        _ => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x00, 0x08],
    }
}
