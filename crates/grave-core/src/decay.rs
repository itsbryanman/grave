use crate::{GraveError, GraveHeader};

pub const DAY_SECONDS: u64 = 86_400;
pub const TERMINAL_INTENSITY: f64 = 0.985;
pub const TERMINAL_Q: u32 = 9_850;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecaySnapshot {
    pub age_days: u64,
    pub neglect_days: u64,
    pub effective_half_life_days: f64,
    pub intensity: f64,
    pub q: u32,
}

pub fn decay_snapshot(header: &GraveHeader, now: u64) -> Result<DecaySnapshot, GraveError> {
    decay_snapshot_with_mode(header, now, DecayMode::QuantizedDays)
}

pub fn inspect_decay_snapshot(header: &GraveHeader, now: u64) -> Result<DecaySnapshot, GraveError> {
    decay_snapshot_with_mode(header, now, DecayMode::Precise)
}

#[derive(Clone, Copy)]
enum DecayMode {
    QuantizedDays,
    Precise,
}

fn decay_snapshot_with_mode(
    header: &GraveHeader,
    now: u64,
    mode: DecayMode,
) -> Result<DecaySnapshot, GraveError> {
    if now < header.buried_at {
        return Err(GraveError::DateBeforeBurial);
    }
    if header.half_life_days == 0 {
        return Err(GraveError::InvalidHalfLife);
    }

    let age_seconds = now.saturating_sub(header.buried_at);
    let neglect_seconds = now.saturating_sub(header.last_opened);
    let age_days = quantized_day_delta(header.buried_at, now);
    let neglect_days = quantized_day_delta(header.last_opened, now);
    let (age_days_f64, neglect_days_f64) = match mode {
        DecayMode::QuantizedDays => (age_days as f64, neglect_days as f64),
        DecayMode::Precise => (
            age_seconds as f64 / DAY_SECONDS as f64,
            neglect_seconds as f64 / DAY_SECONDS as f64,
        ),
    };
    let effective_half_life_days =
        header.half_life_days as f64 * (1.0 + header.mourn_credit as f64 * 0.10);

    let base = 1.0 - 0.5f64.powf(age_days_f64 / effective_half_life_days);
    let neglect_boost = (neglect_days_f64 / (4.0 * effective_half_life_days)).min(0.25);
    let wear = (header.open_count as f64 * 0.002).min(0.10);
    let intensity = (base + neglect_boost + wear).clamp(0.0, 1.0);
    let q = (intensity * 10_000.0).floor().min(10_000.0) as u32;

    Ok(DecaySnapshot {
        age_days,
        neglect_days,
        effective_half_life_days,
        intensity,
        q,
    })
}

fn quantized_day_delta(earlier: u64, later: u64) -> u64 {
    later
        .saturating_div(DAY_SECONDS)
        .saturating_sub(earlier.saturating_div(DAY_SECONDS))
}

pub fn prognosis(header: &GraveHeader, now: u64) -> Result<u64, GraveError> {
    let current = decay_snapshot(header, now)?;
    if current.q >= TERMINAL_Q {
        return Ok(now);
    }

    let mut low_days = 0u64;
    let mut high_days = 1u64;
    let search_ceiling_days = 365u64 * 200;

    while high_days <= search_ceiling_days {
        let candidate_now = now.saturating_add(high_days.saturating_mul(DAY_SECONDS));
        if decay_snapshot(header, candidate_now)?.q >= TERMINAL_Q {
            break;
        }
        low_days = high_days;
        high_days = high_days.saturating_mul(2);
    }

    if high_days > search_ceiling_days {
        high_days = search_ceiling_days;
    }

    while low_days + 1 < high_days {
        let mid_days = low_days + (high_days - low_days) / 2;
        let candidate_now = now.saturating_add(mid_days.saturating_mul(DAY_SECONDS));
        if decay_snapshot(header, candidate_now)?.q >= TERMINAL_Q {
            high_days = mid_days;
        } else {
            low_days = mid_days;
        }
    }

    Ok(now.saturating_add(high_days.saturating_mul(DAY_SECONDS)))
}

