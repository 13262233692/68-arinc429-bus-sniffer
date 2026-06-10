use crate::core::types::{EngineeringValue, SsmSign};
use crate::core::dictionary::LabelDefinition;
use crate::core::word::PAYLOAD_BITS;

#[derive(Debug, Clone)]
pub struct BnrDecoderConfig {
    pub bits: u32,
    pub resolution: f64,
    pub offset: f64,
    pub signed: bool,
    pub unit: String,
    pub range_min: f64,
    pub range_max: f64,
}

impl BnrDecoderConfig {
    pub fn from_label_def(def: &LabelDefinition) -> Self {
        BnrDecoderConfig {
            bits: PAYLOAD_BITS,
            resolution: def.resolution,
            offset: 0.0,
            signed: true,
            unit: def.unit.clone(),
            range_min: def.range_min,
            range_max: def.range_max,
        }
    }

    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    pub fn unsigned(mut self) -> Self {
        self.signed = false;
        self
    }
}

impl Default for BnrDecoderConfig {
    fn default() -> Self {
        BnrDecoderConfig {
            bits: PAYLOAD_BITS,
            resolution: 1.0,
            offset: 0.0,
            signed: true,
            unit: "unit".to_string(),
            range_min: f64::NEG_INFINITY,
            range_max: f64::INFINITY,
        }
    }
}

pub struct BnrDecoder;

impl BnrDecoder {
    pub fn new() -> Self {
        BnrDecoder
    }

    pub fn decode_raw(
        &self,
        data: u32,
        ssm: SsmSign,
        config: &BnrDecoderConfig,
    ) -> Result<EngineeringValue, BnrDecodeError> {
        let sign = match ssm {
            SsmSign::Plus | SsmSign::No => 1.0_f64,
            SsmSign::Minus => -1.0_f64,
            SsmSign::Spare => 1.0_f64,
        };

        let mask = if config.bits == 32 {
            u32::MAX
        } else {
            (1u32 << config.bits) - 1
        };

        let masked_data = data & mask;

        let magnitude = if config.signed {
            let sign_bit = 1u32 << (config.bits - 1);
            if (masked_data & sign_bit) != 0 {
                -(((!masked_data & mask) + 1) as f64)
            } else {
                masked_data as f64
            }
        } else {
            masked_data as f64
        };

        let raw_value = magnitude * config.resolution;
        let signed_value = raw_value * sign;
        let final_value = signed_value + config.offset;

        if final_value < config.range_min || final_value > config.range_max {
            return Err(BnrDecodeError::OutOfRange {
                value: final_value,
                min: config.range_min,
                max: config.range_max,
            });
        }

        let display = format_value_with_precision(final_value, config.resolution, &config.unit);

        Ok(EngineeringValue {
            value: final_value,
            unit: config.unit.clone(),
            display,
        })
    }

    pub fn decode_with_label(
        &self,
        data: u32,
        ssm: SsmSign,
        def: &LabelDefinition,
    ) -> Result<EngineeringValue, BnrDecodeError> {
        let config = BnrDecoderConfig::from_label_def(def);
        self.decode_raw(data, ssm, &config)
    }
}

impl Default for BnrDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BnrDecodeError {
    OutOfRange { value: f64, min: f64, max: f64 },
    InvalidBits { bits: u32 },
    SsmSpare,
}

impl std::fmt::Display for BnrDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BnrDecodeError::OutOfRange { value, min, max } => {
                write!(f, "Value {} out of range [{}, {}]", value, min, max)
            }
            BnrDecodeError::InvalidBits { bits } => {
                write!(f, "Invalid bit width: {} (must be 1-32)", bits)
            }
            BnrDecodeError::SsmSpare => {
                write!(f, "SSM=Spare indicates invalid/No computed data")
            }
        }
    }
}

impl std::error::Error for BnrDecodeError {}

fn format_value_with_precision(value: f64, resolution: f64, unit: &str) -> String {
    let precision = if resolution >= 1.0 {
        0
    } else if resolution >= 0.1 {
        1
    } else if resolution >= 0.01 {
        2
    } else if resolution >= 0.001 {
        3
    } else if resolution >= 0.0001 {
        4
    } else if resolution >= 0.000001 {
        6
    } else {
        8
    };

    let abs_val = value.abs();
    let value_str = if abs_val >= 1000000.0 && precision <= 2 {
        format!("{:.2e}", value)
    } else if abs_val < 0.0001 && abs_val > 0.0 && precision < 6 {
        format!("{:.6e}", value)
    } else {
        format!("{:.*}", precision, value)
    };

    format!("{} {}", value_str, unit)
}

