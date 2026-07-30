use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

use super::RotContext;

const BLOCK_SIZE: usize = 8;
const SPREAD_THRESHOLD: i32 = 900;
const SPREAD_FIELD_STEP: usize = 5;
const SPREAD_BIAS_RANGE: i32 = 1_200;
const ACTIVE_SPREAD_BASE_DAYS: usize = 8;
const EDGE_FADE_DISTANCE: i16 = 5;
const HALO_FADE_DISTANCE: i16 = 7;
const YOUNG_COLOR: [u8; 3] = [0x76, 0x8f, 0x5b];
const MATURE_COLOR: [u8; 3] = [0x4a, 0x62, 0x3c];
const OLD_COLOR: [u8; 3] = [0x23, 0x33, 0x1d];

const NORTH: u8 = 1 << 0;
const NORTHEAST: u8 = 1 << 1;
const EAST: u8 = 1 << 2;
const SOUTHEAST: u8 = 1 << 3;
const SOUTH: u8 = 1 << 4;
const SOUTHWEST: u8 = 1 << 5;
const WEST: u8 = 1 << 6;
const NORTHWEST: u8 = 1 << 7;

const NEIGHBOR_STEPS: [(isize, isize, u8); 8] = [
    (0, -1, NORTH),
    (1, -1, NORTHEAST),
    (1, 0, EAST),
    (1, 1, SOUTHEAST),
    (0, 1, SOUTH),
    (-1, 1, SOUTHWEST),
    (-1, 0, WEST),
    (-1, -1, NORTHWEST),
];

pub(super) fn rot_image(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) {
    if width == 0 || height == 0 || context.age_days == 0 {
        return;
    }

    let cells_w = (width as usize).div_ceil(BLOCK_SIZE);
    let cells_h = (height as usize).div_ceil(BLOCK_SIZE);
    let colony_days = simulate_colonies(cells_w, cells_h, context, rng);
    if colony_days.iter().all(Option::is_none) {
        return;
    }

    let mut noise_seeds = vec![0u32; colony_days.len()];
    for (index, colonized_at) in colony_days.iter().enumerate() {
        if colonized_at.is_some() {
            noise_seeds[index] = rng.next_u32();
        }
    }

    for y in 0..cells_h {
        for x in 0..cells_w {
            let index = y * cells_w + x;
            let Some(tone) = colony_tone(&colony_days, x, y, cells_w, cells_h, context.age_days)
            else {
                continue;
            };
            paint_cell(
                rgba,
                width as usize,
                height as usize,
                x,
                y,
                &tone,
                noise_seeds[index],
            );
        }
    }
}

pub(super) fn rot_text(text: &str, context: &RotContext, rng: &mut ChaCha8Rng) -> String {
    let simulated_days = simulated_days(context.age_days);
    let replacements_per_day = (context.q / 400) as usize;
    if simulated_days == 0 || replacements_per_day == 0 {
        return text.to_string();
    }

    let (tokens, word_indices) = tokenize_with_whitespace(text);
    if word_indices.is_empty() {
        return text.to_string();
    }

    let mut word_states = vec![0u8; word_indices.len()];
    for _ in 0..simulated_days {
        for _ in 0..replacements_per_day {
            let word = (rng.next_u32() as usize) % word_indices.len();
            word_states[word] = word_states[word].saturating_add(1).min(3);
        }
    }

    let mut rendered = String::with_capacity(text.len() + word_indices.len() * 2);
    let mut word_cursor = 0usize;
    for token in tokens {
        match token {
            Token::Whitespace(span) => rendered.push_str(&span),
            Token::Word(word) => {
                rendered.push_str(&render_word_stage(&word, word_states[word_cursor]));
                word_cursor += 1;
            }
        }
    }
    rendered
}

