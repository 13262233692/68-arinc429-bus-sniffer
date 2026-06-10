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

#[inline]
pub fn reverse_bits_u8(b: u8) -> u8 {
    let mut v = b;
    v = ((v >> 1) & 0x55) | ((v & 0x55) << 1);
    v = ((v >> 2) & 0x33) | ((v & 0x33) << 2);
    v = ((v >> 4) & 0x0F) | ((v & 0x0F) << 4);
    v
}

#[inline]
pub fn compute_odd_parity_xor_32(raw: u32) -> bool {
    let mut xor_acc: u32 = 0;
    for i in 0..32u32 {
        xor_acc ^= (raw >> i) & 1;
    }
    xor_acc == 1
}

#[inline]
pub fn generate_odd_parity_bit(raw_without_parity: u32) -> u32 {
    let mut xor_acc: u32 = 0;
    for i in 0..31u32 {
        xor_acc ^= (raw_without_parity >> i) & 1;
    }
    if xor_acc == 1 {
        0
    } else {
        1u32 << PARITY_SHIFT
    }
}

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
    raw_label: u8,
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
        let raw_label = ((raw & LABEL_MASK) >> LABEL_SHIFT) as u8;
        let label = reverse_bits_u8(raw_label);
        let sdi = ((raw & SDI_MASK) >> SDI_SHIFT) as u8;
        let data = (raw & DATA_MASK) >> DATA_SHIFT;
        let ssm_bits = ((raw & SSM_MASK) >> SSM_SHIFT) as u8;
        let ssm = SsmSign::from_bits(ssm_bits);
        let parity_bit = (raw & PARITY_MASK) != 0;

        let parity_valid = compute_odd_parity_xor_32(raw);

        ArincWord {
            raw,
            raw_label,
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

    pub fn raw_label(&self) -> u8 {
        self.raw_label
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
        compute_odd_parity_xor_32(self.raw)
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

    fn build_word_with_parity(
        logical_label: u8,
        sdi: u8,
        data: u32,
        ssm_bits: u8,
    ) -> u32 {
        let raw_label = reverse_bits_u8(logical_label);
        let mut raw: u32 = 0;
        raw |= (raw_label as u32) << LABEL_SHIFT;
        raw |= ((sdi & 0b11) as u32) << SDI_SHIFT;
        raw |= (data << DATA_SHIFT) & DATA_MASK;
        raw |= ((ssm_bits & 0b11) as u32) << SSM_SHIFT;
        let parity = generate_odd_parity_bit(raw);
        raw | parity
    }

    #[test]
    fn test_reverse_bits_u8_identity() {
        assert_eq!(reverse_bits_u8(0x00), 0x00);
        assert_eq!(reverse_bits_u8(0xFF), 0xFF);
        assert_eq!(reverse_bits_u8(0xAA), 0x55);
        assert_eq!(reverse_bits_u8(0x55), 0xAA);
    }

    #[test]
    fn test_reverse_bits_u8_standard() {
        assert_eq!(reverse_bits_u8(0b00000001), 0b10000000);
        assert_eq!(reverse_bits_u8(0b10000000), 0b00000001);
        assert_eq!(reverse_bits_u8(0b00000010), 0b01000000);
        assert_eq!(reverse_bits_u8(0b00000100), 0b00100000);
        assert_eq!(reverse_bits_u8(0b01010101), 0b10101010);
        assert_eq!(reverse_bits_u8(0b11000001), 0b10000011);
    }

    #[test]
    fn test_generate_parity_even_data() {
        let raw = 0u32;
        let p = generate_odd_parity_bit(raw);
        assert_ne!(p, 0, "all-zero 31 bits should have parity=1 to make total odd");
        let full = raw | p;
        assert!(compute_odd_parity_xor_32(full), "full 32-bit should be odd parity");
    }

    #[test]
    fn test_generate_parity_odd_data() {
        let raw = 1u32;
        let p = generate_odd_parity_bit(raw);
        assert_eq!(p, 0, "31-bit data with 1 one should have parity=0");
        let full = raw | p;
        assert!(compute_odd_parity_xor_32(full), "full 32-bit should be odd parity");
    }

    #[test]
    fn test_xor_parity_32bit_coverage() {
        for expected in [true, false] {
            for bit in 0..32u32 {
                let mut raw = 1u32 << bit;
                if !expected {
                    raw |= 1u32 << ((bit + 1) % 32);
                }
                let result = compute_odd_parity_xor_32(raw);
                assert_eq!(result, expected,
                    "bit pattern 0x{:08X} should compute to {}", raw, expected);
            }
        }
    }

    #[test]
    fn test_label_bit_reversal_roundtrip() {
        for logical in 0u16..=0o377u16 {
            let logical_u8 = logical as u8;
            let raw = reverse_bits_u8(logical_u8);
            let recovered = reverse_bits_u8(raw);
            assert_eq!(recovered, logical_u8,
                "Label 0o{:03o} failed roundtrip: raw=0x{:02X}", logical, raw);
        }
    }

    #[test]
    fn test_basic_parsing() {
        let logical_label = 0o377u8;
        let sdi = 0b01u8;
        let data = 0b0000000000000000001u32;
        let ssm = 0b00u8;
        let raw = build_word_with_parity(logical_label, sdi, data, ssm);
        let word = ArincWord::from_u32(raw, WordEndianness::Standard);

        assert_eq!(word.label(), 0xFF);
        assert_eq!(word.label_octal(), 0o377);
        assert_eq!(word.label_octal_str(), "377");
        assert_eq!(word.sdi(), 0b01);
        assert_eq!(word.data(), 0b0000000000000000001);
        assert_eq!(word.ssm(), SsmSign::Plus);
        assert!(word.parity_valid(), "parity should be valid for built word");
    }

    #[test]
    fn test_label_octal_conversion_with_reversal() {
        let cases = vec![
            (0o001u8, 0x80u8, "001"),
            (0o007u8, 0xE0u8, "007"),
            (0o010u8, 0x10u8, "010"),
            (0o377u8, 0xFFu8, "377"),
            (0o000u8, 0x00u8, "000"),
            (0o002u8, 0x40u8, "002"),
            (0o040u8, 0x04u8, "040"),
            (0o100u8, 0x02u8, "100"),
        ];

        for (logical_label, raw_label_byte, expected_str) in cases {
            assert_eq!(reverse_bits_u8(logical_label), raw_label_byte,
                "label 0o{:03o} reverse mismatch", logical_label);
            let raw = build_word_with_parity(logical_label, 0, 0, 0b00);
            let word = ArincWord::from_u32(raw, WordEndianness::Standard);
            assert_eq!(word.raw_label(), raw_label_byte, "raw label mismatch");
            assert_eq!(word.label(), logical_label, "logical label mismatch");
            assert_eq!(
                word.label_octal_str(),
                expected_str,
                "Label 0o{:03o} expected {} got {}",
                logical_label,
                expected_str,
                word.label_octal_str()
            );
        }
    }

    #[test]
    fn test_ssm_variants_with_valid_parity() {
        let labels: Vec<(u8, SsmSign)> = vec![
            (0b00, SsmSign::Plus),
            (0b01, SsmSign::Minus),
            (0b10, SsmSign::No),
            (0b11, SsmSign::Spare),
        ];
        for (ssm_bits, expected) in labels {
            let raw = build_word_with_parity(0o001u8, 0, 0, ssm_bits);
            let w = ArincWord::from_u32(raw, WordEndianness::Standard);
            assert_eq!(w.ssm(), expected, "SSM mismatch for bits={:02b}", ssm_bits);
            assert!(w.parity_valid(), "parity should be valid for SSM={:02b}", ssm_bits);
        }
    }

    #[test]
    fn test_odd_parity_known_vectors() {
        let cases = vec![
            (0x00000001u32, true,  "single bit 0 -> odd -> valid"),
            (0x00000003u32, false, "bits 0+1 set -> 2 ones -> invalid"),
            (0x80000000u32, true,  "parity bit alone -> 1 one -> valid odd"),
            (0x80000001u32, false, "parity+bit0 -> 2 ones -> even -> invalid"),
        ];
        for (raw, expected, desc) in cases {
            let word = ArincWord::from_u32(raw, WordEndianness::Standard);
            assert_eq!(word.validate_parity(), expected,
                "raw=0x{:08X} failed: {}", raw, desc);
        }
    }

    #[test]
    fn test_manual_parity_valid_frame_with_even_ones() {
        let logical_label = 0o001u8;
        let raw_label = reverse_bits_u8(logical_label);
        let mut raw = 0u32;
        raw |= (raw_label as u32) << LABEL_SHIFT;
        let data_bits = 0b11u32;
        raw |= (data_bits << DATA_SHIFT) & DATA_MASK;
        let parity = generate_odd_parity_bit(raw);
        let full = raw | parity;
        let word = ArincWord::from_u32(full, WordEndianness::Standard);
        assert!(word.parity_valid(),
            "even data ones should still be valid when parity bit correct");
        assert_eq!(word.label(), logical_label);
        assert_eq!(word.data(), data_bits);
    }

    #[test]
    fn test_data_signed_with_valid_parity() {
        let raw_plus = build_word_with_parity(0o000u8, 0, 0b1010, 0b00);
        let w_plus = ArincWord::from_u32(raw_plus, WordEndianness::Standard);
        assert!(w_plus.parity_valid());
        assert_eq!(w_plus.data_signed(), 10);

        let raw_minus = build_word_with_parity(0o000u8, 0, 0b1010, 0b01);
        let w_minus = ArincWord::from_u32(raw_minus, WordEndianness::Standard);
        assert!(w_minus.parity_valid());
        assert_eq!(w_minus.data_signed(), -10);
    }

    #[test]
    fn test_from_bytes_with_label_reversal() {
        let logical_label = 0o001u8;
        let raw_label = reverse_bits_u8(logical_label);
        let sdi = 0b01u8;
        let mut raw = 0u32;
        raw |= (raw_label as u32) << LABEL_SHIFT;
        raw |= (sdi as u32) << SDI_SHIFT;
        let parity = generate_odd_parity_bit(raw);
        let full = raw | parity;

        let bytes_le: [u8; 4] = full.to_le_bytes();
        let word_le = ArincWord::from_bytes_le(&bytes_le);
        assert_eq!(word_le.label(), logical_label);
        assert_eq!(word_le.raw_label(), raw_label);
        assert_eq!(word_le.sdi(), sdi);
        assert!(word_le.parity_valid());

        let bytes_be: [u8; 4] = full.to_be_bytes();
        let word_be = ArincWord::from_bytes_be(&bytes_be);
        assert_eq!(word_be.raw(), full);
    }
}
