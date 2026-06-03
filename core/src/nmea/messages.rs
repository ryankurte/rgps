//! Specific NMEA messages
//!

#![allow(unused, dead_code)]

use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::nmea::{NmeaMessageBorrowed, NmeaMessageEncodable, NmeaParseError};

// TODO: maybe write a macro to help with the boilerplate of parsing/encoding these?

/// ALM - GPS Almanac Data
///
/// This sentence expresses orbital data for a specified GPS satellite.
///
/// Reference: $--ALM,x.x,x.x,xx,x.x,hh,hhhh,hh,hhhh,hhhh,hhhhhh,hhhhhh,hhhhhh,hhhhhh,hhh,hhh,*hh<CR><LF>
/// Example:   $GPALM,1,1,15,1159,00,441d,4e,16be,fd5e,a10c9f,4a2da4,686e81,58cbe1,0a4,001*77
///
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Alm {
    /// Field 1: Total number of messages
    pub total_number_of_messages: u8,
    /// Field 2: Message Number
    pub message_number: u8,
    /// Field 3: Satellite PRN number (01 to 32)
    pub satellite_prn_number: u8,
    /// Field 4: GPS Week Number :
    pub gps_week_number: u16,
    /// Field 5: SV health, bits 17-24 of each almanac page
    pub sv_health_bits_17_24_of: u8,
    /// Field 6: Eccentricity
    pub eccentricity: u16,
    /// Field 7: Almanac Reference Time
    pub almanac_reference_time: u8,
    /// Field 8: Inclination Angle
    pub inclination_angle: u16,
    /// Field 9: Rate of Right Ascension
    pub rate_of_right_ascension: u16,
    /// Field 10: Root of semi-major axis
    pub root_of_semi_major_axis: u32,
    /// Field 11: Argument of perigee
    pub argument_of_perigee: u32,
    /// Field 12: Longitude of ascension node
    pub longitude_of_ascension_node: u32,
    /// Field 13: Mean anomaly
    pub mean_anomaly: u32,
    /// Field 14: F0 Clock Parameter
    pub f0_clock_parameter: i16,
    /// Field 15: F1 Clock Parameter
    pub f1_clock_parameter: i16,
}

impl NmeaMessageEncodable for Alm {
    fn encode_type(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        buffer
            .write_str("GPALM")
            .map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }

    fn encode_body(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        write!(buffer, "{},{},{},{},{:02X},{:04X},{:02X},{:04X},{:04X},{:04X},{:06X},{:06X},{:06X},{:06X},{:04X}",
            self.total_number_of_messages,
            self.message_number,
            self.satellite_prn_number,
            self.gps_week_number,
            self.sv_health_bits_17_24_of,
            self.eccentricity,
            self.almanac_reference_time,
            self.inclination_angle,
            self.rate_of_right_ascension as u16,
            self.root_of_semi_major_axis,
            self.argument_of_perigee,
            self.longitude_of_ascension_node,
            self.mean_anomaly,
            self.f0_clock_parameter as u16,
            self.f1_clock_parameter as u16,
        ).map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }
}

impl<'a> TryFrom<NmeaMessageBorrowed<'a>> for Alm {
    type Error = NmeaParseError;