fn simulate_colonies(
    cells_w: usize,
    cells_h: usize,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) -> Vec<Option<u16>> {
    let total_cells = cells_w.saturating_mul(cells_h);
    let mut colonized_at = vec![None; total_cells];
    if total_cells == 0 {
        return colonized_at;
    }

    let growth_field = GrowthField::new(cells_w, cells_h, rng);
    let seed_count = (3 + context.q / 1_500) as usize;
    for _ in 0..seed_count.min(total_cells) {
        if let Some(index) = choose_unoccupied_cell(&colonized_at, rng) {
            colonized_at[index] = Some(0);
        }
    }

    let days = simulated_days(context.age_days);
    let mut pending = Vec::new();
    let mut pending_marks = vec![false; total_cells];
    for day in 1..=days {
        pending.clear();
        pending_marks.fill(false);
        for y in 0..cells_h {
            for x in 0..cells_w {
                let index = y * cells_w + x;
                let Some(start_day) = colonized_at[index] else {
                    continue;
                };
                if start_day as usize >= day {
                    continue;
                }
                if day.saturating_sub(start_day as usize)
                    > active_spread_window(&growth_field, x, y)
                {
                    continue;
                }

                for (nx, ny) in neighbors_8(x, y, cells_w, cells_h) {
                    let next = ny * cells_w + nx;
                    if colonized_at[next].is_some() || pending_marks[next] {
                        continue;
                    }
                    if rng.next_u32() % 10_000 < spread_threshold_for_cell(&growth_field, nx, ny) {
                        pending.push(next);
                        pending_marks[next] = true;
                    }
                }
            }
        }

        for index in &pending {
            colonized_at[*index] = Some(day as u16);
        }
    }

    colonized_at
}

fn choose_unoccupied_cell(colonized_at: &[Option<u16>], rng: &mut ChaCha8Rng) -> Option<usize> {
    if colonized_at.iter().all(Option::is_some) {
        return None;
    }

    let mut index = (rng.next_u32() as usize) % colonized_at.len();
    for _ in 0..colonized_at.len() {
        if colonized_at[index].is_none() {
            return Some(index);
        }
        index = (index + 1) % colonized_at.len();
    }
    None
}

