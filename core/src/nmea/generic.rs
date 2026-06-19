use serde::{Deserialize, Serialize};

use crate::nmea::{NmeaMessageBorrowed, NmeaMessageEncodable, NmeaParseError};

/// A generic NMEA message that can be parsed from any NMEA sentence, without validating the sentence type or fields
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GenericNmeaMessage {
    /// The NMEA sentence type (e.g. GGA, RMC, etc.)
    pub sentence_type: String,

    /// The fields of the NMEA sentence, split by commas (e.g. for GGA, this would include time, latitude, longitude, etc.)
    pub fields: Vec<String>,

    /// The checksum computed over the sentence
    pub checksum: u8,
}

impl<'a> TryFrom<NmeaMessageBorrowed<'a>> for GenericNmeaMessage {
    type Error = NmeaParseError;

    fn try_from(value: NmeaMessageBorrowed<'a>) -> Result<Self, Self::Error> {
        let NmeaMessageBorrowed {
            sentence_type,
            fields,
            checksum,
        } = value;
        Ok(GenericNmeaMessage {
            sentence_type: sentence_type.to_string(),
            fields: fields.map(|s| s.to_string()).collect(),
            checksum,
        })
    }
}

/// [INTERNAL] implement [NmeaMessageEncodable] for [GenericNmeaMessage] to allow encoding into NMEA sentences
impl NmeaMessageEncodable for GenericNmeaMessage {
    fn encode_type(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        buffer
            .write_str(&self.sentence_type)
            .map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }

    fn encode_body(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        write!(buffer, "{}", self.fields.join(",")).map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::nmea::NMEA_PARSER_STRICT;

    use super::*;

    #[test]
    fn test_generic_message_parse_and_encode() {
        let raw_sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";

        let parsed = NMEA_PARSER_STRICT
            .parse_internal(raw_sentence)
            .expect("Failed to parse NMEA message");
        let generic_message =
            GenericNmeaMessage::try_from(parsed).expect("Failed to convert to GenericNmeaMessage");
        assert_eq!(generic_message.sentence_type, "GPGGA");
        assert_eq!(
            generic_message.fields,
            vec![
                "123519",
                "4807.038",
                "N",
                "01131.000",
                "E",
                "1",
                "08",
                "0.9",
                "545.4",
                "M",
                "46.9",
                "M",
                "",
                ""
            ]
        );
        assert_eq!(generic_message.checksum, 0x47);

        let mut encoded = String::new();
        NMEA_PARSER_STRICT
            .encode_internal(&generic_message, &mut encoded)
            .expect("Failed to encode NMEA message");
        assert_eq!(encoded, raw_sentence);
    }
}
