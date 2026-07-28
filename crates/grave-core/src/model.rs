use crate::GraveError;

pub const MAGIC_BYTES: [u8; 8] = [0x47, 0x52, 0x41, 0x56, 0x45, 0x00, 0x66, 0x6F];
pub const FORMAT_VERSION: u16 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuryOptions {
    pub burial_id: [u8; 32],
    pub buried_at: u64,
    pub profile: RotProfile,
    pub hardcore: bool,
    pub half_life_days: u32,
    pub epitaph: String,
    pub original_filename: String,
    pub mimetype: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraveHeader {
    pub version: u16,
    pub burial_id: [u8; 32],
    pub buried_at: u64,
    pub last_opened: u64,
    pub open_count: u32,
    pub profile: RotProfile,
    pub flags: GraveFlags,
    pub half_life_days: u32,
    pub mourn_credit: u32,
    pub epitaph: String,
    pub original_filename: String,
    pub mimetype: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraveInspection {
    pub header: GraveHeader,
    pub disturbed: bool,
    pub compressed_len: u64,
    pub original_len: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RotProfile {
    Mold = 0x01,
    Static = 0x02,
    BurnIn = 0x03,
    DataLoss = 0x04,
}

impl RotProfile {
    pub fn as_byte(self) -> u8 {
        self as u8
    }

    pub fn from_byte(value: u8) -> Result<Self, GraveError> {
        match value {
            0x01 => Ok(Self::Mold),
            0x02 => Ok(Self::Static),
            0x03 => Ok(Self::BurnIn),
            0x04 => Ok(Self::DataLoss),
            other => Err(GraveError::InvalidProfile(other)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mold => "mold",
            Self::Static => "static",
            Self::BurnIn => "burnin",
            Self::DataLoss => "dataloss",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GraveFlags(u8);

impl GraveFlags {
    pub const HARDCORE: u8 = 0b0000_0001;
    pub const MOURNED_RECENTLY: u8 = 0b0000_0010;

    pub fn new(hardcore: bool) -> Self {
        let mut bits = 0u8;
        if hardcore {
            bits |= Self::HARDCORE;
        }
        Self(bits)
    }

    pub fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn hardcore(self) -> bool {
        self.0 & Self::HARDCORE != 0
    }

    pub fn mourned_recently(self) -> bool {
        self.0 & Self::MOURNED_RECENTLY != 0
    }

    pub fn set_mourned_recently(&mut self, mourned_recently: bool) {
        if mourned_recently {
            self.0 |= Self::MOURNED_RECENTLY;
        } else {
            self.0 &= !Self::MOURNED_RECENTLY;
        }
    }
}
