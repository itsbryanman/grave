mod dataloss;
mod static_;

use rand_chacha::ChaCha8Rng;

use crate::{GraveError, RotProfile};

#[derive(Clone, Copy, Debug)]
pub struct RotContext {
    pub q: u32,
    pub age_days: u64,
    pub neglect_days: u64,
    pub open_count: u32,
}

pub fn rot_image(
    profile: RotProfile,
    rgba: &mut [u8],
    width: u32,
    height: u32,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) -> Result<(), GraveError> {
    match profile {
        RotProfile::Static => {
            static_::rot_image(rgba, width, height, context, rng);
            Ok(())
        }
        RotProfile::DataLoss => {
            dataloss::rot_image(rgba, width, height, context, rng);
            Ok(())
        }
        RotProfile::Mold => Err(GraveError::RenderUnavailable("mold")),
        RotProfile::BurnIn => Err(GraveError::RenderUnavailable("burnin")),
    }
}

pub fn rot_text(
    profile: RotProfile,
    text: &str,
    context: &RotContext,
    rng: &mut ChaCha8Rng,
) -> Result<String, GraveError> {
    match profile {
        RotProfile::Static => Ok(static_::rot_text(text, context, rng)),
        RotProfile::DataLoss => Ok(dataloss::rot_text(text, context, rng)),
        RotProfile::Mold => Err(GraveError::RenderUnavailable("mold")),
        RotProfile::BurnIn => Err(GraveError::RenderUnavailable("burnin")),
    }
}

pub(super) fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

pub(super) fn has_dead_neighbor(
    dead: &[bool],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> bool {
    let y_start = y.saturating_sub(1);
    let y_end = (y + 1).min(height.saturating_sub(1));
    let x_start = x.saturating_sub(1);
    let x_end = (x + 1).min(width.saturating_sub(1));

    for ny in y_start..=y_end {
        for nx in x_start..=x_end {
            if nx == x && ny == y {
                continue;
            }
            if dead[ny * width + nx] {
                return true;
            }
        }
    }
    false
}
