use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use crate::{
    BuryOptions, GraveError, GraveFlags, GraveHeader, GraveInspection, MournOutcome, RotProfile,
    DAY_SECONDS, FORMAT_VERSION, MAGIC_BYTES, MOURNING_WINDOW_DAYS,
};

const LAST_OPENED_OFFSET: usize = 50;
const MUTABLE_RANGE_START: usize = LAST_OPENED_OFFSET;
const MUTABLE_RANGE_END: usize = 72;
const MAX_DECOMPRESSED_BYTES: usize = 512 * 1024 * 1024;
const INITIAL_HEADER_READ: usize = 4 * 1024;
const MAX_HEADER_BYTES: usize = 256 * 1024;

#[derive(Debug)]
struct ParsedHeaderPrefix {
    header: GraveHeader,
    original_len: u64,
    payload_offset: usize,
}

#[derive(Debug)]
pub(crate) struct ParsedGrave<'a> {
    pub(crate) header: GraveHeader,
    pub(crate) disturbed: bool,
    pub(crate) compressed_len: usize,
    pub(crate) original_len: u64,
    pub(crate) payload: &'a [u8],
    pub(crate) payload_crc32: u32,
}

pub fn bury(input: &[u8], options: BuryOptions) -> Result<Vec<u8>, GraveError> {
    if options.half_life_days == 0 {
        return Err(GraveError::InvalidHalfLife);
    }
    if input.len() > MAX_DECOMPRESSED_BYTES {
        return Err(GraveError::PayloadTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }

    let compressed = compress_payload(input)?;

    let header = GraveHeader {
        version: FORMAT_VERSION,
        burial_id: options.burial_id,
        buried_at: options.buried_at,
        last_opened: options.buried_at,
        open_count: 0,
        profile: options.profile,
        flags: GraveFlags::new(options.hardcore),
        half_life_days: options.half_life_days,
        mourn_credit: 0,
        epitaph: options.epitaph,
        original_filename: options.original_filename,
        mimetype: options.mimetype,
    };

    let mut bytes = serialize_header_without_payload(&header, input.len() as u64)?;
    let header_crc32 = crc32fast::hash(&bytes);
    bytes.extend_from_slice(&compressed);
    bytes.extend_from_slice(&crc32fast::hash(input).to_le_bytes());
    bytes.extend_from_slice(&header_crc32.to_le_bytes());
    Ok(bytes)
}

pub fn read_header(bytes: &[u8]) -> Result<GraveHeader, GraveError> {
    Ok(parse_header_prefix(bytes)?.header)
}

pub fn inspect_grave(bytes: &[u8]) -> Result<GraveInspection, GraveError> {
    let parsed = parse_grave(bytes)?;
    Ok(GraveInspection {
        header: parsed.header,
        disturbed: parsed.disturbed,
        compressed_len: parsed.compressed_len as u64,
        original_len: parsed.original_len,
    })
}

pub fn inspect_grave_file(file: &mut File) -> Result<GraveInspection, GraveError> {
    let file_len = usize::try_from(file.metadata()?.len()).map_err(|_| GraveError::Truncated)?;
    let (header_bytes, parsed) = read_header_prefix_from_file(file)?;
    if file_len < parsed.payload_offset + 8 {
        return Err(GraveError::Truncated);
    }

    let stored_header_crc32 = read_trailing_u32(file)?;
    let computed_header_crc32 = crc32fast::hash(&header_bytes[..parsed.payload_offset]);
    Ok(GraveInspection {
        header: parsed.header,
        disturbed: stored_header_crc32 != computed_header_crc32,
        compressed_len: (file_len - parsed.payload_offset - 8) as u64,
        original_len: parsed.original_len,
    })
}

pub fn exhume(bytes: &[u8]) -> Result<Vec<u8>, GraveError> {
    let parsed = parse_grave(bytes)?;
    if parsed.header.flags.hardcore() {
        return Err(GraveError::Hardcore);
    }

    let output = decompress_payload(parsed.payload, parsed.original_len)?;
    if crc32fast::hash(&output) != parsed.payload_crc32 {
        return Err(GraveError::CrcMismatch);
    }
    Ok(output)
}

pub fn reinter(bytes: &[u8], payload: &[u8], now: u64) -> Result<Vec<u8>, GraveError> {
    let parsed = parse_grave(bytes)?;
    if now < parsed.header.buried_at {
        return Err(GraveError::DateBeforeBurial);
    }
    if payload.len() > MAX_DECOMPRESSED_BYTES {
        return Err(GraveError::PayloadTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }

    let compressed = compress_payload(payload)?;
    let mut header = parsed.header;
    header.last_opened = now;
    header.open_count = header.open_count.saturating_add(1);
    header.flags.set_mourned_recently(false);

    let mut bytes = serialize_header_without_payload(&header, payload.len() as u64)?;
    let header_crc32 = crc32fast::hash(&bytes);
    bytes.extend_from_slice(&compressed);
    bytes.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    bytes.extend_from_slice(&header_crc32.to_le_bytes());
    Ok(bytes)
}

pub fn touch(file: &mut File, now: u64) -> Result<(), GraveError> {
    patch_mutable_header(file, now, |header| {
        header.last_opened = now;
        header.open_count = header.open_count.saturating_add(1);
        header.flags.set_mourned_recently(false);
        ((), true)
    })
}

pub fn touch_bytes(bytes: &[u8], now: u64) -> Result<Vec<u8>, GraveError> {
    let parsed = parse_grave(bytes)?;
    if parsed.disturbed {
        return Err(GraveError::Disturbed);
    }
    if now < parsed.header.buried_at {
        return Err(GraveError::DateBeforeBurial);
    }

    let mut header = parsed.header;
    header.last_opened = now;
    header.open_count = header.open_count.saturating_add(1);
    header.flags.set_mourned_recently(false);

    let mut updated = serialize_header_without_payload(&header, parsed.original_len)?;
    let header_crc32 = crc32fast::hash(&updated);
    updated.extend_from_slice(parsed.payload);
    updated.extend_from_slice(&parsed.payload_crc32.to_le_bytes());
    updated.extend_from_slice(&header_crc32.to_le_bytes());
    Ok(updated)
}

pub fn mourn(file: &mut File, now: u64) -> Result<MournOutcome, GraveError> {
    patch_mutable_header(file, now, |header| {
        let mourning_window = MOURNING_WINDOW_DAYS.saturating_mul(DAY_SECONDS);
        if header.flags.mourned_recently()
            && now.saturating_sub(header.last_opened) < mourning_window
        {
            return (MournOutcome::AlreadyMournedRecently, false);
        }

        header.last_opened = now;
        header.mourn_credit = header.mourn_credit.saturating_add(1).min(20);
        header.flags.set_mourned_recently(true);
        (
            MournOutcome::PaidRespects {
                mourn_credit: header.mourn_credit,
            },
            true,
        )
    })
}

pub(crate) fn parse_grave(bytes: &[u8]) -> Result<ParsedGrave<'_>, GraveError> {
    let prefix = parse_header_prefix(bytes)?;
    if bytes.len() < prefix.payload_offset + 8 {
        return Err(GraveError::Truncated);
    }

    let payload_end = bytes.len() - 8;
    if payload_end < prefix.payload_offset {
        return Err(GraveError::Truncated);
    }

    let payload = &bytes[prefix.payload_offset..payload_end];
    let payload_crc32 = u32::from_le_bytes([
        bytes[payload_end],
        bytes[payload_end + 1],
        bytes[payload_end + 2],
        bytes[payload_end + 3],
    ]);
    let stored_header_crc32 = u32::from_le_bytes([
        bytes[payload_end + 4],
        bytes[payload_end + 5],
        bytes[payload_end + 6],
        bytes[payload_end + 7],
    ]);
    let computed_header_crc32 = crc32fast::hash(&bytes[..prefix.payload_offset]);

    Ok(ParsedGrave {
        header: prefix.header,
        disturbed: stored_header_crc32 != computed_header_crc32,
        compressed_len: payload.len(),
        original_len: prefix.original_len,
        payload,
        payload_crc32,
    })
}

fn parse_header_prefix(bytes: &[u8]) -> Result<ParsedHeaderPrefix, GraveError> {
    let mut cursor = 0usize;

    let magic = read_exact(bytes, &mut cursor, MAGIC_BYTES.len())?;
    if magic != MAGIC_BYTES {
        return Err(GraveError::BadMagic);
    }

    let version = read_u16(bytes, &mut cursor)?;
    if version != FORMAT_VERSION {
        return Err(GraveError::UnsupportedVersion(version));
    }

    let burial_id = read_array::<32>(bytes, &mut cursor)?;
    let buried_at = read_u64(bytes, &mut cursor)?;
    let last_opened = read_u64(bytes, &mut cursor)?;
    let open_count = read_u32(bytes, &mut cursor)?;
    let profile = RotProfile::from_byte(read_u8(bytes, &mut cursor)?)?;
    let flags = GraveFlags::from_bits(read_u8(bytes, &mut cursor)?);
    let half_life_days = read_u32(bytes, &mut cursor)?;
    let mourn_credit = read_u32(bytes, &mut cursor)?;
    let epitaph = read_string(bytes, &mut cursor, "epitaph")?;
    let original_filename = read_string(bytes, &mut cursor, "filename")?;
    let mimetype = read_string(bytes, &mut cursor, "mimetype")?;
    let original_len = read_u64(bytes, &mut cursor)?;

    Ok(ParsedHeaderPrefix {
        header: GraveHeader {
            version,
            burial_id,
            buried_at,
            last_opened,
            open_count,
            profile,
            flags,
            half_life_days,
            mourn_credit,
            epitaph,
            original_filename,
            mimetype,
        },
        original_len,
        payload_offset: cursor,
    })
}

fn serialize_header_without_payload(
    header: &GraveHeader,
    original_len: u64,
) -> Result<Vec<u8>, GraveError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC_BYTES);
    bytes.extend_from_slice(&header.version.to_le_bytes());
    bytes.extend_from_slice(&header.burial_id);
    bytes.extend_from_slice(&header.buried_at.to_le_bytes());
    bytes.extend_from_slice(&header.last_opened.to_le_bytes());
    bytes.extend_from_slice(&header.open_count.to_le_bytes());
    bytes.push(header.profile.as_byte());
    bytes.push(header.flags.bits());
    bytes.extend_from_slice(&header.half_life_days.to_le_bytes());
    bytes.extend_from_slice(&header.mourn_credit.to_le_bytes());
    write_string(&mut bytes, &header.epitaph, "epitaph")?;
    write_string(&mut bytes, &header.original_filename, "filename")?;
    write_string(&mut bytes, &header.mimetype, "mimetype")?;
    bytes.extend_from_slice(&original_len.to_le_bytes());
    Ok(bytes)
}

