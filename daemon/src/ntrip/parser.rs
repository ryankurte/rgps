use robust_ntrip_client::RobustNtripClient;
use rtcm_rs::Message;

pub struct FrameData {
    pub frame_data: bytes::BytesMut,
    pub message_number: u16,
}

/// A client which parses RTCM messages from the NTRIP stream.
pub struct ParsingNtripClient {
    client: RobustNtripClient,
    buf: bytes::BytesMut,
}

impl ParsingNtripClient {
    /// Create a parsing NTRIP client by wrapping a low-level NTRIP client.
    pub fn new(client: RobustNtripClient) -> Self {
        let buf = bytes::BytesMut::new();
        Self { client, buf }
    }

    /// Get the next RTCM message from the NTRIP server.
    pub async fn next(&mut self) -> Result<FrameData, anyhow::Error> {
        loop {
            let mut advance_info = None;
            for (i, start_byte) in (&self.buf).into_iter().enumerate() {
                if *start_byte == 0xd3 {
                    match rtcm_rs::MessageFrame::new(&self.buf[i..]) {
                        Ok(m) => {
                            tracing::debug!(
                                "Found RTCM message {} frame with length {}",
                                m.message_number().unwrap(),
                                m.frame_len()
                            );
                            advance_info = Some((
                                i,
                                false,
                                Some((m.frame_len(), m.message_number().unwrap())),
                            ));
                            break;
                        }
                        Err(rtcm_rs::rtcm_error::RtcmError::Incomplete) => {
                            advance_info = Some((i, true, None)); // discard data prior to the start byte.
                            break;
                        }
                        Err(rtcm_rs::rtcm_error::RtcmError::NotValid) => {
                            advance_info = Some((i + 1, false, None)); // advance past the invalid "start byte".
                            break;
                        }
                        _ => unreachable!(),
                    }
                }
            }

            let (n_discard, do_read_more, msg_info) = if let Some(x) = advance_info {
                x
            } else {
                // no start byte found, so we need to read more data.
                (self.buf.len(), true, None)
            };

            let _discard_bytes = self.buf.split_to(n_discard);
            if let Some((frame_len, message_number)) = msg_info {
                assert!(!do_read_more);
                let frame_data = self.buf.split_to(frame_len);
                return Ok(FrameData {
                    frame_data,
                    message_number,
                });
            }

            if do_read_more {
                // Fetch more data.
                let this_buf = self
                    .client
                    .chunk()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read from NTRIP client: {}", e))?;
                self.buf.extend_from_slice(&this_buf);
            }
        }
    }
}
