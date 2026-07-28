use grave_core::{
    bury, encode_png, render_grave, BuryOptions, RenderedImage, RenderedPayload, RotProfile,
    DAY_SECONDS,
};
use sha2::{Digest, Sha256};

const FIXTURE_BURIED_AT: u64 = 200 * DAY_SECONDS + 12_345;
const FIXTURE_NOW: u64 = 235 * DAY_SECONDS + 32_100;
const SAME_DAY_EARLY: u64 = 235 * DAY_SECONDS + 1_000;
const SAME_DAY_LATE: u64 = 235 * DAY_SECONDS + 80_000;

const STATIC_IMAGE_HASH: &str = "da67dca1ad5be3a9fd0d9d09dd2a732b4d0e3cabe35f6061aa4a623acbbe6c1e";
const DATALOSS_IMAGE_HASH: &str =
    "df5f3d0f4cb017e7b9f89edf344c1e2e1549ea841dc5a2faaa98befb3402b395";
const STATIC_TEXT_HASH: &str = "ca4583e2d58fa83f3d4d1aa809c0995a4b7ae7a2c4a9e669b350b0628a6a3225";
const DATALOSS_TEXT_HASH: &str = "0c3275dd653c3727e1a35ec98f192995d7ea86c045af35cb021fb32e7a002bf8";

#[test]
fn static_image_golden_is_stable() {
    let grave = bury(
        &fixture_image_bytes(),
        fixture_options(RotProfile::Static, "image/png", "fixture.png"),
    )
    .expect("bury");
    let render = render_grave(&grave, FIXTURE_NOW).expect("render");
    let RenderedPayload::Image(image) = render.payload else {
        panic!("expected image render");
    };

    assert_eq!(hash_image(&image), STATIC_IMAGE_HASH);
}

#[test]
fn dataloss_image_golden_is_stable() {
    let grave = bury(
        &fixture_image_bytes(),
        fixture_options(RotProfile::DataLoss, "image/png", "fixture.png"),
    )
    .expect("bury");
    let render = render_grave(&grave, FIXTURE_NOW).expect("render");
    let RenderedPayload::Image(image) = render.payload else {
        panic!("expected image render");
    };

    assert_eq!(hash_image(&image), DATALOSS_IMAGE_HASH);
}

#[test]
fn static_text_golden_is_stable() {
    let grave = bury(
        fixture_text().as_bytes(),
        fixture_options(RotProfile::Static, "text/plain", "fixture.txt"),
    )
    .expect("bury");
    let render = render_grave(&grave, FIXTURE_NOW).expect("render");
    let RenderedPayload::Text(text) = render.payload else {
        panic!("expected text render");
    };

    assert_eq!(hash_text(&text.body), STATIC_TEXT_HASH);
}

#[test]
fn dataloss_text_golden_is_stable() {
    let grave = bury(
        fixture_text().as_bytes(),
        fixture_options(RotProfile::DataLoss, "text/plain", "fixture.txt"),
    )
    .expect("bury");
    let render = render_grave(&grave, FIXTURE_NOW).expect("render");
    let RenderedPayload::Text(text) = render.payload else {
        panic!("expected text render");
    };

    assert_eq!(hash_text(&text.body), DATALOSS_TEXT_HASH);
}

#[test]
fn same_day_renders_share_the_same_seed_stream() {
    let grave = bury(
        &fixture_image_bytes(),
        fixture_options(RotProfile::DataLoss, "image/png", "fixture.png"),
    )
    .expect("bury");
    let early = render_grave(&grave, SAME_DAY_EARLY).expect("early render");
    let late = render_grave(&grave, SAME_DAY_LATE).expect("late render");

    let RenderedPayload::Image(early_image) = early.payload else {
        panic!("expected image render");
    };
    let RenderedPayload::Image(late_image) = late.payload else {
        panic!("expected image render");
    };

    assert_eq!(hash_image(&early_image), hash_image(&late_image));
}

fn fixture_options(profile: RotProfile, mimetype: &str, filename: &str) -> BuryOptions {
    BuryOptions {
        burial_id: [
            0x13, 0x37, 0x66, 0x6F, 0x72, 0x65, 0x76, 0x65, 0x72, 0x19, 0x84, 0x20, 0xAA, 0xBB,
            0xCC, 0xDD, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x20, 0x30, 0x40,
            0x50, 0x60, 0x70, 0x80,
        ],
        buried_at: FIXTURE_BURIED_AT,
        profile,
        hardcore: false,
        half_life_days: 20,
        epitaph: "we left this where the damp could find it".to_string(),
        original_filename: filename.to_string(),
        mimetype: mimetype.to_string(),
    }
}

fn fixture_image_bytes() -> Vec<u8> {
    let width = 48u32;
    let height = 40u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let base = ((y * width + x) * 4) as usize;
            let checker = ((x / 6 + y / 5) % 2) as u8;
            rgba[base] = ((x * 5 + y * 3) % 256) as u8;
            rgba[base + 1] = (((x * 13) ^ (y * 7)) % 256) as u8;
            rgba[base + 2] = ((x * y * 3 + y * 17 + checker as u32 * 61) % 256) as u8;
            rgba[base + 3] = 255;
        }
    }

    encode_png(&RenderedImage {
        rgba,
        width,
        height,
    })
    .expect("encode png")
}

fn fixture_text() -> String {
    [
        "They catalogued the room, then argued about whether the damp had a memory. Nobody agreed.",
        "A photograph leaned against the ledger. The faces were still visible, but only if you looked sideways.",
        "",
        "By the third night the labels had curled in on themselves. Every box sounded heavier than it was.",
        "Someone whispered that opening the lid too often made the hinge remember you.",
        "",
        "The final inventory ended with a question mark. It was the most honest mark in the building.",
    ]
    .join("\n")
}

fn hash_image(image: &RenderedImage) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image.width.to_le_bytes());
    hasher.update(image.height.to_le_bytes());
    hasher.update(&image.rgba);
    hex_string(&hasher.finalize())
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex_string(&hasher.finalize())
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
