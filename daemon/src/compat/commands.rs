use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use strum::{Display, EnumString};

/// Arguments for the ?WATCH command.
///
/// See: https://gpsd.gitlab.io/gpsd/gpsd_json.html#_watch
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct WatchArgs {
    /// Enable (true) or disable (false) watcher mode. Default is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Enable or disable dumping of JSON reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<bool>,

    /// Enable or disable dumping of binary packets as pseudo-NMEA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nmea: Option<bool>,

    /// Controls raw mode. 1 = hex-dump unprocessed stream, 2 = verbatim binary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<u8>,

    /// If true, apply scaling divisors to output before dumping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaled: Option<bool>,

    /// If true, aggregate AIS type24 sentence parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split24: Option<bool>,

    /// If true, emit TOFF and PPS JSON messages each cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pps: Option<bool>,

    /// If present, watch only the specified device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

/// Arguments for the ?DEVICE command.
///
/// See: https://gpsd.gitlab.io/gpsd/gpsd_json.html#_device
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DeviceArgs {
    /// Device path (e.g. "/dev/ttyUSB0").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// 0 = NMEA mode, 1 = alternate/binary mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<u8>,

    /// Device speed in bits per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bps: Option<u32>,

    /// Parity: "N", "O", or "E".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parity: Option<String>,

    /// Stop bits (1 or 2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopbits: Option<u8>,

    /// Device cycle time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<f64>,

    /// Raw hex data to write to the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hexdata: Option<String>,
}

/// Commands received from GPSD protocol clients.
///
/// The wire format uses `?COMMAND;` or `?COMMAND=<json>;`.
///
/// See: https://gpsd.gitlab.io/gpsd/gpsd_json.html#_core_protocol_commands
#[derive(Clone, Debug, PartialEq)]
pub enum GpsdCommand {
    /// `?VERSION;` — request daemon version and protocol level.
    Version,

    /// `?DEVICES;` — request the list of devices known to the daemon.
    Devices,

    /// `?WATCH;` or `?WATCH=<json>;` — get or set per-subscriber watcher policy.
    /// When `None`, the current policy is returned without modification.
    Watch(Option<WatchArgs>),

    /// `?POLL;` — request the latest cached fix data from all active devices.
    Poll,

    /// `?DEVICE;` or `?DEVICE=<json>;` — get or set device-specific parameters.
    /// When `None`, the state of all devices is returned.
    Device(Option<DeviceArgs>),
}

impl std::fmt::Display for GpsdCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpsdCommand::Version => write!(f, "?VERSION;"),
            GpsdCommand::Devices => write!(f, "?DEVICES;"),
            GpsdCommand::Poll => write!(f, "?POLL;"),
            GpsdCommand::Watch(None) => write!(f, "?WATCH;"),
            GpsdCommand::Watch(Some(args)) => {
                let json = serde_json::to_string(args).map_err(|_| std::fmt::Error)?;
                write!(f, "?WATCH={json};")
            }
            GpsdCommand::Device(None) => write!(f, "?DEVICE;"),
            GpsdCommand::Device(Some(args)) => {
                let json = serde_json::to_string(args).map_err(|_| std::fmt::Error)?;
                write!(f, "?DEVICE={json};")
            }
        }
    }
}

/// Error returned when parsing a GPSD command string fails.
#[derive(Clone, Debug, PartialEq)]
pub enum ParseCommandError {
    /// The input did not start with '?' or end with ';'.
    InvalidFormat,
    /// The command name is not recognised.
    UnknownCommand(String),
    /// The JSON argument could not be deserialised.
    InvalidArgs(String),
}

impl std::fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseCommandError::InvalidFormat => write!(f, "invalid command format"),
            ParseCommandError::UnknownCommand(cmd) => write!(f, "unknown command: {cmd}"),
            ParseCommandError::InvalidArgs(msg) => write!(f, "invalid command args: {msg}"),
        }
    }
}

impl std::error::Error for ParseCommandError {}

impl GpsdCommand {
    /// Parse a command, returning the parsed command and the number of bytes
    /// consumed from the input string.
    pub fn parse(s: &str) -> Result<(Self, usize), ParseCommandError> {
        // Look for the start of the command (a '?') and trim any leading whitespace.
        let s = s.trim();
        if !s.starts_with('?') {
            return Err(ParseCommandError::InvalidFormat);
        }

        // The command name ends with either `;` or `=`, so look for the first occurrence of either.
        let name_end_pos = s.find([';', '=']).ok_or(ParseCommandError::InvalidFormat)?;

        // Grab the command name and convert to uppercase
        let name = &s[1..name_end_pos]; // Skip the leading '?'
        let name = name.trim().to_ascii_uppercase();

        if s[name_end_pos..].starts_with(';') {
            // No JSON args, just a simple command
            let cmd = match name.as_str() {
                "VERSION" => GpsdCommand::Version,
                "DEVICES" => GpsdCommand::Devices,
                "POLL" => GpsdCommand::Poll,
                "WATCH" => GpsdCommand::Watch(None),
                "DEVICE" => GpsdCommand::Device(None),
                other => return Err(ParseCommandError::UnknownCommand(other.to_string())),
            };
            Ok((cmd, name_end_pos + 1))
        } else {
            // JSON args, use a streaming parser to find the end of the JSON object.
            match name.as_str() {
                "WATCH" => {
                    let decoder = &mut Deserializer::from_str(&s[name_end_pos + 1..])
                        .into_iter::<WatchArgs>();
                    let args = decoder
                        .next()
                        .transpose()
                        .map_err(|e| ParseCommandError::InvalidArgs(e.to_string()))?
                        .ok_or_else(|| {
                            ParseCommandError::InvalidArgs("missing JSON value".to_string())
                        })?;
                    Ok((
                        GpsdCommand::Watch(Some(args)),
                        decoder.byte_offset() + name_end_pos + 1,
                    ))
                }
                "DEVICE" => {
                    let decoder = &mut Deserializer::from_str(&s[name_end_pos + 1..])
                        .into_iter::<DeviceArgs>();
                    let args = decoder
                        .next()
                        .transpose()
                        .map_err(|e| ParseCommandError::InvalidArgs(e.to_string()))?
                        .ok_or_else(|| {
                            ParseCommandError::InvalidArgs("missing JSON value".to_string())
                        })?;
                    Ok((
                        GpsdCommand::Device(Some(args)),
                        decoder.byte_offset() + name_end_pos + 1,
                    ))
                }
                other => Err(ParseCommandError::UnknownCommand(other.to_string())),
            }
        }
    }
}