    fn try_from(value: NmeaMessageBorrowed<'a>) -> Result<Self, Self::Error> {
        let NmeaMessageBorrowed {
            sentence_type,
            mut fields,
            checksum: _,
        } = value;

        // Check the sentence type matches
        if &sentence_type[2..] != "ALM" {
            return Err(NmeaParseError::InvalidMessageType);
        }

        Ok(Self {
            total_number_of_messages: fields
                .next()
                .ok_or(NmeaParseError::MissingField(1))
                .and_then(|s| u8::from_str_radix(s, 10).map_err(|e| e.into()))?,
            message_number: fields
                .next()
                .ok_or(NmeaParseError::MissingField(2))
                .and_then(|s| u8::from_str_radix(s, 10).map_err(|e| e.into()))?,
            satellite_prn_number: fields
                .next()
                .ok_or(NmeaParseError::MissingField(3))
                .and_then(|s| u8::from_str_radix(s, 10).map_err(|e| e.into()))?,
            gps_week_number: fields
                .next()
                .ok_or(NmeaParseError::MissingField(4))
                .and_then(|s| u16::from_str_radix(s, 10).map_err(|e| e.into()))?,
            sv_health_bits_17_24_of: fields
                .next()
                .ok_or(NmeaParseError::MissingField(5))
                .and_then(|s| u8::from_str_radix(s, 16).map_err(|e| e.into()))?,
            eccentricity: fields
                .next()
                .ok_or(NmeaParseError::MissingField(6))
                .and_then(|s| u16::from_str_radix(s, 16).map_err(|e| e.into()))?,
            almanac_reference_time: fields
                .next()
                .ok_or(NmeaParseError::MissingField(7))
                .and_then(|s| u8::from_str_radix(s, 16).map_err(|e| e.into()))?,
            inclination_angle: fields
                .next()
                .ok_or(NmeaParseError::MissingField(8))
                .and_then(|s| u16::from_str_radix(s, 16).map_err(|e| e.into()))?,
            rate_of_right_ascension: fields
                .next()
                .ok_or(NmeaParseError::MissingField(9))
                .and_then(|s| u16::from_str_radix(s, 16).map_err(|e| e.into()))?,
            root_of_semi_major_axis: fields
                .next()
                .ok_or(NmeaParseError::MissingField(10))
                .and_then(|s| u32::from_str_radix(s, 16).map_err(|e| e.into()))?,
            argument_of_perigee: fields
                .next()
                .ok_or(NmeaParseError::MissingField(11))
                .and_then(|s| u32::from_str_radix(s, 16).map_err(|e| e.into()))?,
            longitude_of_ascension_node: fields
                .next()
                .ok_or(NmeaParseError::MissingField(12))
                .and_then(|s| u32::from_str_radix(s, 16).map_err(|e| e.into()))?,
            mean_anomaly: fields
                .next()
                .ok_or(NmeaParseError::MissingField(13))
                .and_then(|s| u32::from_str_radix(s, 16).map_err(|e| e.into()))?,
            f0_clock_parameter: fields
                .next()
                .ok_or(NmeaParseError::MissingField(14))
                .and_then(|s| i16::from_str_radix(s, 16).map_err(|e| e.into()))?,
            f1_clock_parameter: fields
                .next()
                .ok_or(NmeaParseError::MissingField(15))
                .and_then(|s| i16::from_str_radix(s, 16).map_err(|e| e.into()))?,
        })
    }
}

#[derive(Copy, Clone, PartialEq, Debug, EnumString, Display, Serialize, Deserialize)]
pub enum NorthOrSouth {
    #[strum(serialize = "N")]
    North,
    #[strum(serialize = "S")]
    South,
}

#[derive(Copy, Clone, PartialEq, Debug, EnumString, Display, Serialize, Deserialize)]
pub enum EastOrWest {
    #[strum(serialize = "E")]
    East,
    #[strum(serialize = "W")]
    West,
}

#[derive(Copy, Clone, PartialEq, Debug, EnumString, Display, Serialize, Deserialize)]
pub enum Units {
    #[strum(serialize = "M")]
    Meters,
}

/// GGA - Global Positioning System Fix Data
///
/// Reference: $--GGA,hhmmss.ss,llll.ll,a,yyyyy.yy,a,x,xx,x.x,x.x,M,x.x,M,x.x,xxxx*hh<CR><LF>
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Gga {
    /// Field 1: Universal Time Coordinated (UTC)
    pub universal_time_coordinated: NaiveTime,
    /// Field 2: Latitude
    pub latitude: f32,
    /// Field 3: N or S (North or South)
    pub n_or_s: NorthOrSouth,
    /// Field 4: Longitude
    pub longitude: f32,
    /// Field 5: E or W (East or West)
    pub e_or_w: EastOrWest,
    /// Field 6: GPS Quality Indicator,
    pub gps_quality_indicator: u32,
    /// Field 7: Number of satellites in view
    pub number_of_satellites_in_view_00: u8,
    /// Field 8: Horizontal Dilution of precision (meters)
    pub horizontal_dilution_of_precision: f32,
    /// Field 9: Antenna Altitude above/below mean-sea-level (geoid) (in meters)
    pub antenna_altitude_above_below_mean_sea_level: f32,
    /// Field 10: Units of antenna altitude, meters
    pub units_of_antenna_altitude_meters: Units,
    /// Field 11: Geoidal separation, the difference between the WGS-84 earth
    pub geoidal_separation_the_difference_between_the: f32,
    /// Field 12: Units of geoidal separation, meters
    pub units_of_geoidal_separation_meters: Units,
    /// Field 13: Age of differential GPS data, time in seconds since last SC104
    pub age_of_differential_gps_data_time: f32,
    /// Field 14: Differential reference station ID, 0000-1023
    pub differential_reference_station_id_0000_1023: u16,
}

