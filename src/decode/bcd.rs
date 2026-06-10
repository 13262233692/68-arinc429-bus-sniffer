use crate::core::types::{EngineeringValue, SsmSign};
use crate::core::dictionary::LabelDefinition;

#[derive(Debug, Clone)]
pub struct BcdDigitLayout {
    pub n_digits: usize,
    pub decimal_pos: i32,
    pub is_octal: bool,
    pub unit: String,
    pub sign_from_ssm: bool,
    pub resolution: f64,
}

impl BcdDigitLayout {
    pub fn new(n_digits: usize) -> Self {
        BcdDigitLayout {
            n_digits,
            decimal_pos: 0,
            is_octal: false,
            unit: "".to_string(),
            sign_from_ssm: false,
            resolution: 1.0,
        }
    }

    pub fn with_decimal(mut self, pos: i32) -> Self {
        self.decimal_pos = pos;
        self
    }

    pub fn octal(mut self) -> Self {
        self.is_octal = true;
        self
    }

    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = unit.to_string();
        self
    }

    pub fn with_sign_from_ssm(mut self) -> Self {
        self.sign_from_ssm = true;
        self
    }

    pub fn with_resolution(mut self, res: f64) -> Self {
        self.resolution = res;
        self
    }
}

#[derive(Debug)]
pub struct BcdDecoder;

impl BcdDecoder {
    pub fn new() -> Self {
        BcdDecoder
    }

    pub fn decode_raw(
        &self,
        data: u32,
        ssm: SsmSign,
        layout: &BcdDigitLayout,
    ) -> Result<EngineeringValue, BcdDecodeError> {
        let _digits = Vec::with_capacity(layout.n_digits);
        let value = Self::extract_digits(data, layout, _digits)?;

        let sign = if layout.sign_from_ssm {
            match ssm {
                SsmSign::Plus | SsmSign::No => 1.0_f64,
                SsmSign::Minus => -1.0_f64,
                SsmSign::Spare => 1.0_f64,
            }
        } else {
            1.0_f64
        };

        let fvalue = value as f64 * sign * layout.resolution;
        let fvalue = fvalue / 10.0_f64.powi(layout.decimal_pos);
        let display = self.format_display(fvalue, layout);

        Ok(EngineeringValue {
            value: fvalue,
            unit: layout.unit.clone(),
            display,
        })
    }

    fn extract_digits(
        data: u32,
        layout: &BcdDigitLayout,
        _digits: Vec<u8>) -> Result<u64, BcdDecodeError> {
            let mut value: u64 = 0;
            let digit_base = if layout.is_octal { 8 } else { 10 };

            let total_shift = (layout.n_digits - 1) * 4;
            let mut mask_shift: i32 = total_shift as i32;

            for i in 0..layout.n_digits {
                let digit = if mask_shift >= 0 {
                    ((data >> mask_shift) & 0x0F) as u8
                } else {
                    0u8
                };
                mask_shift -= 4;

                if digit >= digit_base {
                    return Err(BcdDecodeError::InvalidDigit {
                        position: i,
                        digit,
                        base: digit_base as u8,
                    });
                }

                value = value * (digit_base as u64) + (digit as u64);
            }

            let _ = _digits;
            Ok(value)
        }

    pub fn decode_with_label(
        &self,
        data: u32,
        ssm: SsmSign,
        _def: &LabelDefinition,
    ) -> Result<EngineeringValue, BcdDecodeError> {
        let layout = BcdDigitLayout::new(5)
            .with_decimal(3)
            .with_unit(&_def.unit)
            .with_resolution(_def.resolution);
        self.decode_raw(data, ssm, &layout)
    }

    fn format_display(&self, value: f64, layout: &BcdDigitLayout) -> String {
        if layout.is_octal {
            format!("{:0width$o} {}",
                value as u64,
                layout.unit,
                width = layout.n_digits)
        } else if layout.decimal_pos > 0 {
            let precision = layout.decimal_pos as usize;
            format!("{:.*} {}", precision, value, layout.unit)
        } else {
            format!("{} {}", value as u64, layout.unit)
        }
    }
}

impl Default for BcdDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BcdDecodeError {
    InvalidDigit { position: usize, digit: u8, base: u8 },
    TooManyDigits { requested: usize, max: usize },
}

