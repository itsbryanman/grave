use rand_chacha::ChaCha8Rng;
use rand_core::RngCore;

use super::RotContext;

const BLOCK_SIZE: usize = 8;
const SPREAD_THRESHOLD: u32 = 1_800;
const YOUNG_COLOR: [u8; 3] = [0x2a, 0x3d, 0x24];
const MATURE_COLOR: [u8; 3] = [0x14, 0x1f, 0x10];
const OLD_COLOR: [u8; 3] = [0x0a, 0x0d, 0x08];

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

                for (nx, ny) in neighbors_8(x, y, cells_w, cells_h) {
                    let next = ny * cells_w + nx;
                    if colonized_at[next].is_some() || pending_marks[next] {
                        continue;
                    }
                    if rng.next_u32() % 10_000 < SPREAD_THRESHOLD {
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
    let mut neighbors = Vec::with_capacity(8);
    let y_start = y.saturating_sub(1);
    let y_end = (y + 1).min(height.saturating_sub(1));
    let x_start = x.saturating_sub(1);
    let x_end = (x + 1).min(width.saturating_sub(1));
    for ny in y_start..=y_end {
        for nx in x_start..=x_end {
            if nx == x && ny == y {
                continue;
            }
            neighbors.push((nx, ny));
        }
    }
    neighbors.into_iter()
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
        let edge = has_empty_neighbor(colony_days, x, y, cells_w, cells_h);
        return Some(CellTone::for_maturity(maturity, edge, false));
    }

    let mut strongest_neighbor: Option<usize> = None;
    for (nx, ny) in neighbors_8(x, y, cells_w, cells_h) {
        let Some(start_day) = colony_days[ny * cells_w + nx] else {
            continue;
        };
        let maturity = days.saturating_sub(start_day as usize).max(1);
        strongest_neighbor = Some(strongest_neighbor.map_or(maturity, |best| best.max(maturity)));
    }

    strongest_neighbor.map(|maturity| CellTone::for_maturity(maturity, false, true))
}

fn has_empty_neighbor(
    colony_days: &[Option<u16>],
    x: usize,
    y: usize,
    cells_w: usize,
    cells_h: usize,
) -> bool {
    neighbors_8(x, y, cells_w, cells_h).any(|(nx, ny)| colony_days[ny * cells_w + nx].is_none())
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

    for py in y0..y1 {
        for px in x0..x1 {
            let base = (py * width + px) * 4;
            let noise = noise_at(noise_seed, px, py);
            if tone.fuzzy_edge && tone.edge && noise.is_multiple_of(5) {
                continue;
            }

            blend_pixel(&mut rgba[base..base + 4], tone.color, tone.opacity);

            if tone.speckle && noise.is_multiple_of(7) {
                rgba[base] = rgba[base].saturating_sub(6);
                rgba[base + 1] = rgba[base + 1].saturating_add(4);
                rgba[base + 2] = rgba[base + 2].saturating_sub(4);
            }

            if tone.spore_dots && noise.is_multiple_of(97) {
                let spore = 232 + (noise % 24) as u8;
                rgba[base] = spore;
                rgba[base + 1] = spore;
                rgba[base + 2] = spore;
            }
        }
    }
}

fn blend_pixel(pixel: &mut [u8], target: [u8; 3], opacity: u16) {
    for channel in 0..3 {
        let source = pixel[channel] as u16;
        let mixed = (source * (256 - opacity) + target[channel] as u16 * opacity + 128) / 256;
        pixel[channel] = mixed as u8;
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
    edge: bool,
    fuzzy_edge: bool,
    speckle: bool,
    spore_dots: bool,
}

impl CellTone {
    fn for_maturity(maturity: usize, edge: bool, feathered: bool) -> Self {
        let (color, opacity, fuzzy_edge, speckle, spore_dots) = if maturity <= 5 {
            (YOUNG_COLOR, 64u16, false, true, false)
        } else if maturity <= 20 {
            (MATURE_COLOR, 154u16, true, false, false)
        } else {
            (OLD_COLOR, 214u16, false, false, true)
        };

        Self {
            color,
            opacity: if feathered { opacity / 2 } else { opacity },
            edge,
            fuzzy_edge,
            speckle,
            spore_dots,
        }
    }
}
