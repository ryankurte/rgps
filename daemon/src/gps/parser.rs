use nmea::{Nmea, SentenceType};
use rtcm_rs::MessageFrame;
use tracing::{debug, trace};

use super::GpsMessage;

const NMEA_START: [u8; 1] = [b'$'];
const RTCM3_START: [u8; 1] = [0xD3];
const UBX_START: [u8; 2] = [0xB5, 0x62];

// TODO: clean up the accumulator / packet detection / etc.

/// Accumulates and parses GPS messages from a stream of bytes.
///
pub struct GpsAccumulator {
    buffer: Vec<u8>,

    nmea_parser: Nmea,
    // TODO: parse UBX, Quectel messages?
}

impl GpsAccumulator {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
            nmea_parser: Nmea::default(),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn next_message(&mut self) -> Option<GpsMessage> {
        // We expect the first byte in the buffer to indicate the message type (NMEA, UBX, RTCM3).
        // If the buffer isn't yet long enough for the detected message type, we return None and wait for more data to accumulate.
        // If the first byte doesn't match any known message type, we discard it and continue looking for the next message.

        while self.buffer.len() > 0 {
            match self.buffer.get(0) {
                Some(b) if *b == NMEA_START[0] => {
                    trace!("Found NMEA start byte");

                    // See if we can find the end of the NMEA sentence (newline)
                    let pos = match self.buffer.iter().position(|&b| b == b'\n') {
                        Some(p) => p,
                        // Continue accumulating until we get a full sentence
                        None => {
                            trace!("No newline found in buffer yet, waiting for more data");
                            return None;
                        }
                    };

                    // Try to parse the NMEA sentence
                    let message = match Self::parse_nmea(&mut self.nmea_parser, &self.buffer[..pos])
                    {
                        Some(msg) => Some(msg),
                        None => {
                            debug!("Failed to parse NMEA sentence");
                            None
                        }
                    };

                    // Remove the parsed sentence from the buffer
                    self.buffer.drain(..pos + 1);

                    // Return the message if we successfully parsed it
                    return message.map(|msg| GpsMessage::Nmea(msg.0, msg.1));
                }
                Some(b) if *b == RTCM3_START[0] => {
                    trace!("Found RTCM3 start byte");

                    // Check the buffer is long enough for the RTCM3 header (3 bytes)
                    if self.buffer.len() < 3 {
                        trace!("Buffer too short for RTCM3 header, waiting for more data");
                        return None; // Wait for more data
                    }

                    trace!("Buffer: {:02x?}", self.buffer);

                    // Read the RTCM3 length byte
                    let len = ((self.buffer[1] & 0x03) as usize) << 8 | (self.buffer[2] as usize);

                    // Check the buffer is long enough for the full RTCM3 message (header + length + payload + checksum)
                    if self.buffer.len() < 3 + len + 3 {
                        trace!(
                            "Buffer ({} bytes) too short for full ({} bytes) RTCM3 message, waiting for more data",
                            self.buffer.len(),
                            3 + len + 3
                        );
                        return None; // Wait for more data
                    }

                    // Attempt to parse the RTCM3 message
                    let frame = match MessageFrame::new(&self.buffer[..]) {
                        Ok(frame) => frame,
                        Err(e) => {
                            debug!("Failed to parse RTCM3 message: {}", e);
                            self.drain_to_next();
                            continue;
                        }
                    };

                    let message = frame.get_message();
                    let message_raw = frame.frame_data().to_vec();
                    trace!("Parsed RTCM3 message: {:?}", message);

                    // Remove the parsed sentence from the buffer
                    self.buffer.drain(..frame.data_len());

                    return Some(GpsMessage::Rtcm3(message, message_raw));
                }
                Some(b) if *b == UBX_START[0] => {
                    trace!("Found UBX start byte");

                    // Check the buffer is long enough for the UBX header
                    if self.buffer.len() < 6 {
                        trace!("Buffer too short for UBX header, waiting for more data");
                        return None; // Wait for more data
                    }

                    // Check the second UBX header byte
                    if self.buffer.get(1) != Some(&UBX_START[1]) {
                        // Invalid UBX header, discard the first byte and continue looking for the next message
                        trace!("Invalid UBX header, discarding first byte");
                        self.drain_to_next();
                        continue;
                    }

                    // Read the length byte
                    let len = match self.buffer.get(4) {
                        Some(b) => *b as usize,
                        None => {
                            trace!(
                                "Buffer too short to read UBX length byte, waiting for more data"
                            );
                            return None; // Wait for more data
                        }
                    };
                    // Check if we have the full UBX message (header + length + payload + checksum)
                    if self.buffer.len() < 6 + len {
                        trace!("Buffer too short for full UBX message, waiting for more data");
                        return None; // Wait for more data
                    }
                    // TODO: try to parse UBX message

                    // Copy the raw form too (useful for sending to other clients)
                    let message_raw = self.buffer[..6 + len].to_vec();

                    // Remove the parsed message from the buffer
                    self.buffer.drain(..6 + len);

                    return Some(GpsMessage::Ubx(message_raw));
                }
                Some(b) => {
                    trace!(
                        "Unknown GPS message type: 0x{:02X}, searching for new start byte",
                        b
                    );
                    self.drain_to_next();
                }
                _ => return None, // Wait for more data
            }
        }

        None
    }