pub fn decode_bnr_altitude(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.5,
        offset: 0.0,
        signed: true,
        unit: "ft".to_string(),
        range_min: -1000.0,
        range_max: 147000.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_tas(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.5,
        offset: 0.0,
        signed: false,
        unit: "kt".to_string(),
        range_min: 0.0,
        range_max: 2000.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_mach(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.001,
        offset: 0.0,
        signed: false,
        unit: "Mach".to_string(),
        range_min: 0.0,
        range_max: 5.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_pitch(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.00390625,
        offset: 0.0,
        signed: true,
        unit: "deg".to_string(),
        range_min: -90.0,
        range_max: 90.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_heading(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.00390625,
        offset: 0.0,
        signed: false,
        unit: "deg".to_string(),
        range_min: 0.0,
        range_max: 360.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_lat_lon(data: u32, ssm: SsmSign, is_lat: bool) -> Result<EngineeringValue, BnrDecodeError> {
    let (range_min, range_max, unit) = if is_lat {
        (-90.0, 90.0, "deg (Lat)")
    } else {
        (-180.0, 180.0, "deg (Lon)")
    };
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.000000596,
        offset: 0.0,
        signed: true,
        unit: unit.to_string(),
        range_min,
        range_max,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_n1(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.015625,
        offset: 0.0,
        signed: false,
        unit: "%".to_string(),
        range_min: 0.0,
        range_max: 150.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_egt(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.25,
        offset: 0.0,
        signed: true,
        unit: "°C".to_string(),
        range_min: -200.0,
        range_max: 2000.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

pub fn decode_bnr_fuel(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BnrDecodeError> {
    let config = BnrDecoderConfig {
        bits: 19,
        resolution: 0.1,
        offset: 0.0,
        signed: false,
        unit: "kg".to_string(),
        range_min: 0.0,
        range_max: 100000.0,
    };
    BnrDecoder.decode_raw(data, ssm, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bnr_radio_altitude_zero() {
        let result = decode_bnr_altitude(0, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 0.0);
        assert!(result.display.contains("ft"));
    }

    #[test]
    fn test_bnr_radio_altitude_positive() {
        let data = 400u32;
        let result = decode_bnr_altitude(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 200.0);
        assert_eq!(result.display, "200.0 ft");
    }

    #[test]
    fn test_bnr_radio_altitude_negative() {
        let data = 8u32;
        let result = decode_bnr_altitude(data, SsmSign::Minus).unwrap();
        assert_eq!(result.value, -4.0);
    }

    #[test]
    fn test_bnr_tas_typical() {
        let data = 500u32;
        let result = decode_bnr_tas(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 250.0);
        assert!(result.display.contains("kt"));
    }

    #[test]
    fn test_bnr_mach_high() {
        let data = 820u32;
        let result = decode_bnr_mach(data, SsmSign::Plus).unwrap();
        assert!((result.value - 0.820).abs() < 0.0001);
    }

    #[test]
    fn test_bnr_pitch_up() {
        let data = 2560u32;
        let result = decode_bnr_pitch(data, SsmSign::Plus).unwrap();
        assert!((result.value - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_bnr_heading() {
        let data = 46080u32;
        let result = decode_bnr_heading(data, SsmSign::Plus).unwrap();
        assert!((result.value - 180.0).abs() < 0.01);
    }

    #[test]
    fn test_bnr_out_of_range() {
        let data = 500000u32;
        let result = decode_bnr_tas(data, SsmSign::Plus);
        assert!(matches!(result, Err(BnrDecodeError::OutOfRange { .. })));
    }

    #[test]
    fn test_bnr_precision_formatting() {
        let v1 = format_value_with_precision(123.456, 0.01, "m");
        assert_eq!(v1, "123.46 m");

        let v2 = format_value_with_precision(42.0, 1.0, "ft");
        assert_eq!(v2, "42 ft");

        let v3 = format_value_with_precision(0.825, 0.001, "Mach");
        assert_eq!(v3, "0.825 Mach");
    }

    #[test]
    fn test_bnr_twos_complement_negative() {
        let mut config = BnrDecoderConfig::default();
        config.bits = 19;
        config.resolution = 0.5;
        config.signed = true;
        config.unit = "ft".to_string();
        config.range_min = -1000.0;
        config.range_max = 10000.0;

        let neg_data = 0x7FFFFu32 - 99;
        let result = BnrDecoder.decode_raw(neg_data, SsmSign::Plus, &config).unwrap();
        assert!((result.value - -50.0).abs() < 1.0);
    }

    #[test]
    fn test_bnr_lat() {
        let data = 33557u32;
        let result = decode_bnr_lat_lon(data, SsmSign::Plus, true).unwrap();
        assert!((result.value - 0.02).abs() < 0.001);
    }

    #[test]
    fn test_bnr_n1_idle() {
        let data = 1408u32;
        let result = decode_bnr_n1(data, SsmSign::Plus).unwrap();
        assert!((result.value - 22.0).abs() < 0.1);
    }

    #[test]
    fn test_bnr_egt_hot() {
        let data = 3000u32;
        let result = decode_bnr_egt(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 750.0);
    }

    #[test]
    fn test_bnr_fuel_qty() {
        let data = 120000u32;
        let result = decode_bnr_fuel(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 12000.0);
    }
}