impl NmeaMessageEncodable for Gga {
    fn encode_type(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        buffer
            .write_str("GPGGA")
            .map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }

    fn encode_body(&self, mut buffer: impl core::fmt::Write) -> Result<(), NmeaParseError> {
        write!(
            buffer,
            "{},{},{},{:05.2},{},{:05.2},{},{},{},{},{},{},{},{:06X}",
            self.universal_time_coordinated.format("%H%M%S%.2f"),
            self.latitude,
            self.n_or_s,
            self.longitude,
            self.e_or_w,
            self.gps_quality_indicator,
            self.number_of_satellites_in_view_00,
            self.horizontal_dilution_of_precision,
            self.antenna_altitude_above_below_mean_sea_level,
            self.units_of_antenna_altitude_meters,
            self.geoidal_separation_the_difference_between_the,
            self.units_of_geoidal_separation_meters,
            self.age_of_differential_gps_data_time,
            self.differential_reference_station_id_0000_1023,
        )
        .map_err(|_| NmeaParseError::BufferWrite)?;
        Ok(())
    }
}

impl<'a> TryFrom<NmeaMessageBorrowed<'a>> for Gga {
    type Error = NmeaParseError;

    fn try_from(value: NmeaMessageBorrowed<'a>) -> Result<Self, Self::Error> {
        let NmeaMessageBorrowed {
            sentence_type,
            mut fields,
            checksum: _,
        } = value;

        // Check the sentence type matches
        if &sentence_type[2..] != "GGA" {
            return Err(NmeaParseError::InvalidMessageType);
        }

        Ok(Self {
            universal_time_coordinated: fields
                .next()
                .ok_or(NmeaParseError::MissingField(1))
                .and_then(parse_time)?,
            latitude: fields
                .next()
                .ok_or(NmeaParseError::MissingField(2))
                .and_then(|s| s.parse::<f32>().map_err(|e| e.into()))?,
            n_or_s: fields
                .next()
                .ok_or(NmeaParseError::MissingField(3))
                .and_then(|s| s.parse::<NorthOrSouth>().map_err(|e| e.into()))?,
            longitude: fields
                .next()
                .ok_or(NmeaParseError::MissingField(4))
                .and_then(|s| s.parse::<f32>().map_err(|e| e.into()))?,
            e_or_w: fields
                .next()
                .ok_or(NmeaParseError::MissingField(5))
                .and_then(|s| s.parse::<EastOrWest>().map_err(|e| e.into()))?,
            gps_quality_indicator: fields
                .next()
                .ok_or(NmeaParseError::MissingField(6))
                .and_then(|s| s.parse::<u32>().map_err(|e| e.into()))?,
            number_of_satellites_in_view_00: fields
                .next()
                .ok_or(NmeaParseError::MissingField(7))
                .and_then(|s| s.parse::<u8>().map_err(|e| e.into()))?,
            horizontal_dilution_of_precision: fields
                .next()
                .ok_or(NmeaParseError::MissingField(8))
                .and_then(|s| s.parse::<f32>().map_err(|e| e.into()))?,
            antenna_altitude_above_below_mean_sea_level: fields
                .next()
                .ok_or(NmeaParseError::MissingField(9))
                .and_then(|s| parse_maybe_f32(Some(s), 9))?,
            units_of_antenna_altitude_meters: fields
                .next()
                .ok_or(NmeaParseError::MissingField(10))
                .and_then(|s| s.parse::<Units>().map_err(|e| e.into()))?,
            geoidal_separation_the_difference_between_the: fields
                .next()
                .ok_or(NmeaParseError::MissingField(11))
                .and_then(|s| s.parse::<f32>().map_err(|e| e.into()))?,
            units_of_geoidal_separation_meters: fields
                .next()
                .ok_or(NmeaParseError::MissingField(12))
                .and_then(|s| s.parse::<Units>().map_err(|e| e.into()))?,
            age_of_differential_gps_data_time: fields
                .next()
                .ok_or(NmeaParseError::MissingField(13))
                .and_then(|s| parse_maybe_f32(Some(s), 13))?,
            differential_reference_station_id_0000_1023: fields
                .next()
                .ok_or(NmeaParseError::MissingField(14))
                .and_then(|s| s.parse::<u16>().map_err(|e| e.into()))?,
        })
    }
}

