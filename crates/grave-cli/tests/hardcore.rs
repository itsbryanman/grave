use std::process::Command;

use grave_core::{
    bury, encode_png, inspect_grave, BuryOptions, RenderedImage, RotProfile, DAY_SECONDS,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn exhume_returns_exit_66_for_hardcore_graves() {
    let dir = tempdir().expect("tempdir");
    let grave_path = dir.path().join("note.txt.grave");
    let grave = bury(
        b"kept under glass",
        hardcore_options("note.txt", "text/plain"),
    )
    .expect("bury");
    std::fs::write(&grave_path, grave).expect("write grave");

    let output = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("exhume")
        .arg(&grave_path)
        .output()
        .expect("run grave exhume");

    assert_eq!(output.status.code(), Some(66));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("The dead do not return from consecrated ground."));
}

#[test]
fn hardcore_open_compounds_and_rewrites_the_grave() {
    let dir = tempdir().expect("tempdir");
    let grave_path = dir.path().join("photo.grave");
    let first_output = dir.path().join("first.png");
    let second_output = dir.path().join("second.png");
    let now = chrono::Utc::now().timestamp().max(0) as u64;

    let mut options = hardcore_options("photo.png", "image/png");
    options.profile = RotProfile::BurnIn;
    options.buried_at = now.saturating_sub(40 * DAY_SECONDS);
    let original_grave = bury(&fixture_image_bytes(), options).expect("bury");
    std::fs::write(&grave_path, &original_grave).expect("write grave");

    let first = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("open")
        .arg(&grave_path)
        .arg("-o")
        .arg(&first_output)
        .output()
        .expect("first open");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let after_first = std::fs::read(&grave_path).expect("read grave");
    let first_inspection = inspect_grave(&after_first).expect("inspect");
    assert_eq!(first_inspection.header.open_count, 1);

    let second = Command::new(env!("CARGO_BIN_EXE_grave"))
        .arg("open")
        .arg(&grave_path)
        .arg("-o")
        .arg(&second_output)
        .output()
        .expect("second open");
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );

    let after_second = std::fs::read(&grave_path).expect("read grave");
    let second_inspection = inspect_grave(&after_second).expect("inspect");
    assert_eq!(second_inspection.header.open_count, 2);

    assert_ne!(hash_bytes(&original_grave), hash_bytes(&after_first));
    assert_ne!(hash_bytes(&after_first), hash_bytes(&after_second));
    assert_ne!(
        hash_bytes(&std::fs::read(&first_output).expect("first render")),
        hash_bytes(&std::fs::read(&second_output).expect("second render"))
    );
}

fn hardcore_options(filename: &str, mimetype: &str) -> BuryOptions {
    BuryOptions {
        burial_id: [0x44; 32],
        buried_at: 1_722_124_800,
        profile: RotProfile::Static,
        hardcore: true,
        half_life_days: 30,
        epitaph: "keep this one shut".to_string(),
        original_filename: filename.to_string(),
        mimetype: mimetype.to_string(),
    }
}

fn fixture_image_bytes() -> Vec<u8> {
    let width = 24u32;
    let height = 24u32;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let base = ((y * width + x) * 4) as usize;
            rgba[base] = ((x * 11 + y * 5) % 256) as u8;
            rgba[base + 1] = ((x * 7 + y * 13) % 256) as u8;
            rgba[base + 2] = ((x * y * 9 + 31) % 256) as u8;
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

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
