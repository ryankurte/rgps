use core::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::nmea::{NmeaMessageBorrowed, NmeaMessageEncodable, NmeaMessageType, NmeaParseError};

/// A specific NMEA message enum
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum SpecificNmeaMessage {
    // TODO: all the messages!
    /// Other / unrecognised message type, passes through the raw NMEA sentence
    Other {
        sentence_type: String,
        fields: Vec<String>,
        checksum: u8,
    },
}

impl<'a> TryFrom<NmeaMessageBorrowed<'a>> for SpecificNmeaMessage {
    type Error = NmeaParseError;

    fn try_from(value: NmeaMessageBorrowed<'a>) -> Result<Self, Self::Error> {
        let NmeaMessageBorrowed {
            sentence_type,
            fields,
            checksum,
        } = value;

        // Attempt to parse the sentence type into a known message type
        let _known_sentence_type = match NmeaMessageType::from_str(sentence_type) {
            Ok(t) => t,
            Err(_) => {
                return Ok(SpecificNmeaMessage::Other {
                    sentence_type: sentence_type.to_string(),
                    fields: fields.map(|s| s.to_string()).collect(),
                    checksum,
                });
            }
        };

        todo!()
    }
}

/// [INTERNAL] implement [NmeaMessageEncodable] for [NmeaMessage] to allow encoding into NMEA sentences
impl NmeaMessageEncodable for SpecificNmeaMessage {
    fn encode_type(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        match self {
            SpecificNmeaMessage::Other { sentence_type, .. } => {
                buffer
                    .write_str(sentence_type)
                    .map_err(|_| NmeaParseError::BufferWrite)?;
                Ok(())
            }
        }
    }

    fn encode_body(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        match self {
            SpecificNmeaMessage::Other { fields, .. } => {
                write!(buffer, "{}", fields.join(",")).map_err(|_| NmeaParseError::BufferWrite)?;
                Ok(())
            }
        }
    }
}