fn parse_maybe_f32(s: Option<&str>, index: usize) -> Result<f32, NmeaParseError> {
    let s = match s {
        Some(s) => s,
        None => return Err(NmeaParseError::MissingField(index)),
    };

    if s.is_empty() {
        Ok(0.0)
    } else {
        s.parse::<f32>().map_err(|e| e.into())
    }
}

fn parse_time(s: &str) -> Result<NaiveTime, NmeaParseError> {
    NaiveTime::parse_from_str(s, "%H%M%S%.f").map_err(|_| NmeaParseError::InvalidTimestamp)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use crate::nmea::NMEA_PARSER_STRICT;

    use super::*;

    #[test]
    fn test_parse_encode_alm() {
        let sentence =
            "$GPALM,1,1,15,1159,00,441d,4e,16be,fd5e,a10c9f,4a2da4,686e81,58cbe1,0a4,001*77";
        let expected = Alm {
            total_number_of_messages: 1,
            message_number: 1,
            satellite_prn_number: 15,
            gps_week_number: 1159,
            sv_health_bits_17_24_of: 0,
            eccentricity: 0x441d,
            almanac_reference_time: 0x4e,
            inclination_angle: 0x16be,
            rate_of_right_ascension: 0xfd5e,
            root_of_semi_major_axis: 0xa10c9f,
            argument_of_perigee: 0x4a2da4,
            longitude_of_ascension_node: 0x686e81,
            mean_anomaly: 0x58cbe1,
            f0_clock_parameter: 0x0a4,
            f1_clock_parameter: 0x001,
        };
        let parsed = NMEA_PARSER_STRICT
            .parse_internal(sentence)
            .expect("Failed to parse ALM sentence");
        let alm = Alm::try_from(parsed).expect("Failed to convert to ALM struct");
        assert_eq!(alm, expected);
    }

    #[test]
    fn test_parse_timestamp() {
        NaiveTime::parse_from_str("232047.3", "%H%M%S%.f").unwrap();
    }

    #[test]
    fn test_parse_encode_gga() {
        let sentence = "$GPGGA,172814.0,3723.46587704,N,12202.26957864,W,2,6,1.2,18.893,M,-25.669,M,2.0,0031*4F";
        let expected = Gga {
            universal_time_coordinated: NaiveTime::parse_from_str("172814.0", "%H%M%S%.f").unwrap(),
            latitude: 3723.46587704,
            n_or_s: NorthOrSouth::North,
            longitude: 12202.26957864,
            e_or_w: EastOrWest::West,
            gps_quality_indicator: 2,
            number_of_satellites_in_view_00: 6,
            horizontal_dilution_of_precision: 1.2,
            antenna_altitude_above_below_mean_sea_level: 18.893,
            units_of_antenna_altitude_meters: Units::Meters,
            geoidal_separation_the_difference_between_the: -25.669,
            units_of_geoidal_separation_meters: Units::Meters,
            age_of_differential_gps_data_time: 2.0,
            differential_reference_station_id_0000_1023: 31,
        };
        let parsed = NMEA_PARSER_STRICT
            .parse_internal(sentence)
            .expect("Failed to parse GGA sentence");
        let gga = Gga::try_from(parsed).expect("Failed to convert to GGA struct");
        assert_eq!(gga, expected);
    }
}
