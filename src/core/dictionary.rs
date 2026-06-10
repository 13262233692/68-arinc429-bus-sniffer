use std::collections::HashMap;
use std::sync::OnceLock;

use crate::core::types::PayloadFormat;

#[derive(Debug, Clone)]
pub struct LabelDefinition {
    pub label_octal: u16,
    pub label_name: String,
    pub param_name: String,
    pub equipment: String,
    pub format: PayloadFormat,
    pub unit: String,
    pub resolution: f64,
    pub range_min: f64,
    pub range_max: f64,
    pub description: String,
}

impl LabelDefinition {
    pub fn new(
        label_octal: u16,
        label_name: &str,
        param_name: &str,
        equipment: &str,
        format: PayloadFormat,
        unit: &str,
        resolution: f64,
        range_min: f64,
        range_max: f64,
        description: &str,
    ) -> Self {
        LabelDefinition {
            label_octal,
            label_name: label_name.to_string(),
            param_name: param_name.to_string(),
            equipment: equipment.to_string(),
            format,
            unit: unit.to_string(),
            resolution,
            range_min,
            range_max,
            description: description.to_string(),
        }
    }
}

pub fn get_avionics_dictionary() -> &'static HashMap<u16, LabelDefinition> {
    static DICT: OnceLock<HashMap<u16, LabelDefinition>> = OnceLock::new();

    DICT.get_or_init(|| {
        let mut dict: HashMap<u16, LabelDefinition> = HashMap::new();

        dict.insert(0o001, LabelDefinition::new(
            0o001, "LBL001", "RADIO_ALTITUDE", "LRALT",
            PayloadFormat::Bnr, "ft",
            0.5, -1000.0, 147000.0,
            "Radar Altitude - CW Pulse Radar Altimeter Measurement"
        ));

        dict.insert(0o002, LabelDefinition::new(
            0o002, "LBL002", "TRUE_AIRSPEED", "ADIRU",
            PayloadFormat::Bnr, "kt",
            0.5, 0.0, 2000.0,
            "True Airspeed - Air Data Inertial Reference Unit"
        ));

        dict.insert(0o003, LabelDefinition::new(
            0o003, "LBL003", "INDICATED_AIRSPEED", "ADIRU",
            PayloadFormat::Bnr, "kt",
            0.25, 0.0, 1000.0,
            "Indicated Airspeed (IAS)"
        ));

        dict.insert(0o004, LabelDefinition::new(
            0o004, "LBL004", "MACH_NUMBER", "ADIRU",
            PayloadFormat::Bnr, "Mach",
            0.001, 0.0, 5.0,
            "Mach Number"
        ));

        dict.insert(0o005, LabelDefinition::new(
            0o005, "LBL005", "BAROMETRIC_ALTITUDE", "ADIRU",
            PayloadFormat::Bnr, "ft",
            1.0, -1500.0, 50000.0,
            "Barometric Corrected Altitude (Pressure Altitude)"
        ));

        dict.insert(0o010, LabelDefinition::new(
            0o010, "LBL010", "VERTICAL_SPEED", "ADIRU",
            PayloadFormat::Bnr, "ft/min",
            4.0, -8000.0, 8000.0,
            "Vertical Speed / Rate of Climb"
        ));

        dict.insert(0o011, LabelDefinition::new(
            0o011, "LBL011", "PITCH_ATTITUDE", "IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, -90.0, 90.0,
            "Pitch Attitude (Theta Angle) - Inertial Reference"
        ));

        dict.insert(0o012, LabelDefinition::new(
            0o012, "LBL012", "ROLL_ATTITUDE", "IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, -180.0, 180.0,
            "Roll Attitude (Phi Angle) - Inertial Reference"
        ));

        dict.insert(0o013, LabelDefinition::new(
            0o013, "LBL013", "TRUE_HEADING", "IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, 0.0, 360.0,
            "True Heading - Inertial Reference"
        ));

        dict.insert(0o014, LabelDefinition::new(
            0o014, "LBL014", "MAGNETIC_HEADING", "IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, 0.0, 360.0,
            "Magnetic Heading - Slaved to Magnetic Compass"
        ));

        dict.insert(0o015, LabelDefinition::new(
            0o015, "LBL015", "YAW_RATE", "IRU",
            PayloadFormat::Bnr, "deg/s",
            0.00390625, -30.0, 30.0,
            "Yaw Rate - Inertial Angular Rate Sensor"
        ));

        dict.insert(0o016, LabelDefinition::new(
            0o016, "LBL016", "LATERAL_ACCEL", "IRU",
            PayloadFormat::Bnr, "g",
            0.0009765625, -2.0, 2.0,
            "Lateral Acceleration - Body Axis"
        ));

        dict.insert(0o017, LabelDefinition::new(
            0o017, "LBL017", "NORMAL_ACCEL", "IRU",
            PayloadFormat::Bnr, "g",
            0.0009765625, -4.0, 4.0,
            "Normal (Vertical) Acceleration - Body Axis"
        ));

        dict.insert(0o020, LabelDefinition::new(
            0o020, "LBL020", "LATITUDE", "GPS/IRU",
            PayloadFormat::Bnr, "deg",
            0.000000596, -90.0, 90.0,
            "Geodetic Latitude - WGS84"
        ));

        dict.insert(0o021, LabelDefinition::new(
            0o021, "LBL021", "LONGITUDE", "GPS/IRU",
            PayloadFormat::Bnr, "deg",
            0.000000596, -180.0, 180.0,
            "Geodetic Longitude - WGS84"
        ));

        dict.insert(0o022, LabelDefinition::new(
            0o022, "LBL022", "GROUND_SPEED", "GPS/IRU",
            PayloadFormat::Bnr, "kt",
            0.25, 0.0, 2000.0,
            "Ground Speed - Inertial / GPS Derived"
        ));

        dict.insert(0o023, LabelDefinition::new(
            0o023, "LBL023", "GROUND_TRACK", "GPS/IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, 0.0, 360.0,
            "Ground Track Angle - Inertial / GPS"
        ));

        dict.insert(0o024, LabelDefinition::new(
            0o024, "LBL024", "DRIFT_ANGLE", "IRU",
            PayloadFormat::Bnr, "deg",
            0.00390625, -30.0, 30.0,
            "Drift Angle (Track - Heading)"
        ));

        dict.insert(0o025, LabelDefinition::new(
            0o025, "LBL025", "WIND_DIRECTION", "ADC",
            PayloadFormat::Bnr, "deg",
            0.3515625, 0.0, 360.0,
            "Wind Direction - True Reference"
        ));

        dict.insert(0o026, LabelDefinition::new(
            0o026, "LBL026", "WIND_SPEED", "ADC",
            PayloadFormat::Bnr, "kt",
            0.5, 0.0, 300.0,
            "Wind Speed - Computed from TAS & GS"
        ));

        dict.insert(0o030, LabelDefinition::new(
            0o030, "LBL030", "VHF_NAV_FREQ", "VHFNAV",
            PayloadFormat::Bcd, "MHz",
            0.005, 108.0, 136.975,
            "VHF NAV/COM Frequency - BCD Encoded"
        ));

        dict.insert(0o031, LabelDefinition::new(
            0o031, "LBL031", "ADF_FREQUENCY", "ADF",
            PayloadFormat::Bcd, "kHz",
            0.5, 190.0, 1800.0,
            "Automatic Direction Finder Frequency"
        ));

        dict.insert(0o032, LabelDefinition::new(
            0o032, "LBL032", "DME_FREQUENCY", "DME",
            PayloadFormat::Bcd, "MHz",
            0.05, 960.0, 1215.0,
            "Distance Measuring Equipment Frequency"
        ));

        dict.insert(0o033, LabelDefinition::new(
            0o033, "LBL033", "XPDR_CODE", "TRANSPONDER",
            PayloadFormat::Bcd, "octal",
            1.0, 0.0, 7777.0,
            "ATC Transponder Mode A Code (4-digit Octal)"
        ));

        dict.insert(0o034, LabelDefinition::new(
            0o034, "LBL034", "NAV_COURSE", "NAV",
            PayloadFormat::Bcd, "deg",
            1.0, 0.0, 359.0,
            "Selected Navigation Course (VOR/Loc)"
        ));

        dict.insert(0o040, LabelDefinition::new(
            0o040, "LBL040", "ENG_N1", "EEC",
            PayloadFormat::Bnr, "%",
            0.015625, 0.0, 150.0,
            "Engine N1 Fan Speed (% RPM)"
        ));

        dict.insert(0o041, LabelDefinition::new(
            0o041, "LBL041", "ENG_N2", "EEC",
            PayloadFormat::Bnr, "%",
            0.015625, 0.0, 150.0,
            "Engine N2 Core Speed (% RPM)"
        ));

        dict.insert(0o042, LabelDefinition::new(
            0o042, "LBL042", "ENG_EGT", "EEC",
            PayloadFormat::Bnr, "degC",
            0.25, -200.0, 2000.0,
            "Engine Exhaust Gas Temperature"
        ));

        dict.insert(0o043, LabelDefinition::new(
            0o043, "LBL043", "ENG_FUEL_FLOW", "EEC",
            PayloadFormat::Bnr, "lb/hr",
            1.0, 0.0, 20000.0,
            "Engine Fuel Flow Rate"
        ));

        dict.insert(0o044, LabelDefinition::new(
            0o044, "LBL044", "OIL_PRESSURE", "EEC",
            PayloadFormat::Bnr, "psi",
            0.25, 0.0, 400.0,
            "Engine Oil Pressure"
        ));

        dict.insert(0o045, LabelDefinition::new(
            0o045, "LBL045", "OIL_TEMPERATURE", "EEC",
            PayloadFormat::Bnr, "degC",
            0.25, -50.0, 300.0,
            "Engine Oil Temperature"
        ));

        dict.insert(0o050, LabelDefinition::new(
            0o050, "LBL050", "FUEL_QTY_TOTAL", "FQMS",
            PayloadFormat::Bnr, "kg",
            0.1, 0.0, 100000.0,
            "Total Fuel Quantity On Board"
        ));

        dict.insert(0o051, LabelDefinition::new(
            0o051, "LBL051", "FUEL_QTY_LEFT", "FQMS",
            PayloadFormat::Bnr, "kg",
            0.1, 0.0, 50000.0,
            "Left Wing Tank Fuel Quantity"
        ));

        dict.insert(0o052, LabelDefinition::new(
            0o052, "LBL052", "FUEL_QTY_RIGHT", "FQMS",
            PayloadFormat::Bnr, "kg",
            0.1, 0.0, 50000.0,
            "Right Wing Tank Fuel Quantity"
        ));

        dict.insert(0o060, LabelDefinition::new(
            0o060, "LBL060", "HYD_PRESS_SYS1", "HYD",
            PayloadFormat::Bnr, "psi",
            0.5, 0.0, 5000.0,
            "Hydraulic System 1 Pressure"
        ));

        dict.insert(0o061, LabelDefinition::new(
            0o061, "LBL061", "HYD_PRESS_SYS2", "HYD",
            PayloadFormat::Bnr, "psi",
            0.5, 0.0, 5000.0,
            "Hydraulic System 2 Pressure"
        ));

        dict.insert(0o062, LabelDefinition::new(
            0o062, "LBL062", "HYD_QUANTITY", "HYD",
            PayloadFormat::Bnr, "L",
            0.1, 0.0, 200.0,
            "Hydraulic Fluid Quantity"
        ));

        dict.insert(0o070, LabelDefinition::new(
            0o070, "LBL070", "APU_GEN_FREQ", "APU",
            PayloadFormat::Bnr, "Hz",
            0.1, 350.0, 450.0,
            "APU Generator Output Frequency"
        ));

        dict.insert(0o071, LabelDefinition::new(
            0o071, "LBL071", "APU_GEN_VOLT", "APU",
            PayloadFormat::Bnr, "V",
            0.1, 0.0, 150.0,
            "APU Generator Output Voltage (Phase-Neutral)"
        ));

        dict.insert(0o072, LabelDefinition::new(
            0o072, "LBL072", "CABIN_ALT", "PCS",
            PayloadFormat::Bnr, "ft",
            1.0, -1500.0, 45000.0,
            "Cabin Altitude (Pressurization)"
        ));

        dict.insert(0o073, LabelDefinition::new(
            0o073, "LBL073", "CABIN_DIFF_PRESS", "PCS",
            PayloadFormat::Bnr, "psi",
            0.00390625, 0.0, 15.0,
            "Cabin Differential Pressure"
        ));

        dict.insert(0o100, LabelDefinition::new(
            0o100, "LBL100", "FLAP_POSITION", "FCC",
            PayloadFormat::Bcd, "deg",
            1.0, 0.0, 45.0,
            "Flap Lever / Surface Position"
        ));

        dict.insert(0o101, LabelDefinition::new(
            0o101, "LBL101", "SLAT_POSITION", "FCC",
            PayloadFormat::Bcd, "deg",
            1.0, 0.0, 25.0,
            "Slat Position Indication"
        ));

        dict.insert(0o102, LabelDefinition::new(
            0o102, "LBL102", "GEAR_DISCRETE", "LGCIU",
            PayloadFormat::Discrete, "state",
            1.0, 0.0, 7.0,
            "Landing Gear Discrete Word: UP/DOWN/LOCK"
        ));

        dict.insert(0o103, LabelDefinition::new(
            0o103, "LBL103", "DOOR_STATUS", "DSCU",
            PayloadFormat::Discrete, "bitmask",
            1.0, 0.0, 524287.0,
            "Door Open/Closed/Locked Status Bitmask"
        ));

        dict.insert(0o104, LabelDefinition::new(
            0o104, "LBL104", "ANTI_ICE_STATUS", "AI",
            PayloadFormat::Discrete, "bitmask",
            1.0, 0.0, 65535.0,
            "Anti-Ice System ON/OFF Status"
        ));

        dict.insert(0o105, LabelDefinition::new(
            0o105, "LBL105", "AUTOPILOT_MODE", "FCU",
            PayloadFormat::Discrete, "enum",
            1.0, 0.0, 255.0,
            "Autopilot Active Modes Bitmask"
        ));

        dict.insert(0o110, LabelDefinition::new(
            0o110, "LBL110", "MAINT_WORD", "CMC",
            PayloadFormat::Maintenance, "code",
            1.0, 0.0, 524287.0,
            "Central Maintenance Computer Fault Code"
        ));

        dict.insert(0o111, LabelDefinition::new(
            0o111, "LBL111", "EQUIP_ID_ACK", "SYS",
            PayloadFormat::Ack, "id",
            1.0, 0.0, 65535.0,
            "Equipment Identification Acknowledge"
        ));

        dict.insert(0o377, LabelDefinition::new(
            0o377, "LBL377", "NULL_WORD", "SYS",
            PayloadFormat::Unknown, "N/A",
            1.0, 0.0, 0.0,
            "Null / Sync Word - Bus Idle Filler"
        ));

        dict
    })
}

