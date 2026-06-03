use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// NMEA message types, as defined by the NMEA standard (e.g. GGA, RMC, etc.)
///
/// NOTE: this should be -exhaustive-, but devices also often have proprietary message types
/// so we sometimes need to support an Other variant.
#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum NmeaMessageType {
    /// Waypoint Arrival Alarm
    AAM,
    /// GPS Almanac Data
    ALM,
    /// Autopilot Sentence "A"
    APA,
    /// Autopilot Sentence "B"
    APB,
    /// Bearing - Waypoint to Waypoint
    BOD,
    /// Bearing and Distance to Waypoint - Great Circle
    BWC,
    /// Bearing and Distance to Waypoint - Rhumb Line
    BWR,
    /// Bearing - Waypoint to Waypoint
    BWW,
    /// Depth Below Keel
    DBK,
    /// Depth Below Surface
    DBS,
    /// Depth below transducer
    DBT,
    /// Decca Position
    DCN,
    /// Depth of Water
    DPT,
    /// Datum Reference
    DTM,
    /// Frequency Set Information
    FSI,
    /// GPS Satellite Fault Detection
    GBS,
    /// Global Positioning System Fix Data
    GGA,
    /// Geographic Position, Loran-C
    GLC,
    /// Geographic Position - Latitude/Longitude
    GLL,
    /// GPS Range Residuals
    GRS,
    /// GPS Pseudorange Noise Statistics
    GST,
    /// GPS DOP and active satellites
    GSA,
    /// Satellites in view
    GSV,
    /// Geographic Location in Time Differences
    GTD,
    /// TRANSIT Position - Latitude/Longitude
    GXA,
    /// Heading - Deviation & Variation
    HDG,
    /// Heading - Magnetic
    HDM,
    /// Heading - True
    HDT,
    /// Trawl Headrope to Footrope and Bottom
    HFB,
    /// Heading Steering Command
    HSC,
    /// Trawl Door Spread 2 Distance
    ITS,
    /// Loran-C Signal Data
    LCD,
    /// Control for a Beacon Receiver
    MSK,
    /// Beacon Receiver Status
    MSS,
    /// Mean Temperature of Water
    MTW,
    /// Wind Speed and Angle
    MWV,
    /// Omega Lane Numbers
    OLN,
    /// Own Ship Data
    OSD,
    /// Waypoints in active route
    R00,
    /// Recommended Minimum Navigation Information
    RMA,
    /// Recommended Minimum Navigation Information
    RMB,
    /// Recommended Minimum Navigation Information
    RMC,
    /// Rate Of Turn
    ROT,
    /// Revolutions
    RPM,
    /// Rudder Sensor Angle
    RSA,
    /// RADAR System Data
    RSD,
    /// Routes
    RTE,
    /// Scanning Frequency Information
    SFI,
    /// Multiple Data ID
    STN,
    /// Trawl Door Spread Distance
    TDS,
    /// Trawl Filling Indicator
    TFI,
    /// Trawl Position Cartesian Coordinates
    TPC,
    /// Trawl Position Relative Vessel
    TPR,
    /// Trawl Position True
    TPT,
    /// TRANSIT Fix Data
    TRF,
    /// Tracked Target Message
    TTM,
    /// Dual Ground/Water Speed
    VBW,
    /// Set and Drift
    VDR,
    /// Water speed and heading
    VHW,
    /// Distance Traveled through Water
    VLW,
    /// Speed - Measured Parallel to Wind
    VPW,
    /// Track made good and Ground speed
    VTG,
    /// Relative Wind Speed and Angle
    VWR,
    /// Waypoint Closure Velocity
    WCV,
    /// Distance - Waypoint to Waypoint
    WNC,
    /// Waypoint Location
    WPL,
    /// Cross Track Error - Dead Reckoning
    XDR,
    /// Cross-Track Error, Measured
    XTE,
    /// Cross Track Error - Dead Reckoning
    XTR,
    /// Time & Date - UTC, day, month, year and local time zone
    ZDA,
    /// UTC & Time from origin Waypoint
    ZFO,
    /// UTC & Time to Destination Waypoint
    ZTG,
    /// Autopilot System Data
    ASD,
    /// Digital Selective Calling Information
    DSC,
    /// Extended DSC
    DSE,
    /// DSC Transponder Initiate
    DSI,
    /// DSC Transponder Response
    DSR,
    /// Wind Direction & Speed
    MWD,
    /// Target Latitude and Longitude
    TLL,
    /// Distance to Waypoint - Rhumb Line
    WDR,
    /// Distance to Waypoint - Great Circle
    WDC,
    /// Time and Distance to Variable Point
    ZDL,
    /// Garmin Estimated Error
    PGRME,
    /// uBlox Lat/Long Position Data
    PUBX00,
    /// uBlox UTM Position Data
    PUBX01,
    /// uBlox Satellite Status
    PUBX03,
    /// uBlox Time of Day and Clock Information
    PUBX04,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum TalkerId {
    /// Autopilot - General
    AG,
    /// Autopilot - Magnetic
    AP,
    /// Computer - Programmed Calculator (outdated)
    CC,
    /// Communications - Digital Selective Calling (DSC)
    CD,
    /// Computer - Memory Data (outdated)
    CM,
    /// Communications - Satellite
    CS,
    /// Communications - Radio-Telephone (MF/HF)
    CT,
    /// Communications - Radio-Telephone (VHF)
    CV,
    /// Communications - Scanning Receiver
    CX,
    /// DECCA Navigation (outdated)
    DE,
    /// Direction Finder
    DF,
    /// Electronic Chart Display & Information System (ECDIS)
    EC,
    /// Emergency Position Indicating Beacon (EPIRB)
    EP,
    /// Engine Room Monitoring Systems
    ER,
    /// Global Positioning System (GPS)
    GP,
    /// Heading - Magnetic Compass
    HC,
    /// Heading - North Seeking Gyro
    HE,
    /// Heading - Non North Seeking Gyro
    HN,
    /// Integrated Instrumentation
    II,
    /// Integrated Navigation
    IN,
    /// Loran A (outdated)
    LA,
    /// Loran C
    LC,
    /// Microwave Positioning System (outdated)
    MP,
    /// OMEGA Navigation System (outdated)
    OM,
    /// Distress Alarm System (outdated)
    OS,
    /// RADAR and/or ARPA
    RA,
    /// Sounder, Depth
    SD,
    /// Electronic Positioning System, other/general
    SN,
    /// Sounder, Scanning
    SS,
    /// Turn Rate Indicator
    TI,
    /// TRANSIT Navigation System
    TR,
    /// Velocity Sensor, Doppler, other/general
    VD,
    /// Velocity Sensor, Speed Log, Water, Magnetic
    DM,
    /// Velocity Sensor, Speed Log, Water, Mechanical
    VW,
    /// Weather Instruments
    WI,
    /// Transducer - Temperature (outdated)
    YC,
    /// Transducer - Displacement, Angular or Linear (outdated)
    YD,
    /// Transducer - Frequency (outdated)
    YF,
    /// Transducer - Level (outdated)
    YL,
    /// Transducer - Pressure (outdated)
    YP,
    /// Transducer - Flow Rate (outdated)
    YR,
    /// Transducer - Tachometer (outdated)
    YT,
    /// Transducer - Volume (outdated)
    YV,
    /// Transducer
    YX,
    /// Timekeeper - Atomic Clock
    ZA,
    /// Timekeeper - Chronometer
    ZC,
    /// Timekeeper - Quartz
    ZQ,
    /// Timekeeper - Radio Update, WWV or WWVH
    ZV,
}