#[cfg(test)]
mod tests {
    use super::{
        decay_snapshot, inspect_decay_snapshot, prognosis, quantized_day_delta, DAY_SECONDS,
        TERMINAL_Q,
    };
    use crate::{GraveFlags, GraveHeader, RotProfile, FORMAT_VERSION};

    fn header() -> GraveHeader {
        GraveHeader {
            version: FORMAT_VERSION,
            burial_id: [7; 32],
            buried_at: 1_720_000_000,
            last_opened: 1_720_000_000,
            open_count: 0,
            profile: RotProfile::Static,
            flags: GraveFlags::new(false),
            half_life_days: 30,
            mourn_credit: 0,
            epitaph: "ashes".to_string(),
            original_filename: "note.txt".to_string(),
            mimetype: "text/plain".to_string(),
        }
    }

    #[test]
    fn intensity_starts_at_zero() {
        let snapshot = decay_snapshot(&header(), 1_720_000_000).expect("snapshot");
        assert_eq!(snapshot.q, 0);
    }

    #[test]
    fn intensity_hits_half_life_midpoint() {
        let mut grave = header();
        grave.last_opened = grave.buried_at + 30 * DAY_SECONDS;
        let snapshot =
            decay_snapshot(&grave, grave.buried_at + 30 * DAY_SECONDS).expect("snapshot");
        assert_eq!(snapshot.q, 5_000);
    }

    #[test]
    fn wear_cap_stops_at_ten_percent() {
        let mut grave = header();
        grave.open_count = 500;
        let snapshot = decay_snapshot(&grave, grave.buried_at).expect("snapshot");
        assert_eq!(snapshot.q, 1_000);
    }

    #[test]
    fn intensity_clamps_at_terminal() {
        let mut grave = header();
        grave.open_count = 500;
        let snapshot =
            decay_snapshot(&grave, grave.buried_at + 120 * DAY_SECONDS).expect("snapshot");
        assert_eq!(snapshot.q, 10_000);
    }

    #[test]
    fn prognosis_moves_forward_until_terminal() {
        let grave = header();
        let terminal_at = prognosis(&grave, grave.buried_at).expect("prognosis");
        let snapshot = decay_snapshot(&grave, terminal_at).expect("terminal snapshot");
        assert!(snapshot.q >= TERMINAL_Q);
    }

    #[test]
    fn whole_day_quantization_ignores_intra_day_clock() {
        let mut grave = header();
        grave.buried_at = 100 * DAY_SECONDS + 3_600;
        grave.last_opened = 101 * DAY_SECONDS + 7_200;

        let morning = 104 * DAY_SECONDS + 300;
        let evening = 104 * DAY_SECONDS + 80_000;
        let morning_snapshot = decay_snapshot(&grave, morning).expect("morning snapshot");
        let evening_snapshot = decay_snapshot(&grave, evening).expect("evening snapshot");

        assert_eq!(morning_snapshot.q, evening_snapshot.q);
        assert_eq!(quantized_day_delta(grave.buried_at, morning), 4);
        assert_eq!(quantized_day_delta(grave.last_opened, morning), 3);
    }

    #[test]
    fn inspect_snapshot_preserves_fractional_progress() {
        let mut grave = header();
        grave.buried_at = 100 * DAY_SECONDS + 3_600;

        let morning = 104 * DAY_SECONDS + 300;
        let evening = 104 * DAY_SECONDS + 80_000;
        let morning_snapshot =
            inspect_decay_snapshot(&grave, morning).expect("morning inspect snapshot");
        let evening_snapshot =
            inspect_decay_snapshot(&grave, evening).expect("evening inspect snapshot");

        assert!(evening_snapshot.intensity > morning_snapshot.intensity);
    }
}