    fn drain_to_next(&mut self) -> bool {
        // Remove the first (correct) byte
        self.buffer.drain(..1);

        // Look for the next potential start byte (NMEA, RTCM3, UBX)
        let pos = self
            .buffer
            .iter()
            .position(|&b| b == NMEA_START[0] || b == RTCM3_START[0] || b == UBX_START[0]);
        match pos {
            Some(p) => {
                trace!(
                    "Found potential start byte at position {}, discarding {} bytes",
                    p, p
                );
                self.buffer.drain(..p);
                true
            }
            None => {
                trace!(
                    "No start byte found, discarding entire buffer of {} bytes",
                    self.buffer.len()
                );
                self.buffer.clear();
                false
            }
        }
    }

    fn parse_nmea(nmea: &mut Nmea, buff: &[u8]) -> Option<(Nmea, SentenceType)> {
        // NMEA sentences are ASCII, so we can try to parse the buffer as UTF-8.
        let s = match str::from_utf8(&buff) {
            Ok(s) => s,
            Err(e) => {
                debug!("Failed to parse GPS data as UTF-8: {}", e);
                return None;
            }
        };

        match nmea.parse(s) {
            Ok(sentence) => {
                trace!("Parsed NMEA sentence: {:?}", sentence);
                trace!("Current state: {:?}", nmea);

                Some((nmea.clone(), sentence))
            }
            Err(e) => {
                debug!("Failed to parse NMEA sentence: {}", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use tracing::level_filters::LevelFilter;

    use crate::setup_logging;

    use super::*;

    #[test]
    fn test_nmea_parsing() {
        setup_logging(LevelFilter::TRACE);

        let mut a = GpsAccumulator::new();

        let sentence = "$GPGGA,123519,4807.038,N,01131.000,E,1,08,0.9,545.4,M,46.9,M,,*47";
        a.push(&sentence.as_bytes());

        match a.next_message() {
            Some(s) => {
                println!("Parsed sentence: {:?}", s);
            }
            None => {
                panic!("Failed to parse NMEA sentence");
            }
        }
    }

    #[test]
    fn test_rtcm3_parsing() {
        setup_logging(LevelFilter::TRACE);

        let message = &[
            0xd3, 0x02, 0x16, 0x46, 0x70, 0x00, 0x80, 0x96, 0x46, 0x20, 0x00, 0x20, 0x00, 0x00,
            0x01, 0xc1, 0xe2, 0x40, 0x00, 0x80, 0x20, 0x82, 0x01, 0x41, 0x6f, 0xbe, 0xfb, 0xed,
            0xbe, 0xfb, 0xef, 0x82, 0x8a, 0x9a, 0x52, 0xaa, 0xaa, 0x82, 0x6a, 0x8a, 0x63, 0xf0,
            0x00, 0x00, 0x00, 0x00, 0x02, 0xad, 0x53, 0xda, 0x1d, 0x1f, 0x4d, 0x3c, 0x43, 0xeb,
            0xff, 0xbb, 0x29, 0xfe, 0x3a, 0x09, 0x00, 0x02, 0x20, 0x8e, 0x7d, 0x1e, 0x11, 0x7f,
            0xec, 0x7e, 0xf7, 0x81, 0x41, 0xf9, 0x91, 0x59, 0x25, 0x94, 0xaf, 0x01, 0x56, 0x30,
            0x95, 0x02, 0xd1, 0x5c, 0xee, 0x05, 0x90, 0x78, 0x5d, 0x33, 0x05, 0xf6, 0xd0, 0x58,
            0xf5, 0x85, 0xe6, 0x08, 0x3a, 0x4e, 0x03, 0xb3, 0x88, 0x3d, 0xef, 0x03, 0x8a, 0xf8,
            0x3b, 0xd6, 0x6e, 0x7f, 0xce, 0xf7, 0xe2, 0xf0, 0x28, 0xb6, 0xfb, 0x03, 0x6e, 0xc8,
            0x11, 0xc3, 0x72, 0x9d, 0xca, 0x79, 0xe9, 0xf4, 0x1d, 0xf2, 0xbf, 0x04, 0xd8, 0xf3,
            0xf5, 0xdf, 0x70, 0x0b, 0x76, 0x9d, 0xbf, 0x0a, 0x81, 0x87, 0x10, 0xa0, 0x7c, 0x43,
            0x88, 0x16, 0xa0, 0x7b, 0x8a, 0x87, 0x4e, 0xf1, 0x19, 0xdf, 0x12, 0x6e, 0x51, 0x31,
            0x75, 0x12, 0xb0, 0xd1, 0x1c, 0x8c, 0x14, 0x0c, 0x51, 0x5f, 0x3b, 0x17, 0x14, 0x91,
            0x6d, 0x4e, 0x14, 0x7e, 0x2f, 0xa7, 0xcc, 0x7b, 0x8d, 0x68, 0x5b, 0xd8, 0x10, 0x5f,
            0xaf, 0xd8, 0x61, 0x1e, 0x48, 0x5e, 0xfa, 0x50, 0x5a, 0x5f, 0xb0, 0x1a, 0x00, 0x40,
            0x1b, 0x97, 0xc8, 0x1e, 0x1c, 0x48, 0x1c, 0x1e, 0xa8, 0x1b, 0x8a, 0x58, 0x1c, 0x54,
            0x88, 0x2b, 0x1f, 0x10, 0x30, 0xb5, 0xf8, 0x2e, 0x06, 0xc8, 0x1c, 0x9c, 0xef, 0xbb,
            0xde, 0x7f, 0xc5, 0x2b, 0xcf, 0xc8, 0xfd, 0xbf, 0xc3, 0x3f, 0x17, 0xbc, 0x1a, 0xa0,
            0x71, 0x97, 0x20, 0x7b, 0xcb, 0xf0, 0x7d, 0xa4, 0x28, 0x7b, 0x41, 0xef, 0xc3, 0x93,
            0x37, 0xd0, 0x25, 0x5f, 0xda, 0xd3, 0xef, 0xdb, 0x67, 0x87, 0xc2, 0x22, 0xe8, 0x2f,
            0x9a, 0x78, 0x3d, 0xd2, 0x98, 0x43, 0x0a, 0xf8, 0x41, 0x31, 0x48, 0x2e, 0x02, 0x60,
            0x58, 0x79, 0x38, 0x63, 0x39, 0x38, 0x6b, 0x31, 0x08, 0x65, 0xf6, 0x08, 0x57, 0x64,
            0xe8, 0x59, 0x66, 0x38, 0x64, 0xf8, 0x10, 0x6b, 0x4f, 0x10, 0x68, 0x39, 0xd0, 0x57,
            0xbb, 0x5f, 0xf2, 0x91, 0x6f, 0xf9, 0x58, 0x4c, 0x95, 0x30, 0x4a, 0xb2, 0xa4, 0x8b,
            0x47, 0x51, 0xd4, 0x75, 0x1d, 0x47, 0x4f, 0x93, 0xfc, 0xff, 0x3f, 0xcf, 0x92, 0xb4,
            0xc7, 0x31, 0xcb, 0xf2, 0x43, 0x87, 0x0c, 0x43, 0x90, 0xa5, 0x03, 0x42, 0x50, 0x94,
            0x14, 0xff, 0x32, 0xcc, 0xb3, 0x2c, 0xcb, 0x31, 0xc7, 0x91, 0xe4, 0x79, 0x1e, 0x47,
            0x73, 0x4c, 0xd1, 0x34, 0xcd, 0x13, 0x2c, 0xc3, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x60, 0xde, 0x37, 0x25, 0xdf, 0x47, 0x50, 0xf5, 0xcd, 0x3d, 0x54, 0xca, 0xf9,
            0x96, 0x69, 0x8b, 0xe4, 0xd7, 0x94, 0x3b, 0x47, 0x4f, 0xd4, 0x4b, 0xeb, 0x12, 0x4e,
            0x53, 0xfc, 0xf7, 0x6f, 0xdf, 0xf7, 0x6d, 0xf7, 0x59, 0xde, 0x17, 0x85, 0xd5, 0x7c,
            0xd7, 0x56, 0x9d, 0x85, 0x53, 0xd7, 0x34, 0xce, 0x47, 0x9c, 0xe4, 0xf9, 0x6d, 0xf1,
            0x53, 0x55, 0x20, 0xc6, 0x41, 0x28, 0x82, 0x5f, 0x05, 0x18, 0x06, 0x54, 0x8f, 0xf1,
            0x05, 0xc2, 0x10, 0xa4, 0x0c, 0x88, 0x59, 0x91, 0xa9, 0x22, 0x96, 0x44, 0xf4, 0x8a,
            0x51, 0x17, 0xf0, 0xca, 0xa1, 0x9a, 0xc3, 0x47, 0x07, 0xb2, 0x10, 0xda, 0x0c, 0x4c,
            0x17, 0x38, 0x48, 0xc0, 0x7c, 0x41, 0x08, 0x02, 0x16, 0x84, 0x4c, 0x07, 0x3a, 0x12,
            0xbc, 0x42, 0x00, 0x83, 0x41, 0x09, 0xa2, 0x1b, 0x04, 0x2e, 0x92, 0x9e, 0x24, 0xaa,
            0x4e, 0xb0, 0x97, 0x11, 0x30, 0x8e, 0x69, 0xdc, 0xd2, 0xb9, 0xc0, 0xf3, 0x2d, 0xe5,
            0x98, 0x4e, 0x30, 0x96, 0xa8, 0xcf, 0x10, 0x7d,
        ];

        let mut a = GpsAccumulator::new();

        a.push(message);

        match a.next_message() {
            Some(s) => {
                println!("Parsed message: {:?}", s);
            }
            None => {
                panic!("Failed to parse RTCM3 message");
            }
        }
    }
}
