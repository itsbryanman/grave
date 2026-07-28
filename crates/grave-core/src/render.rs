use std::fmt::Write as _;

use image::codecs::png::PngEncoder;
use image::{self, ColorType, ImageEncoder};

use crate::container::{decompress_payload, parse_grave};
use crate::profiles::{rot_image, rot_text, RotContext};
use crate::rng::render_rng;
use crate::{decay_snapshot, DecaySnapshot, GraveError, GraveHeader};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedText {
    pub body: String,
    pub is_hex_dump: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderedPayload {
    Image(RenderedImage),
    Text(RenderedText),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderResult {
    pub header: GraveHeader,
    pub disturbed: bool,
    pub snapshot: DecaySnapshot,
    pub payload: RenderedPayload,
}

pub fn render_grave(bytes: &[u8], now: u64) -> Result<RenderResult, GraveError> {
    let parsed = parse_grave(bytes)?;
    let mut snapshot = decay_snapshot(&parsed.header, now)?;
    if parsed.disturbed {
        snapshot.intensity = 1.0;
        snapshot.q = 10_000;
    }

    let payload = decompress_payload(parsed.payload, parsed.original_len)?;
    let context = RotContext {
        q: snapshot.q,
        age_days: snapshot.age_days,
        neglect_days: snapshot.neglect_days,
        open_count: parsed.header.open_count,
    };

    let rendered = match render_image_payload(&payload, &parsed.header, &context)? {
        Some(image) => RenderedPayload::Image(image),
        None => render_text_payload(&payload, &parsed.header, &context)?,
    };

    Ok(RenderResult {
        header: parsed.header,
        disturbed: parsed.disturbed,
        snapshot,
        payload: rendered,
    })
}

pub fn encode_png(image: &RenderedImage) -> Result<Vec<u8>, GraveError> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes).write_image(
        &image.rgba,
        image.width,
        image.height,
        ColorType::Rgba8,
    )?;
    Ok(bytes)
}

fn render_image_payload(
    payload: &[u8],
    header: &GraveHeader,
    context: &RotContext,
) -> Result<Option<RenderedImage>, GraveError> {
    if !header.mimetype.starts_with("image/") {
        return Ok(None);
    }

    let image = match image::load_from_memory(payload) {
        Ok(image) => image.to_rgba8(),
        Err(_) => return Ok(None),
    };
    let (width, height) = image.dimensions();
    let mut rgba = image.into_raw();
    let seed_values = [
        context.age_days,
        context.neglect_days,
        context.open_count as u64,
        context.q as u64,
    ];
    let mut rng = render_rng(
        &header.burial_id,
        0x10 | header.profile.as_byte(),
        &seed_values,
    );
    rot_image(header.profile, &mut rgba, width, height, context, &mut rng)?;

    Ok(Some(RenderedImage {
        rgba,
        width,
        height,
    }))
}

fn render_text_payload(
    payload: &[u8],
    header: &GraveHeader,
    context: &RotContext,
) -> Result<RenderedPayload, GraveError> {
    let (text, is_hex_dump) = if header.mimetype.starts_with("text/") {
        (String::from_utf8_lossy(payload).into_owned(), false)
    } else if let Ok(text) = std::str::from_utf8(payload) {
        (text.to_string(), false)
    } else {
        (hex_dump(payload), true)
    };

    let seed_values = [
        context.age_days,
        context.neglect_days,
        context.open_count as u64,
        context.q as u64,
        is_hex_dump as u64,
    ];
    let mut rng = render_rng(
        &header.burial_id,
        0x20 | header.profile.as_byte(),
        &seed_values,
    );
    let body = rot_text(header.profile, &text, context, &mut rng)?;
    Ok(RenderedPayload::Text(RenderedText { body, is_hex_dump }))
}

fn hex_dump(bytes: &[u8]) -> String {
    let mut output = String::new();
    for (line, chunk) in bytes.chunks(16).enumerate() {
        let offset = line * 16;
        let _ = write!(output, "{offset:08X}  ");
        for index in 0..16 {
            if let Some(byte) = chunk.get(index) {
                let _ = write!(output, "{byte:02X} ");
            } else {
                output.push_str("   ");
            }
        }
        output.push_str(" |");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                *byte as char
            } else {
                '.'
            };
            output.push(ch);
        }
        output.push('|');
        if offset + chunk.len() < bytes.len() {
            output.push('\n');
        }
    }
    output
}