impl FromStr for GpsdCommand {
    type Err = ParseCommandError;

    /// Parse a GPSD wire-format command string such as `?WATCH;` or `?WATCH={"json":true};`.
    ///
    /// A request line may contain multiple `?CMD;` tokens; this parses exactly one.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).map(|(cmd, _)| cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commands() {
        let test: &[(&str, Result<GpsdCommand, ParseCommandError>)] = &[
            ("?VERSION;", Ok(GpsdCommand::Version)),
            ("?DEVICES;", Ok(GpsdCommand::Devices)),
            ("?POLL;", Ok(GpsdCommand::Poll)),
            ("?WATCH;", Ok(GpsdCommand::Watch(None))),
            (
                "?WATCH={\"json\":true,\"pps\":true};",
                Ok(GpsdCommand::Watch(Some(WatchArgs {
                    json: Some(true),
                    pps: Some(true),
                    ..Default::default()
                }))),
            ),
            (
                "?WATCH={\"enable\":false,\"nmea\":true,\"raw\":1,\"scaled\":true,\"split24\":false,\"device\":\"/dev/ttyUSB0\"};",
                Ok(GpsdCommand::Watch(Some(WatchArgs {
                    enable: Some(false),
                    nmea: Some(true),
                    raw: Some(1),
                    scaled: Some(true),
                    split24: Some(false),
                    device: Some("/dev/ttyUSB0".to_string()),
                    ..Default::default()
                }))),
            ),
            ("?DEVICE;", Ok(GpsdCommand::Device(None))),
            (
                "?DEVICE={\"path\":\"/dev/ttyUSB0\",\"bps\":115200,\"parity\":\"N\",\"stopbits\":1};",
                Ok(GpsdCommand::Device(Some(DeviceArgs {
                    path: Some("/dev/ttyUSB0".to_string()),
                    bps: Some(115200),
                    parity: Some("N".to_string()),
                    stopbits: Some(1),
                    ..Default::default()
                }))),
            ),
            (
                "?DEVICE={\"path\":\"/dev/ttyUSB1\",\"native\":1,\"hexdata\":\"b5620a04\"};",
                Ok(GpsdCommand::Device(Some(DeviceArgs {
                    path: Some("/dev/ttyUSB1".to_string()),
                    native: Some(1),
                    hexdata: Some("b5620a04".to_string()),
                    ..Default::default()
                }))),
            ),
            // Invalid commands
            ("VERSION;", Err(ParseCommandError::InvalidFormat)),
            ("?VERSION", Err(ParseCommandError::InvalidFormat)),
            (
                "?FOO;",
                Err(ParseCommandError::UnknownCommand("FOO".to_string())),
            ),
            (
                "?WATCH={bad json};",
                Err(ParseCommandError::InvalidArgs(
                    // Only check the variant, not the exact serde message — see assert below
                    "".to_string(),
                )),
            ),
        ];

        for (input, expected) in test {
            let parsed = GpsdCommand::from_str(input);
            match (parsed, expected) {
                (
                    Err(ParseCommandError::InvalidArgs(_)),
                    Err(ParseCommandError::InvalidArgs(_)),
                ) => {}
                (parsed, expected) => {
                    assert_eq!(&parsed, expected, "parsing command: {input}");
                }
            }
        }
    }

    #[test]
    fn test_roundtrip() {
        let commands = [
            GpsdCommand::Version,
            GpsdCommand::Devices,
            GpsdCommand::Poll,
            GpsdCommand::Watch(None),
            GpsdCommand::Watch(Some(WatchArgs {
                json: Some(true),
                pps: Some(true),
                ..Default::default()
            })),
            GpsdCommand::Device(None),
            GpsdCommand::Device(Some(DeviceArgs {
                path: Some("/dev/ttyUSB0".to_string()),
                bps: Some(115200),
                ..Default::default()
            })),
        ];

        for cmd in &commands {
            let s = cmd.to_string();
            let parsed = GpsdCommand::from_str(&s)
                .unwrap_or_else(|e| panic!("roundtrip failed for {s:?}: {e}"));
            assert_eq!(&parsed, cmd, "roundtrip mismatch for {s:?}");
        }
    }
}
