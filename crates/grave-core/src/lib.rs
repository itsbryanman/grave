mod container;
mod decay;
mod error;
mod model;
#[cfg(feature = "native")]
mod profiles;
#[cfg(feature = "native")]
mod render;
pub mod rng;

pub use container::{bury, exhume, inspect_grave, read_header, touch};
pub use decay::{
    decay_snapshot, inspect_decay_snapshot, prognosis, DecaySnapshot, DAY_SECONDS,
    TERMINAL_INTENSITY, TERMINAL_Q,
};
pub use error::GraveError;
pub use model::{
    BuryOptions, GraveFlags, GraveHeader, GraveInspection, RotProfile, FORMAT_VERSION, MAGIC_BYTES,
};
#[cfg(feature = "native")]
pub use render::{
    encode_png, render_grave, RenderResult, RenderedImage, RenderedPayload, RenderedText,
};
