#![cfg(target_arch = "wasm32")]

use grave_core::{
    bury, encode_png, touch_bytes, BuryOptions, RenderedImage, RotProfile, DAY_SECONDS,
};
use grave_wasm::{read_header, render_at};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use wasm_bindgen_test::wasm_bindgen_test;

const FIXTURE_BURIED_AT: u64 = 200 * DAY_SECONDS + 12_345;
const FIXTURE_NOW: u64 = 235 * DAY_SECONDS + 32_100;

const STATIC_IMAGE_HASH: &str = "da67dca1ad5be3a9fd0d9d09dd2a732b4d0e3cabe35f6061aa4a623acbbe6c1e";
const BURNIN_IMAGE_HASH: &str = "475b25f95c2bf43af391c2284ca1317c14f91a43387460341bfac5f9151017cc";
const MOLD_TEXT_HASH: &str = "f8e850da6825cf16eadd14476a2f4686e63973bd314ab458ccbd835a81f40b31";

#[wasm_bindgen_test]
fn read_header_matches_expected_fields() {
    let grave = bury(
        fixture_text().as_bytes(),
        fixture_options(RotProfile::Mold, "text/plain", "fixture.txt"),
    )
    .expect("bury");
    let header: HeaderResponse =
        serde_wasm_bindgen::from_value(read_header(&grave).expect("header")).expect("deserialize");

    assert_eq!(header.header.profile, "mold");
    assert_eq!(header.header.original_filename, "fixture.txt");
    assert_eq!(header.original_len, fixture_text().as_bytes().len() as u64);
    assert!(!header.disturbed);
}

#[wasm_bindgen_test]
fn static_image_hash_matches_the_native_golden() {
    let grave = bury(
        &fixture_image_bytes(),
        fixture_options(RotProfile::Static, "image/png", "fixture.png"),
    )
    .expect("bury");
    let render: RenderResponse =
        serde_wasm_bindgen::from_value(render_at(&grave, FIXTURE_NOW).expect("render"))
            .expect("deserialize");

    match render.payload {
        Payload::Image {
            rgba,
            width,
            height,
        } => {
            assert_eq!(hash_image(width, height, &rgba), STATIC_IMAGE_HASH);
        }
        _ => panic!("expected image payload"),
    }
}

#[wasm_bindgen_test]
fn burnin_image_hash_matches_the_native_golden() {
    let grave = grave_with_visits(RotProfile::BurnIn, "image/png", "fixture.png", 9);
    let render: RenderResponse =
        serde_wasm_bindgen::from_value(render_at(&grave, FIXTURE_NOW).expect("render"))
            .expect("deserialize");

    match render.payload {
        Payload::Image {
            rgba,
            width,
            height,
        } => {
            assert_eq!(hash_image(width, height, &rgba), BURNIN_IMAGE_HASH);
        }
        _ => panic!("expected image payload"),
    }
}

#[wasm_bindgen_test]
fn mold_text_hash_matches_the_native_golden() {
    let grave = bury(
        fixture_text().as_bytes(),
        fixture_options(RotProfile::Mold, "text/plain", "fixture.txt"),
    )
    .expect("bury");
    let render: RenderResponse =
        serde_wasm_bindgen::from_value(render_at(&grave, FIXTURE_NOW).expect("render"))
            .expect("deserialize");

    match render.payload {
        Payload::Text { text, .. } => {
            assert_eq!(hash_text(&text), MOLD_TEXT_HASH);
        }
        _ => panic!("expected text payload"),
    }
}

fn grave_with_visits(profile: RotProfile, mimetype: &str, filename: &str, visits: u32) -> Vec<u8> {
    let payload = if mimetype.starts_with("image/") {
        fixture_image_bytes()
    } else {
        fixture_text().into_bytes()
    };
    let mut grave = bury(&payload, fixture_options(profile, mimetype, filename)).expect("bury");
    let mut when = FIXTURE_BURIED_AT + DAY_SECONDS;

    for _ in 0..visits {
        grave = touch_bytes(&grave, when).expect("touch bytes");
        when += 2 * DAY_SECONDS;
    }

    grave
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

fn hash_image(width: u32, height: u32, rgba: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(rgba);
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

#[derive(Deserialize)]
struct HeaderResponse {
    header: HeaderView,
    disturbed: bool,
    original_len: u64,
}

#[derive(Deserialize)]
struct HeaderView {
    profile: String,
    original_filename: String,
}

#[derive(Deserialize)]
struct RenderResponse {
    payload: Payload,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Payload {
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    Text {
        text: String,
        #[serde(rename = "is_hex_dump")]
        _is_hex_dump: bool,
    },
    Terminal,
}
