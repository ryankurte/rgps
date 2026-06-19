#[derive(Clone, PartialEq, Debug, thiserror::Error)]
pub enum NmeaParseError {
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Invalid checksum")]
    InvalidChecksum,
    #[error("Checksum mismatch (computed: 0x{0:02x}, provided: 0x{1:02x})")]
    ChecksumMismatch(u8, u8), // (computed, provided)
    #[error("Buffer write failed")]
    BufferWrite,
    #[error("Invalid message type")]
    InvalidMessageType,
    #[error("Invalid field count")]
    InvalidFieldCount,
    #[error("Missing field {0}")]
    MissingField(usize),
    #[error("Integer parse failed {0}")]
    IntegerParseError(#[from] core::num::ParseIntError),
    #[error("Float parse failed {0}")]
    FloatParseError(#[from] core::num::ParseFloatError),
    #[error("Enum variant not found")]
    EnumVariantNotFound,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

impl From<strum::ParseError> for NmeaParseError {
    fn from(_: strum::ParseError) -> Self {
        NmeaParseError::EnumVariantNotFound
    }
}
