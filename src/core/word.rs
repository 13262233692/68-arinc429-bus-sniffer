use std::fmt;
use byteorder::{ByteOrder, LittleEndian, BigEndian};

use crate::core::types::SsmSign;

pub const LABEL_MASK: u32 = 0x000000FF;
pub const LABEL_SHIFT: u32 = 0;
pub const SDI_MASK: u32 = 0x00000300;
pub const SDI_SHIFT: u32 = 8;
pub const DATA_MASK: u32 = 0x1FFFFC00;
pub const DATA_SHIFT: u32 = 10;
pub const SSM_MASK: u32 = 0x60000000;
pub const SSM_SHIFT: u32 = 29;
pub const PARITY_MASK: u32 = 0x80000000;
pub const PARITY_SHIFT: u32 = 31;

pub const PAYLOAD_BITS: u32 = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordEndianness {
    Standard,
    Reversed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityType {
    Odd,
    Even,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArincWord {
    raw: u32,
    label: u8,
    sdi: u8,
    data: u32,
    ssm: SsmSign,
    parity_bit: bool,
    parity_valid: bool,
    endianness: WordEndianness,
}

impl ArincWord {
    pub fn from_bytes_le(bytes: &[u8; 4]) -> Self {
        let raw = LittleEndian::read_u32(bytes);
        Self::from_u32(raw, WordEndianness::Standard)
    }

    pub fn from_bytes_be(bytes: &[u8; 4]) -> Self {
        let raw = BigEndian::read_u32(bytes);
        Self::from_u32(raw, WordEndianness::Reversed)
    }

    pub fn from_u32(raw: u32, endianness: WordEndianness) -> Self {
        let label = ((raw & LABEL_MASK) >> LABEL_SHIFT) as u8;
        let sdi = ((raw & SDI_MASK) >> SDI_SHIFT) as u8;
        let data = (raw & DATA_MASK) >> DATA_SHIFT;
        let ssm_bits = ((raw & SSM_MASK) >> SSM_SHIFT) as u8;
        let ssm = SsmSign::from_bits(ssm_bits);
        let parity_bit = (raw & PARITY_MASK) != 0;

        let ones_count = raw.count_ones();
        let parity_valid = (ones_count % 2) == 1;

        ArincWord {
            raw,
            label,
            sdi,
            data,
            ssm,
            parity_bit,
            parity_valid,
            endianness,
        }
    }

    pub fn raw(&self) -> u32 {
        self.raw
    }

    pub fn label(&self) -> u8 {
        self.label
    }

    pub fn label_octal(&self) -> u16 {
        self.label as u16
    }

    pub fn label_octal_str(&self) -> String {
        format!("{:03o}", self.label)
    }

    pub fn sdi(&self) -> u8 {
        self.sdi
    }

    pub fn data(&self) -> u32 {
        self.data
    }

    pub fn data_signed(&self) -> i32 {
        let sign = match self.ssm {
            SsmSign::Plus | SsmSign::No => 1,
            SsmSign::Minus => -1,
            SsmSign::Spare => 1,
        };
        (self.data as i32) * sign
    }

    pub fn ssm(&self) -> SsmSign {
        self.ssm
    }

    pub fn parity_bit(&self) -> bool {
        self.parity_bit
    }

    pub fn parity_valid(&self) -> bool {
        self.parity_valid
    }

    pub fn endianness(&self) -> WordEndianness {
        self.endianness
    }

    pub fn to_hex_str(&self) -> String {
        format!("{:08X}", self.raw)
    }

    pub fn to_binary_str(&self) -> String {
        format!(
            "{:08b}_{:02b}_{:019b}_{:02b}_{:1b}",
            self.label,
            self.sdi,
            self.data,
            self.ssm.to_bits(),
            if self.parity_bit { 1 } else { 0 }
        )
    }

    pub fn validate_parity(&self) -> bool {
        self.parity_valid
    }

    pub fn compute_parity(&self) -> bool {
        (self.raw.count_ones() % 2) == 1
    }
}

impl fmt::Display for ArincWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parity_mark = if self.parity_valid { "✓" } else { "✗" };
        write!(
            f,
            "LABEL={} (0x{:02X})  SDI={}  DATA=0x{:05X}  SSM={}  PARITY={} [{}]  RAW=0x{:08X}",
            self.label_octal_str(),
            self.label,
            self.sdi,
            self.data,
            self.ssm.as_str(),
            if self.parity_bit { "1" } else { "0" },
            parity_mark,
            self.raw
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let raw: u32 = 0b1_00_0000000000000000001_01_11111111;
        let word = ArincWord::from_u32(raw, WordEndianness::Standard);

        assert_eq!(word.label(), 0xFF);
        assert_eq!(word.label_octal(), 0o377);
        assert_eq!(word.label_octal_str(), "377");
        assert_eq!(word.sdi(), 0b01);
        assert_eq!(word.data(), 0b0000000000000000001);
        assert_eq!(word.ssm(), SsmSign::Plus);
        assert_eq!(word.parity_bit(), true);
    }

    #[test]
    fn test_ssm_variants() {
        let w1 = ArincWord::from_u32(0b00 << 29 | 0x00000001, WordEndianness::Standard);
        assert_eq!(w1.ssm(), SsmSign::Plus);

        let w2 = ArincWord::from_u32(0b01 << 29 | 0x00000001, WordEndianness::Standard);
        assert_eq!(w2.ssm(), SsmSign::Minus);

        let w3 = ArincWord::from_u32(0b10 << 29 | 0x00000001, WordEndianness::Standard);
        assert_eq!(w3.ssm(), SsmSign::No);

        let w4 = ArincWord::from_u32(0b11 << 29 | 0x00000001, WordEndianness::Standard);
        assert_eq!(w4.ssm(), SsmSign::Spare);
    }

    #[test]
    fn test_label_octal_conversion() {
        let cases = vec![
            (0o001 as u8, "001"),
            (0o007 as u8, "007"),
            (0o010 as u8, "010"),
            (0o377 as u8, "377"),
            (0o000 as u8, "000"),
        ];

        for (label, expected_str) in cases {
            let word = ArincWord::from_u32(label as u32, WordEndianness::Standard);
            assert_eq!(
                word.label_octal_str(),
                expected_str,
                "Label 0x{:02X} expected {} got {}",
                label,
                expected_str,
                word.label_octal_str()
            );
        }
    }

    #[test]
    fn test_odd_parity() {
        let raw = 0x00000001;
        let word = ArincWord::from_u32(raw, WordEndianness::Standard);
        assert!(word.validate_parity(), "Odd number of ones should be valid");

        let raw2 = 0x00000003;
        let word2 = ArincWord::from_u32(raw2, WordEndianness::Standard);
        assert!(!word2.validate_parity(), "Even number of ones should be invalid");
    }

    #[test]
    fn test_data_signed() {
        let w_plus = ArincWord::from_u32(
            0b00_0000000000000001010_00_00000000,
            WordEndianness::Standard,
        );
        assert_eq!(w_plus.data_signed(), 10);

        let w_minus = ArincWord::from_u32(
            0b01_0000000000000001010_00_00000000,
            WordEndianness::Standard,
        );
        assert_eq!(w_minus.data_signed(), -10);
    }

    #[test]
    fn test_from_bytes() {
        let bytes_le: [u8; 4] = [0xFF, 0x01, 0x00, 0x00];
        let word_le = ArincWord::from_bytes_le(&bytes_le);
        assert_eq!(word_le.label(), 0xFF);
        assert_eq!(word_le.sdi(), 0b01);

        let bytes_be: [u8; 4] = [0x00, 0x00, 0x01, 0xFF];
        let word_be = ArincWord::from_bytes_be(&bytes_be);
        assert_eq!(word_be.raw(), 0x000001FF);
    }
}