fn neighbors_8(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> impl Iterator<Item = (usize, usize)> {
    NEIGHBOR_STEPS.into_iter().filter_map(move |(dx, dy, _)| {
        let nx = x.checked_add_signed(dx)?;
        let ny = y.checked_add_signed(dy)?;
        (nx < width && ny < height).then_some((nx, ny))
    })
}

fn simulated_days(age_days: u64) -> usize {
    age_days.min(365) as usize
}

fn colony_tone(
    colony_days: &[Option<u16>],
    x: usize,
    y: usize,
    cells_w: usize,
    cells_h: usize,
    age_days: u64,
) -> Option<CellTone> {
    let index = y * cells_w + x;
    let days = simulated_days(age_days);
    if let Some(start_day) = colony_days[index] {
        let maturity = days.saturating_sub(start_day as usize).max(1);
        let exposed_edges = empty_neighbor_mask(colony_days, x, y, cells_w, cells_h);
        return Some(CellTone::for_maturity(maturity, exposed_edges));
    }

    let mut strongest_neighbor: Option<usize> = None;
    let mut contact_edges = 0u8;
    for (dx, dy, mask) in NEIGHBOR_STEPS {
        let Some(nx) = x.checked_add_signed(dx) else {
            continue;
        };
        let Some(ny) = y.checked_add_signed(dy) else {
            continue;
        };
        if nx >= cells_w || ny >= cells_h {
            continue;
        }
        let Some(start_day) = colony_days[ny * cells_w + nx] else {
            continue;
        };
        let maturity = days.saturating_sub(start_day as usize).max(1);
        strongest_neighbor = Some(strongest_neighbor.map_or(maturity, |best| best.max(maturity)));
        contact_edges |= mask;
    }

    strongest_neighbor.map(|maturity| CellTone::for_halo(maturity, contact_edges))
}

fn empty_neighbor_mask(
    colony_days: &[Option<u16>],
    x: usize,
    y: usize,
    cells_w: usize,
    cells_h: usize,
) -> u8 {
    let mut mask = 0u8;
    for (dx, dy, direction) in NEIGHBOR_STEPS {
        let Some(nx) = x.checked_add_signed(dx) else {
            continue;
        };
        let Some(ny) = y.checked_add_signed(dy) else {
            continue;
        };
        if nx >= cells_w || ny >= cells_h {
            continue;
        }
        if colony_days[ny * cells_w + nx].is_none() {
            mask |= direction;
        }
    }
    mask
}

fn paint_cell(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    cell_x: usize,
    cell_y: usize,
    tone: &CellTone,
    noise_seed: u32,
) {
    let x0 = cell_x * BLOCK_SIZE;
    let y0 = cell_y * BLOCK_SIZE;
    let x1 = (x0 + BLOCK_SIZE).min(width);
    let y1 = (y0 + BLOCK_SIZE).min(height);
    let cell_width = x1.saturating_sub(x0);
    let cell_height = y1.saturating_sub(y0);

    for py in y0..y1 {
        for px in x0..x1 {
            let base = (py * width + px) * 4;
            let noise = noise_at(noise_seed, px, py);
            let edge_opacity = if tone.halo {
                halo_coverage(
                    tone.edge_mask,
                    px - x0,
                    py - y0,
                    cell_width,
                    cell_height,
                    noise_seed,
                )
            } else {
                colony_coverage(
                    tone.edge_mask,
                    px - x0,
                    py - y0,
                    cell_width,
                    cell_height,
                    noise_seed,
                )
            };
            let opacity = scale_opacity(tone.opacity, edge_opacity);
            if opacity == 0 {
                continue;
            }

            blend_pixel(&mut rgba[base..base + 4], tone.color, opacity);

            if !tone.halo && tone.speckle_chance > 0 && noise % 10_000 < tone.speckle_chance {
                rgba[base] = rgba[base].saturating_sub(5);
                rgba[base + 1] = rgba[base + 1].saturating_add(7);
                rgba[base + 2] = rgba[base + 2].saturating_sub(3);
            }

            if !tone.halo && tone.spore_chance > 0 && noise % 10_000 < tone.spore_chance {
                let spore = 168 + (noise % 48) as u8;
                rgba[base] = spore.saturating_sub(18);
                rgba[base + 1] = spore;
                rgba[base + 2] = spore.saturating_sub(34);
            }
        }
    }
}

fn blend_pixel(pixel: &mut [u8], target: [u8; 3], opacity: u16) {
    let luminance = pixel_luminance(pixel);
    let desaturate_amount = 96 + opacity / 2;
    let shadow_luminance = scale_channel(luminance, 216u16.saturating_sub(opacity / 2));
    let tint_amount = 120 + opacity / 3;

    for channel in 0..3 {
        let muted = mix_channel(pixel[channel], luminance, desaturate_amount);
        let tinted = mix_channel(shadow_luminance, target[channel], tint_amount);
        pixel[channel] = mix_channel(muted, tinted, opacity);
    }
}

fn noise_at(seed: u32, x: usize, y: usize) -> u32 {
    seed.wrapping_add((x as u32).wrapping_mul(0x045D_9F3B)) ^ (y as u32).wrapping_mul(0x27D4_EB2D)
}

fn render_word_stage(word: &str, stage: u8) -> String {
    match stage {
        0 => word.to_string(),
        1 => format!("▒{word}▒"),
        2 => word
            .chars()
            .enumerate()
            .map(|(index, ch)| if index % 2 == 1 { '▓' } else { ch })
            .collect(),
        _ => "▓".repeat(word.chars().count().max(1)),
    }
}

fn tokenize_with_whitespace(text: &str) -> (Vec<Token>, Vec<usize>) {
    let mut tokens = Vec::new();
    let mut word_indices = Vec::new();
    let mut current = String::new();
    let mut current_is_whitespace = None;

    for ch in text.chars() {
        let is_whitespace = ch.is_whitespace();
        match current_is_whitespace {
            Some(value) if value == is_whitespace => current.push(ch),
            Some(value) => {
                push_token(
                    &mut tokens,
                    &mut word_indices,
                    std::mem::take(&mut current),
                    value,
                );
                current.push(ch);
                current_is_whitespace = Some(is_whitespace);
            }
            None => {
                current.push(ch);
                current_is_whitespace = Some(is_whitespace);
            }
        }
    }

    if let Some(value) = current_is_whitespace {
        push_token(&mut tokens, &mut word_indices, current, value);
    }

    (tokens, word_indices)
}

fn push_token(
    tokens: &mut Vec<Token>,
    word_indices: &mut Vec<usize>,
    value: String,
    is_whitespace: bool,
) {
    if is_whitespace {
        tokens.push(Token::Whitespace(value));
    } else {
        word_indices.push(tokens.len());
        tokens.push(Token::Word(value));
    }
}

#[derive(Clone, Debug)]
enum Token {
    Word(String),
    Whitespace(String),
}

#[derive(Clone, Copy, Debug)]
struct CellTone {
    color: [u8; 3],
    opacity: u16,
    edge_mask: u8,
    halo: bool,
    speckle_chance: u32,
    spore_chance: u32,
}

impl CellTone {
    fn for_maturity(maturity: usize, edge_mask: u8) -> Self {
        let (color, opacity, late_stage) = tone_curve(maturity);
        let speckle_chance = 240 + u32::from(256u16.saturating_sub(late_stage)) * 3;
        let spore_chance = u32::from(late_stage) * 2;

        Self {
            color,
            opacity,
            edge_mask,
            halo: false,
            speckle_chance,
            spore_chance,
        }
    }

    fn for_halo(maturity: usize, edge_mask: u8) -> Self {
        let (color, opacity, _) = tone_curve(maturity);

        Self {
            color,
            opacity: opacity / 3,
            edge_mask,
            halo: true,
            speckle_chance: 0,
            spore_chance: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct GrowthField {
    values: Vec<u16>,
    width: usize,
    height: usize,
}

impl GrowthField {
    fn new(cells_w: usize, cells_h: usize, rng: &mut ChaCha8Rng) -> Self {
        let width = cells_w.div_ceil(SPREAD_FIELD_STEP) + 1;
        let height = cells_h.div_ceil(SPREAD_FIELD_STEP) + 1;
        let mut values = Vec::with_capacity(width.saturating_mul(height));
        for _ in 0..width.saturating_mul(height) {
            values.push((rng.next_u32() % 1_025) as u16);
        }

        Self {
            values,
            width,
            height,
        }
    }

    fn sample(&self, x: usize, y: usize) -> u16 {
        if self.values.is_empty() {
            return 512;
        }

        let sx = (x / SPREAD_FIELD_STEP).min(self.width.saturating_sub(1));
        let sy = (y / SPREAD_FIELD_STEP).min(self.height.saturating_sub(1));
        let tx = x % SPREAD_FIELD_STEP;
        let ty = y % SPREAD_FIELD_STEP;
        let sx1 = (sx + 1).min(self.width.saturating_sub(1));
        let sy1 = (sy + 1).min(self.height.saturating_sub(1));

        let top = lerp_u16(
            self.values[sy * self.width + sx],
            self.values[sy * self.width + sx1],
            tx,
            SPREAD_FIELD_STEP,
        );
        let bottom = lerp_u16(
            self.values[sy1 * self.width + sx],
            self.values[sy1 * self.width + sx1],
            tx,
            SPREAD_FIELD_STEP,
        );
        lerp_u16(top, bottom, ty, SPREAD_FIELD_STEP)
    }
}

fn spread_threshold_for_cell(field: &GrowthField, x: usize, y: usize) -> u32 {
    let centered = i32::from(field.sample(x, y)) - 512;
    (SPREAD_THRESHOLD + (centered * SPREAD_BIAS_RANGE) / 512).clamp(120, 2_400) as u32
}

fn active_spread_window(field: &GrowthField, x: usize, y: usize) -> usize {
    ACTIVE_SPREAD_BASE_DAYS + usize::from(field.sample(x, y) / 64)
}

fn tone_curve(maturity: usize) -> ([u8; 3], u16, u16) {
    let young_to_mature = ramp_between(maturity, 1, 18);
    let mature_to_old = ramp_between(maturity, 18, 72);
    let color = mix_color(
        mix_color(YOUNG_COLOR, MATURE_COLOR, young_to_mature),
        OLD_COLOR,
        mature_to_old,
    );
    let opacity = if maturity <= 18 {
        mix_u16(76, 168, young_to_mature)
    } else {
        mix_u16(168, 228, mature_to_old)
    };

    (color, opacity, mature_to_old)
}

fn ramp_between(value: usize, start: usize, end: usize) -> u16 {
    if value <= start {
        return 0;
    }
    if value >= end {
        return 256;
    }

    (((value - start) * 256) / end.saturating_sub(start).max(1)) as u16
}

fn colony_coverage(
    mask: u8,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    noise_seed: u32,
) -> u16 {
    if mask == 0 {
        return 256;
    }

    let distance = distance_to_edge(mask, x, y, width, height)
        + edge_variation(noise_seed, x, y, width, height);
    smoothstep(progress(distance, EDGE_FADE_DISTANCE))
}

fn halo_coverage(
    mask: u8,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    noise_seed: u32,
) -> u16 {
    if mask == 0 {
        return 0;
    }

    let distance = distance_to_edge(mask, x, y, width, height)
        + edge_variation(noise_seed, x, y, width, height);
    256u16.saturating_sub(smoothstep(progress(distance, HALO_FADE_DISTANCE)))
}

fn distance_to_edge(mask: u8, x: usize, y: usize, width: usize, height: usize) -> i16 {
    let north = y as i16;
    let south = height.saturating_sub(y + 1) as i16;
    let west = x as i16;
    let east = width.saturating_sub(x + 1) as i16;
    let mut best = i16::MAX;

    if mask & NORTH != 0 {
        best = best.min(north);
    }
    if mask & NORTHEAST != 0 {
        best = best.min(north.min(east));
    }
    if mask & EAST != 0 {
        best = best.min(east);
    }
    if mask & SOUTHEAST != 0 {
        best = best.min(south.min(east));
    }
    if mask & SOUTH != 0 {
        best = best.min(south);
    }
    if mask & SOUTHWEST != 0 {
        best = best.min(south.min(west));
    }
    if mask & WEST != 0 {
        best = best.min(west);
    }
    if mask & NORTHWEST != 0 {
        best = best.min(north.min(west));
    }

    if best == i16::MAX {
        0
    } else {
        best
    }
}

fn edge_variation(noise_seed: u32, x: usize, y: usize, width: usize, height: usize) -> i16 {
    let width = width.saturating_sub(1).max(1);
    let height = height.saturating_sub(1).max(1);
    let top = lerp_i16(
        corner_offset(noise_seed, 0),
        corner_offset(noise_seed, 1),
        x,
        width,
    );
    let bottom = lerp_i16(
        corner_offset(noise_seed, 2),
        corner_offset(noise_seed, 3),
        x,
        width,
    );
    lerp_i16(top, bottom, y, height)
}

fn corner_offset(noise_seed: u32, corner: u32) -> i16 {
    let value = noise_seed.rotate_left(corner * 7 + 3) ^ corner.wrapping_mul(0x9E37_79B9);
    (value % 5) as i16 - 2
}

fn progress(distance: i16, radius: i16) -> u16 {
    if radius <= 0 || distance >= radius {
        return 256;
    }
    if distance <= 0 {
        return 0;
    }

    (distance as u16) * 256 / radius as u16
}

fn smoothstep(value: u16) -> u16 {
    let value = u64::from(value.min(256));
    ((3 * value * value * 256).saturating_sub(2 * value * value * value) / 65_536) as u16
}

fn scale_opacity(opacity: u16, coverage: u16) -> u16 {
    ((u32::from(opacity) * u32::from(coverage) + 128) / 256) as u16
}

fn pixel_luminance(pixel: &[u8]) -> u8 {
    ((u16::from(pixel[0]) * 54 + u16::from(pixel[1]) * 183 + u16::from(pixel[2]) * 19 + 128) / 256)
        as u8
}

fn scale_channel(channel: u8, factor: u16) -> u8 {
    ((u16::from(channel) * factor + 128) / 256) as u8
}

fn mix_channel(source: u8, target: u8, amount: u16) -> u8 {
    ((u16::from(source) * (256 - amount) + u16::from(target) * amount + 128) / 256) as u8
}

fn mix_color(source: [u8; 3], target: [u8; 3], amount: u16) -> [u8; 3] {
    [
        mix_channel(source[0], target[0], amount),
        mix_channel(source[1], target[1], amount),
        mix_channel(source[2], target[2], amount),
    ]
}

fn mix_u16(source: u16, target: u16, amount: u16) -> u16 {
    ((u32::from(source) * u32::from(256 - amount) + u32::from(target) * u32::from(amount) + 128)
        / 256) as u16
}

fn lerp_u16(source: u16, target: u16, amount: usize, scale: usize) -> u16 {
    let scale = scale.max(1) as u32;
    let amount = amount.min(scale as usize) as u32;
    ((u32::from(source) * (scale - amount) + u32::from(target) * amount + scale / 2) / scale) as u16
}

fn lerp_i16(source: i16, target: i16, amount: usize, scale: usize) -> i16 {
    let scale = scale.max(1) as i32;
    let amount = amount.min(scale as usize) as i32;
    ((i32::from(source) * (scale - amount) + i32::from(target) * amount + scale / 2) / scale) as i16
}