pub fn lookup_label(label_octal: u16) -> Option<&'static LabelDefinition> {
    get_avionics_dictionary().get(&label_octal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_lookup() {
        let entry = lookup_label(0o001);
        assert!(entry.is_some());
        let def = entry.unwrap();
        assert_eq!(def.param_name, "RADIO_ALTITUDE");
        assert_eq!(def.format, PayloadFormat::Bnr);
        assert_eq!(def.unit, "ft");
        assert_eq!(def.resolution, 0.5);
    }

    #[test]
    fn test_dictionary_missing_label() {
        let entry = lookup_label(0o177);
        assert!(entry.is_none());
    }

    #[test]
    fn test_dictionary_size() {
        let dict = get_avionics_dictionary();
        assert!(dict.len() >= 40);
    }

    #[test]
    fn test_bnr_labels_present() {
        let bnr_labels = [0o001, 0o002, 0o005, 0o011, 0o012, 0o013, 0o020, 0o021, 0o040, 0o042, 0o050];
        for lbl in bnr_labels.iter() {
            let entry = lookup_label(*lbl);
            assert!(entry.is_some(), "Missing BNR label {:03o}", lbl);
            assert_eq!(entry.unwrap().format, PayloadFormat::Bnr,
                "Label {:03o} should be BNR", lbl);
        }
    }

    #[test]
    fn test_bcd_labels_present() {
        let bcd_labels = [0o030, 0o031, 0o032, 0o033, 0o034, 0o100];
        for lbl in bcd_labels.iter() {
            let entry = lookup_label(*lbl);
            assert!(entry.is_some(), "Missing BCD label {:03o}", lbl);
            assert_eq!(entry.unwrap().format, PayloadFormat::Bcd,
                "Label {:03o} should be BCD", lbl);
        }
    }

    #[test]
    fn test_discrete_labels_present() {
        let disc_labels = [0o102, 0o103, 0o104, 0o105];
        for lbl in disc_labels.iter() {
            let entry = lookup_label(*lbl);
            assert!(entry.is_some(), "Missing Discrete label {:03o}", lbl);
            assert_eq!(entry.unwrap().format, PayloadFormat::Discrete,
                "Label {:03o} should be Discrete", lbl);
        }
    }
}
