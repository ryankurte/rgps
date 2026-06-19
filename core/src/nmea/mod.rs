use std::str::Split;

use itertools::Itertools;

mod consts;
pub use consts::{NmeaMessageType, TalkerId};

mod generic;
pub use generic::GenericNmeaMessage;

mod specific;
pub use specific::SpecificNmeaMessage;

mod messages;

mod parser;
pub use parser::{NMEA_PARSER_LENIENT, NMEA_PARSER_STRICT, NmeaParser};

mod error;
pub use error::NmeaParseError;

/// Trait for types that can be parsed from and encoded to an NMEA message using the NmeaParser
pub trait NmeaMessage: Sized + 'static {
    /// Parse an NMEA message into this type using the provided parser
    fn parse(parser: &NmeaParser, message: &str) -> Result<Self, NmeaParseError>;

    /// Encode an NMEA message into a string buffer
    fn encode(
        parser: &NmeaParser,
        message: &Self,
        buffer: impl core::fmt::Write,
    ) -> Result<(), NmeaParseError>;
}

impl<T> NmeaMessage for T
where
    for<'a> T: TryFrom<NmeaMessageBorrowed<'a>, Error = NmeaParseError>
        + NmeaMessageEncodable
        + Sized
        + 'static,
{
    /// Parse an NMEA message into [T] using the provided parser
    fn parse(parser: &NmeaParser, message: &str) -> Result<Self, NmeaParseError> {
        let borrowed = parser.parse_internal(message)?;
        Self::try_from(borrowed)
    }

    /// Encode an NMEA message into a string buffer using the provided parser
    fn encode(
        parser: &NmeaParser,
        message: &Self,
        buffer: impl core::fmt::Write,
    ) -> Result<(), NmeaParseError> {
        parser.encode_internal(message, buffer)
    }
}

/// A NMEA message object that can encode itself into a string buffer
pub(crate) trait NmeaMessageEncodable {
    /// Encode the message type into the provided buffer
    fn encode_type(&self, buffer: impl core::fmt::Write) -> Result<(), NmeaParseError>;

    /// Encode the message body into the provided buffer
    fn encode_body(&self, buffer: impl core::fmt::Write) -> Result<(), NmeaParseError>;
}

/// A view into a parsed NMEA message, providing access to fields without cloning the underlying data
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct NmeaMessageBorrowed<'a, F: Iterator<Item = &'a str> = Split<'a, char>> {
    /// The NMEA sentence type (e.g. GGA, RMC, etc.)
    pub(crate) sentence_type: &'a str,
    /// The fields of the NMEA sentence, split by commas (e.g. for GGA, this would include time, latitude, longitude, etc.)
    pub(crate) fields: F,
    /// The checksum computed over the sentence
    pub(crate) checksum: u8,
}

/// Allow an [NmeaMessageBorrowed] to be directly re-encoded
impl<'a, F: Iterator<Item = &'a str> + Clone> NmeaMessageEncodable for NmeaMessageBorrowed<'a, F> {
    fn encode_type(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        buffer
            .write_str(self.sentence_type)
            .map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }

    fn encode_body(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        #[allow(unstable_name_collisions)]
        for i in self.fields.clone().intersperse(",") {
            buffer
                .write_str(i)
                .map_err(|_| NmeaParseError::BufferWrite)?;
        }
        Ok(())
    }
}
