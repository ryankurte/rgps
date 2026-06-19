use std::{fmt::Write, str::Split};

use crate::nmea::{NmeaMessageBorrowed, NmeaMessageEncodable};

use super::NmeaParseError;

/// A simple NMEA sentence parser that can be used to parse raw NMEA sentences into structured messages
#[derive(Clone, PartialEq, Debug)]
pub struct NmeaParser {
    /// Require a valid checksum when parsing messages
    require_checksum: bool,
    /// Include the checksum when encoding messages
    include_checksum: bool,
    /// Include newlines when encoding messages
    include_newline: bool,
}

/// Strict NMEA parser, requring a valid checksum for all sentences
///
/// This is useful for parsing messages from real devices etc.
pub const NMEA_PARSER_STRICT: NmeaParser = NmeaParser {
    require_checksum: true,
    include_checksum: true,
    include_newline: false,
};

/// Lenient NMEA parser, which does not require a checksum (but will still validate it if provided)
///
/// This is useful for parsing human-generated sentences where we can dynamically generate checksums instead
/// of requiring users to provide them.
pub const NMEA_PARSER_LENIENT: NmeaParser = NmeaParser {
    require_checksum: false,
    include_checksum: true,
    include_newline: false,
};

impl NmeaParser {
    /// Parse an NMEA sentence into a structured message
    ///
    /// NOTE: this expects a string starting with '$' and ending with the checksum (e.g. "*hh")
    pub(crate) fn parse_internal<'a>(
        &self,
        sentence: &'a str,
    ) -> Result<NmeaMessageBorrowed<'a, Split<'a, char>>, NmeaParseError> {
        // Validate the sentence format
        if !sentence.starts_with('$') {
            return Err(NmeaParseError::InvalidFormat);
        }
        // Split the sentence into the main part and the checksum
        let mut parts = sentence[1..].split('*');

        // We can always expect to have a body part
        let body_part = match parts.next() {
            Some(b) => b,
            None => return Err(NmeaParseError::InvalidFormat),
        };

        let checksum_part = match parts.next() {
            Some(c) => Some(c),
            None if self.require_checksum => return Err(NmeaParseError::InvalidFormat),
            None => None, // No checksum provided, but it's not required
        };

        // Compute the actual checksum (always possible)
        let computed_checksum = Self::checksum(body_part);

        // If a checksum is provided, validate it
        if let Some(checksum_str) = checksum_part {
            let provided_checksum = u8::from_str_radix(checksum_str, 16)
                .map_err(|_| NmeaParseError::InvalidChecksum)?;
            if computed_checksum != provided_checksum {
                return Err(NmeaParseError::ChecksumMismatch(
                    computed_checksum,
                    provided_checksum,
                ));
            }
        }

        // Split the main part into fields
        let mut fields = body_part.split(',');
        let message_type = match fields.next() {
            Some(t) => t,
            None => return Err(NmeaParseError::InvalidFormat),
        };

        // For now, we just return the raw sentence for unrecognised message types
        Ok(NmeaMessageBorrowed {
            sentence_type: message_type,
            fields: fields,
            checksum: computed_checksum,
        })
    }

    pub(crate) fn encode_internal(
        &self,
        message: &impl NmeaMessageEncodable,
        mut buff: impl core::fmt::Write,
    ) -> Result<(), NmeaParseError> {
        // Write the start character
        buff.write_char('$')
            .map_err(|_| NmeaParseError::BufferWrite)?;

        // Setup the checksum buffer
        let mut checksum_writer = ChecksumWriter { buff, checksum: 0 };

        // Write the message type and body
        message.encode_type(&mut checksum_writer)?;
        checksum_writer
            .write_char(',')
            .map_err(|_| NmeaParseError::BufferWrite)?;
        message.encode_body(&mut checksum_writer)?;

        // Extract the checksum and buffer
        let ChecksumWriter { mut buff, checksum } = checksum_writer;

        // Write the checksum
        if self.include_checksum {
            buff.write_char('*')
                .map_err(|_| NmeaParseError::BufferWrite)?;
            write!(buff, "{:02X}", checksum).map_err(|_| NmeaParseError::BufferWrite)?;
        }

        // Write the end of line
        if self.include_newline {
            buff.write_str("\r\n")
                .map_err(|_| NmeaParseError::BufferWrite)?;
        }

        Ok(())
    }

    /// Compute the checksum for a raw NMEA sentence (aka, the part between '$' and '*')
    pub fn checksum(raw_sentence: &str) -> u8 {
        let checksum = raw_sentence.bytes().fold(0u8, |acc, b| acc ^ b);
        checksum
    }
}

/// Helper to compute checksums while writing to the underlying buffer
struct ChecksumWriter<B: core::fmt::Write> {
    buff: B,
    checksum: u8,
}

impl<B: core::fmt::Write> core::fmt::Write for ChecksumWriter<B> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.checksum = s.bytes().fold(self.checksum, |acc, b| acc ^ b);
        self.buff.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum() {
        let sentence = "$PQTMCFGRCVRMODE,W,2*29";
        let checksum = NmeaParser::checksum(&sentence[1..sentence.find('*').unwrap()]);
        assert_eq!(checksum, 0x29);
    }

    #[test]
    fn test_base_nmea_message_parse_encode() {
        let mut tests = vec![
            (
                "$PQTMCFGRCVRMODE,W,2*29",
                "PQTMCFGRCVRMODE",
                vec!["W", "2"],
                0x29,
            ),
            (
                "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47",
                "GPGGA",
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
                    "",
                ],
                0x47,
            ),
            (
                "$GPRMC,235947,A,3723.2475,N,12202.3416,W,0.13,309.62,120598,,,A*65",
                "GPRMC",
                vec![
                    "235947",
                    "A",
                    "3723.2475",
                    "N",
                    "12202.3416",
                    "W",
                    "0.13",
                    "309.62",
                    "120598",
                    "",
                    "",
                    "A",
                ],
                0x65,
            ),
        ];

        for (raw, expected_type, expected_fields, expected_checksum) in tests.drain(..) {
            let parsed = NMEA_PARSER_STRICT
                .parse_internal(raw)
                .expect("Failed to parse NMEA message");
            assert_eq!(parsed.sentence_type, expected_type);
            assert_eq!(parsed.fields.clone().collect::<Vec<_>>(), expected_fields);
            assert_eq!(parsed.checksum, expected_checksum);

            let mut encoded = String::new();
            NMEA_PARSER_STRICT
                .encode_internal(&parsed, &mut encoded)
                .expect("Failed to encode NMEA message");
            assert_eq!(encoded, raw);
        }
    }
}