impl std::fmt::Display for BcdDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BcdDecodeError::InvalidDigit { position, digit, base } => {
                write!(f, "Invalid digit {} at position {} (base {})",
                digit, position, base)
            }
            BcdDecodeError::TooManyDigits { requested, max } => {
                write!(f, "Requested {} digits, max is {}", requested, max)
            }
        }
    }
}

impl std::error::Error for BcdDecodeError {}

pub fn decode_vhf_freq(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(5)
        .with_decimal(2)
        .with_unit("MHz");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

pub fn decode_adf_freq(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(4)
        .with_decimal(0)
        .with_unit("kHz");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

pub fn decode_dme_freq(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(5)
        .with_decimal(1)
        .with_unit("MHz");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

pub fn decode_xpdr_code(data: u32, _ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(4)
        .octal()
        .with_unit("XPDR");
    BcdDecoder.decode_raw(data, _ssm, &layout)
}

pub fn decode_flap_angle(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(2)
        .with_unit("deg");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

pub fn decode_slat_angle(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(2)
        .with_unit("deg");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

pub fn decode_course(data: u32, ssm: SsmSign) -> Result<EngineeringValue, BcdDecodeError> {
    let layout = BcdDigitLayout::new(3)
        .with_unit("deg");
    BcdDecoder.decode_raw(data, ssm, &layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcd_basic_decimal() {
        let data = 0x00123u32;
        let layout = BcdDigitLayout::new(5)
            .with_decimal(3)
            .with_unit("MHz");
        let result = BcdDecoder.decode_raw(data, SsmSign::Plus, &layout).unwrap();
        assert!((result.value - 0.123).abs() < 0.0001);
    }

    #[test]
    fn test_vhf_freq_118_000() {
        let data = 0x11800u32;
        let result = decode_vhf_freq(data, SsmSign::Plus).unwrap();
        assert!((result.value - 118.00).abs() < 0.01);
        assert!(result.display.contains("118"));
    }

    #[test]
    fn test_vhf_freq_121_500() {
        let data = 0x12150u32;
        let result = decode_vhf_freq(data, SsmSign::Plus).unwrap();
        assert!((result.value - 121.50).abs() < 0.01);
    }

    #[test]
    fn test_bcd_invalid_digit() {
        let data = 0x0000Fu32;
        let layout = BcdDigitLayout::new(5).with_unit("test");
        let result = BcdDecoder.decode_raw(data, SsmSign::Plus, &layout);
        assert!(matches!(result, Err(BcdDecodeError::InvalidDigit { .. })));
    }

    #[test]
    fn test_bcd_octal_xpdr_7700() {
        let data = 0x7700u32;
        let result = decode_xpdr_code(data, SsmSign::Plus).unwrap();
        assert!(result.display.contains("7700"));
    }

    #[test]
    fn test_bcd_adf_355_khz() {
        let data = 0x0355u32;
        let layout = BcdDigitLayout::new(3)
            .with_decimal(0)
            .with_unit("kHz");
        let result = BcdDecoder.decode_raw(data, SsmSign::Plus, &layout).unwrap();
        assert!((result.value - 355.0).abs() < 0.1);
    }

    #[test]
    fn test_bcd_flap_15_deg() {
        let data = 0x15u32;
        let result = decode_flap_angle(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 15.0);
    }

    #[test]
    fn test_bcd_course_094() {
        let data = 0x094u32;
        let result = decode_course(data, SsmSign::Plus).unwrap();
        assert_eq!(result.value, 94.0);
    }

    #[test]
    fn test_bcd_sign_from_ssm() {
        let data = 0x1234u32;
        let layout = BcdDigitLayout::new(4)
            .with_unit("deg")
            .with_sign_from_ssm();
        let r1 = BcdDecoder.decode_raw(data, SsmSign::Plus, &layout).unwrap();
        assert!(r1.value > 0.0);
        let r2 = BcdDecoder.decode_raw(data, SsmSign::Minus, &layout).unwrap();
        assert!(r2.value < 0.0);
        assert_eq!(r1.value, -r2.value);
    }

    #[test]
    fn test_bcd_resolution() {
        let data = 0x025u32;
        let layout = BcdDigitLayout::new(2)
            .with_unit("ft")
            .with_resolution(0.5);
        let result = BcdDecoder.decode_raw(data, SsmSign::Plus, &layout).unwrap();
        assert_eq!(result.value, 12.5);
    }
}
