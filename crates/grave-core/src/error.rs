use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraveError {
    #[error("This is no grave.")]
    BadMagic,
    #[error("This grave was sealed with newer rites (version {0}).")]
    UnsupportedVersion(u16),
    #[error("The grave is incomplete.")]
    Truncated,
    #[error("The {0} inscription could not be read.")]
    InvalidUtf8(&'static str),
    #[error("The {0} inscription is too long for this coffin.")]
    StringTooLong(&'static str),
    #[error("This grave bears an unknown rot profile (0x{0:02X}).")]
    InvalidProfile(u8),
    #[error("A grave cannot have a half-life of zero.")]
    InvalidHalfLife,
    #[error("The coffin would burst if exhumed past {limit} bytes.")]
    PayloadTooLarge { limit: usize },
    #[error("The remains do not match the burial record.")]
    CrcMismatch,
    #[error("The grave has been disturbed.")]
    Disturbed,
    #[error("The {0} rite has not yet taken hold here.")]
    RenderUnavailable(&'static str),
    #[error("This build cannot {0}.")]
    CodecUnavailable(&'static str),
    #[error("The dead do not return from consecrated ground.")]
    Hardcore,
    #[error("Nothing was here yet.")]
    DateBeforeBurial,
    #[cfg(any(feature = "native", feature = "wasm"))]
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