fn write_string(target: &mut Vec<u8>, value: &str, field: &'static str) -> Result<(), GraveError> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| GraveError::StringTooLong(field))?;
    target.extend_from_slice(&len.to_le_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}

fn read_array<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], GraveError> {
    let slice = read_exact(bytes, cursor, N)?;
    let mut array = [0u8; N];
    array.copy_from_slice(slice);
    Ok(array)
}

fn read_string(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> Result<String, GraveError> {
    let len = read_u16(bytes, cursor)? as usize;
    let raw = read_exact(bytes, cursor, len)?;
    String::from_utf8(raw.to_vec()).map_err(|_| GraveError::InvalidUtf8(field))
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, GraveError> {
    Ok(read_exact(bytes, cursor, 1)?[0])
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, GraveError> {
    let raw = read_exact(bytes, cursor, 2)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, GraveError> {
    let raw = read_exact(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, GraveError> {
    let raw = read_exact(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

fn read_exact<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], GraveError> {
    let end = (*cursor).checked_add(len).ok_or(GraveError::Truncated)?;
    let slice = bytes.get(*cursor..end).ok_or(GraveError::Truncated)?;
    *cursor = end;
    Ok(slice)
}

fn read_header_prefix_from_file(
    file: &mut File,
) -> Result<(Vec<u8>, ParsedHeaderPrefix), GraveError> {
    let file_len = usize::try_from(file.metadata()?.len()).map_err(|_| GraveError::Truncated)?;
    let max_read = file_len.min(MAX_HEADER_BYTES);
    let mut read_len = INITIAL_HEADER_READ.min(max_read);

    loop {
        let mut header_bytes = Vec::with_capacity(read_len);
        file.rewind()?;
        std::io::Read::by_ref(file)
            .take(read_len as u64)
            .read_to_end(&mut header_bytes)?;

        match parse_header_prefix(&header_bytes) {
            Ok(parsed) => return Ok((header_bytes, parsed)),
            Err(GraveError::Truncated) if read_len < max_read => {
                read_len = (read_len.saturating_mul(2)).min(max_read);
            }
            Err(error) => return Err(error),
        }
    }
}

fn read_trailing_u32(file: &mut File) -> Result<u32, GraveError> {
    let mut raw = [0u8; 4];
    file.seek(SeekFrom::End(-4))?;
    file.read_exact(&mut raw)?;
    Ok(u32::from_le_bytes(raw))
}

#[cfg(feature = "native")]
fn compress_payload(payload: &[u8]) -> Result<Vec<u8>, GraveError> {
    Ok(zstd::stream::encode_all(Cursor::new(payload), 19)?)
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
fn compress_payload(_payload: &[u8]) -> Result<Vec<u8>, GraveError> {
    Ok(ruzstd::encoding::compress_to_vec(
        Cursor::new(_payload),
        ruzstd::encoding::CompressionLevel::Fastest,
    ))
}

fn patch_mutable_header<T, F>(file: &mut File, now: u64, mutate: F) -> Result<T, GraveError>
where
    F: FnOnce(&mut GraveHeader) -> (T, bool),
{
    let (header_bytes, parsed) = read_header_prefix_from_file(file)?;
    let stored_header_crc32 = read_trailing_u32(file)?;
    let computed_header_crc32 = crc32fast::hash(&header_bytes[..parsed.payload_offset]);

    if stored_header_crc32 != computed_header_crc32 {
        return Err(GraveError::Disturbed);
    }

    let mut header = parsed.header;
    if now < header.buried_at {
        return Err(GraveError::DateBeforeBurial);
    }

    let (result, changed) = mutate(&mut header);
    if !changed {
        return Ok(result);
    }

    let header_bytes = serialize_header_without_payload(&header, parsed.original_len)?;
    let new_header_crc32 = crc32fast::hash(&header_bytes);

    file.seek(SeekFrom::Start(MUTABLE_RANGE_START as u64))?;
    file.write_all(&header_bytes[MUTABLE_RANGE_START..MUTABLE_RANGE_END])?;
    file.seek(SeekFrom::End(-4))?;
    file.write_all(&new_header_crc32.to_le_bytes())?;
    file.sync_data()?;
    Ok(result)
}

#[cfg(feature = "native")]
pub(crate) fn decompress_payload(payload: &[u8], expected_len: u64) -> Result<Vec<u8>, GraveError> {
    let expected_len = usize::try_from(expected_len).map_err(|_| GraveError::PayloadTooLarge {
        limit: MAX_DECOMPRESSED_BYTES,
    })?;
    if expected_len > MAX_DECOMPRESSED_BYTES {
        return Err(GraveError::PayloadTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }

    let mut decoder = zstd::stream::read::Decoder::new(Cursor::new(payload))?;
    let mut output = Vec::with_capacity(expected_len.min(64 * 1024));
    let mut chunk = [0u8; 8 * 1024];

    loop {
        let read = decoder.read(&mut chunk)?;
        if read == 0 {
            break;
        }

        if output.len().saturating_add(read) > expected_len {
            return Err(GraveError::CrcMismatch);
        }
        output.extend_from_slice(&chunk[..read]);
    }

    if output.len() != expected_len {
        return Err(GraveError::CrcMismatch);
    }

    Ok(output)
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
pub(crate) fn decompress_payload(payload: &[u8], expected_len: u64) -> Result<Vec<u8>, GraveError> {
    let expected_len = usize::try_from(expected_len).map_err(|_| GraveError::PayloadTooLarge {
        limit: MAX_DECOMPRESSED_BYTES,
    })?;
    if expected_len > MAX_DECOMPRESSED_BYTES {
        return Err(GraveError::PayloadTooLarge {
            limit: MAX_DECOMPRESSED_BYTES,
        });
    }

    let mut decoder = ruzstd::decoding::StreamingDecoder::new_with_max_window_size(
        Cursor::new(payload),
        MAX_DECOMPRESSED_BYTES as u64,
    )
    .map_err(codec_error)?;
    let mut output = Vec::with_capacity(expected_len.min(64 * 1024));
    let mut chunk = [0u8; 8 * 1024];

    loop {
        let read = decoder.read(&mut chunk)?;
        if read == 0 {
            break;
        }

        if output.len().saturating_add(read) > expected_len {
            return Err(GraveError::CrcMismatch);
        }
        output.extend_from_slice(&chunk[..read]);
    }

    if output.len() != expected_len {
        return Err(GraveError::CrcMismatch);
    }

    Ok(output)
}

#[cfg(all(feature = "wasm", not(feature = "native")))]
fn codec_error(error: impl ToString) -> GraveError {
    GraveError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};
    use tempfile::NamedTempFile;

    use super::{
        bury, exhume, inspect_grave, inspect_grave_file, mourn, read_header, reinter, touch,
        touch_bytes,
    };
    use crate::{BuryOptions, MournOutcome, RotProfile, DAY_SECONDS, FORMAT_VERSION};

    fn options() -> BuryOptions {
        BuryOptions {
            burial_id: [9; 32],
            buried_at: 1_722_124_800,
            profile: RotProfile::DataLoss,
            hardcore: false,
            half_life_days: 30,
            epitaph: "not forgotten".to_string(),
            original_filename: "note.txt".to_string(),
            mimetype: "text/plain".to_string(),
        }
    }

    #[test]
    fn bury_roundtrip_and_header_readback() {
        let input = b"we become what we keep";
        let bytes = bury(input, options()).expect("bury");
        let header = read_header(&bytes).expect("header");
        let inspection = inspect_grave(&bytes).expect("inspection");
        assert_eq!(header.version, FORMAT_VERSION);
        assert_eq!(header.open_count, 0);
        assert_eq!(header.profile, RotProfile::DataLoss);
        assert_eq!(inspection.original_len, input.len() as u64);
        assert!(inspection.compressed_len > 0);
        assert_eq!(exhume(&bytes).expect("exhume"), input);
    }

    #[test]
    fn disturbance_does_not_block_exhumation() {
        let input = b"quiet archive";
        let mut bytes = bury(input, options()).expect("bury");
        bytes[42] ^= 0x01;

        let inspection = inspect_grave(&bytes).expect("inspection");
        assert!(inspection.disturbed);
        assert_eq!(exhume(&bytes).expect("exhume"), input);
    }

    #[test]
    fn touch_updates_header_without_touching_payload() {
        let input = b"memento";
        let bytes = bury(input, options()).expect("bury");
        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");

        {
            let mut file = temp.reopen().expect("reopen");
            touch(&mut file, 1_722_211_200).expect("touch");
        }

        let updated = std::fs::read(temp.path()).expect("read");
        let inspection = inspect_grave(&updated).expect("inspection");
        assert_eq!(inspection.header.open_count, 1);
        assert_eq!(inspection.header.last_opened, 1_722_211_200);
        assert_eq!(exhume(&updated).expect("exhume"), input);
    }

    #[test]
    fn inspect_grave_file_matches_in_memory_inspection() {
        let bytes = bury(b"memento", options()).expect("bury");
        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");

        let inspection = {
            let mut file = temp.reopen().expect("reopen");
            inspect_grave_file(&mut file).expect("inspect file")
        };
        let from_bytes = inspect_grave(&bytes).expect("inspect bytes");

        assert_eq!(inspection, from_bytes);
    }

    #[test]
    fn highly_compressible_payload_still_buries() {
        let input = vec![b'A'; 512 * 1024];
        let bytes = bury(&input, options()).expect("bury");
        let inspection = inspect_grave(&bytes).expect("inspection");
        assert!(inspection.compressed_len < 8 * 1024);
        assert_eq!(inspection.original_len, input.len() as u64);
        assert_eq!(exhume(&bytes).expect("exhume"), input);
    }

    #[test]
    fn touch_handles_headers_larger_than_the_initial_read_window() {
        let mut options = options();
        options.epitaph = "a".repeat(5_000);
        options.original_filename = "b".repeat(256);
        let bytes = bury(b"small", options).expect("bury");
        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");

        {
            let mut file = temp.reopen().expect("reopen");
            touch(&mut file, 1_722_300_000).expect("touch");
        }

        let updated = std::fs::read(temp.path()).expect("read");
        let inspection = inspect_grave(&updated).expect("inspection");
        assert_eq!(inspection.header.open_count, 1);
        assert_eq!(inspection.header.last_opened, 1_722_300_000);
    }

    #[test]
    fn mourning_is_rate_limited_for_a_week() {
        let bytes = bury(b"small", options()).expect("bury");
        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");

        {
            let mut file = temp.reopen().expect("reopen");
            let outcome = mourn(&mut file, 1_722_211_200).expect("first mourn");
            assert_eq!(outcome, MournOutcome::PaidRespects { mourn_credit: 1 });
        }

        let after_first = std::fs::read(temp.path()).expect("read");
        let first_inspection = inspect_grave(&after_first).expect("inspection");
        assert_eq!(first_inspection.header.mourn_credit, 1);
        assert!(first_inspection.header.flags.mourned_recently());

        {
            let mut file = temp.reopen().expect("reopen");
            let outcome = mourn(&mut file, 1_722_211_200 + 3 * DAY_SECONDS).expect("second mourn");
            assert_eq!(outcome, MournOutcome::AlreadyMournedRecently);
        }

        let after_second = std::fs::read(temp.path()).expect("read");
        let second_inspection = inspect_grave(&after_second).expect("inspection");
        assert_eq!(second_inspection.header.mourn_credit, 1);
        assert_eq!(
            second_inspection.header.last_opened,
            first_inspection.header.last_opened
        );
        assert!(second_inspection.header.flags.mourned_recently());
    }

    #[test]
    fn opening_clears_recent_mourning_flag() {
        let bytes = bury(b"small", options()).expect("bury");
        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");

        {
            let mut file = temp.reopen().expect("reopen");
            mourn(&mut file, 1_722_211_200).expect("mourn");
        }
        {
            let mut file = temp.reopen().expect("reopen");
            touch(&mut file, 1_722_211_200 + DAY_SECONDS).expect("touch");
        }

        let updated = std::fs::read(temp.path()).expect("read");
        let inspection = inspect_grave(&updated).expect("inspection");
        assert_eq!(inspection.header.open_count, 1);
        assert!(!inspection.header.flags.mourned_recently());
    }

    #[test]
    fn touch_bytes_matches_the_file_touch_path() {
        let bytes = bury(b"small", options()).expect("bury");
        let from_bytes = touch_bytes(&bytes, 1_722_211_200).expect("touch bytes");

        let temp = NamedTempFile::new().expect("tempfile");
        std::fs::write(temp.path(), &bytes).expect("write");
        {
            let mut file = temp.reopen().expect("reopen");
            touch(&mut file, 1_722_211_200).expect("touch");
        }
        let from_file = std::fs::read(temp.path()).expect("read");

        assert_eq!(from_bytes, from_file);
    }

    #[test]
    fn hardcore_reinterment_replaces_the_payload_and_keeps_the_original_burial_date() {
        let mut options = options();
        options.hardcore = true;
        let first = bury(b"first body", options).expect("bury");
        let second = reinter(&first, b"second body", 1_722_300_000).expect("reinter");
        let third = reinter(&second, b"third body", 1_722_400_000).expect("reinter");

        let second_inspection = inspect_grave(&second).expect("inspection");
        let third_inspection = inspect_grave(&third).expect("inspection");

        assert_eq!(second_inspection.header.buried_at, 1_722_124_800);
        assert_eq!(third_inspection.header.buried_at, 1_722_124_800);
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_eq!(second_inspection.header.open_count, 1);
        assert_eq!(third_inspection.header.open_count, 2);
    }

    #[test]
    fn randomized_bytes_survive_interment() {
        let mut rng = ChaCha8Rng::from_seed([3; 32]);
        for len in [0usize, 1, 7, 32, 255, 1024, 2048] {
            let mut input = vec![0u8; len];
            rng.fill_bytes(&mut input);
            let bytes = bury(&input, options()).expect("bury");
            let exhumed = exhume(&bytes).expect("exhume");
            assert_eq!(exhumed, input);
        }
    }
}
