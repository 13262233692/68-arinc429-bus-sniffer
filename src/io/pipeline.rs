use std::collections::HashMap;

use crate::core::word::ArincWord;
use crate::core::types::{PayloadFormat, EngineeringValue};
use crate::core::dictionary::{lookup_label, LabelDefinition};
use crate::decode::{BnrDecoder, bnr::BnrDecodeError};
use crate::decode::{BcdDecoder, bcd::BcdDecodeError};
use crate::decode::discrete::*;

#[derive(Debug, Clone)]
pub struct DecodedWord {
    pub word: ArincWord,
    pub label_def: Option<&'static LabelDefinition>,
    pub eng_value: Option<EngineeringValue>,
    pub decode_result: DecodeResult,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodeResult {
    Success,
    UnknownLabel,
    BnrError(BnrDecodeError),
    BcdError(BcdDecodeError),
    FormatNotSupported(PayloadFormat),
    SpareData,
}

pub struct WordPipeline {
    bnr: BnrDecoder,
    bcd: BcdDecoder,
    skip_parity_invalid: bool,
    only_known_labels: bool,
}

impl WordPipeline {
    pub fn new() -> Self {
        WordPipeline {
            bnr: BnrDecoder::new(),
            bcd: BcdDecoder::new(),
            skip_parity_invalid: false,
            only_known_labels: false,
        }
    }

    pub fn skip_parity_invalid(mut self, skip: bool) -> Self {
        self.skip_parity_invalid = skip;
        self
    }

    pub fn only_known_labels(mut self, only: bool) -> Self {
        self.only_known_labels = only;
        self
    }

    pub fn process_word(&self, word: ArincWord) -> Option<DecodedWord> {
        if self.skip_parity_invalid && !word.parity_valid() {
            return None;
        }

        let label_octal = word.label_octal();
        let label_def = lookup_label(label_octal);

        if self.only_known_labels && label_def.is_none() {
            return None;
        }

        let (eng_value, decode_result) = match label_def {
            Some(def) => self.decode_known(&word, def),
            None => (None, DecodeResult::UnknownLabel),
        };

        Some(DecodedWord {
            word,
            label_def,
            eng_value,
            decode_result,
        })
    }

    fn decode_known(
        &self,
        word: &ArincWord,
        def: &LabelDefinition
    ) -> (Option<EngineeringValue>, DecodeResult) {
        match def.format {
            PayloadFormat::Bnr => {
                match self.bnr.decode_with_label(word.data(), word.ssm(), def) {
                    Ok(val) => (Some(val), DecodeResult::Success),
                    Err(e) => (None, DecodeResult::BnrError(e)),
                }
            }
            PayloadFormat::Bcd => {
                match self.bcd.decode_with_label(word.data(), word.ssm(), def) {
                    Ok(val) => (Some(val), DecodeResult::Success),
                    Err(e) => (None, DecodeResult::BcdError(e)),
                }
            }
            PayloadFormat::Discrete => {
                let val = match def.label_octal {
                    0o102 => decode_gear_discrete(word.data()),
                    0o103 => decode_door_status(word.data()),
                    0o104 => decode_anti_ice(word.data()),
                    0o105 => decode_autopilot_modes(word.data()),
                    _ => {
                        let generic_def = DiscreteWordDef::new(
                            &def.param_name,
                            (0..19u8).map(|i| DiscreteBit::new(i, &format!("BIT{}", i)))
                                .collect()
                        );
                        decode_discrete(word.data(), &generic_def)
                    }
                };
                (Some(val), DecodeResult::Success)
            }
            PayloadFormat::Maintenance | PayloadFormat::Ack => {
                let hex = EngineeringValue {
                    value: word.data() as f64,
                    unit: "hex".to_string(),
                    display: format!("{} 0x{:05X}", def.param_name, word.data()),
                };
                (Some(hex), DecodeResult::Success)
            }
            other => (None, DecodeResult::FormatNotSupported(other)),
        }
    }

    pub fn process_all(
        &self,
        words: Vec<ArincWord>
    ) -> (Vec<DecodedWord>, PipelineStats) {
        let mut stats = PipelineStats::new();
        let total = words.len();
        stats.total_words = total;

        let mut by_label: HashMap<u16, usize> = HashMap::new();
        let mut by_format: HashMap<&'static str, usize> = HashMap::new();
        let mut errors_by_type: HashMap<&'static str, usize> = HashMap::new();

        let decoded: Vec<DecodedWord> = words
            .into_iter()
            .filter_map(|w| {
                let result = self.process_word(w);
                if let Some(dw) = &result {
                    let label_octal = dw.word.label_octal();
                    *by_label.entry(label_octal).or_insert(0) += 1;

                    match &dw.decode_result {
                        DecodeResult::Success => {
                            stats.decoded_successfully += 1;
                            if let Some(def) = dw.label_def {
                                let fmt_str = match def.format {
                                    PayloadFormat::Bnr => "BNR",
                                    PayloadFormat::Bcd => "BCD",
                                    PayloadFormat::Discrete => "Discrete",
                                    PayloadFormat::Maintenance => "Maintenance",
                                    PayloadFormat::Ack => "Ack",
                                    PayloadFormat::Unknown => "Unknown",
                                };
                                *by_format.entry(fmt_str).or_insert(0) += 1;
                            }
                        }
                        DecodeResult::UnknownLabel => {
                            stats.unknown_labels += 1;
                            *errors_by_type.entry("UnknownLabel").or_insert(0) += 1;
                        }
                        DecodeResult::BnrError(e) => {
                            stats.decode_errors += 1;
                            let tag = match e {
                                BnrDecodeError::OutOfRange { .. } => "BNR:OutOfRange",
                                BnrDecodeError::InvalidBits { .. } => "BNR:InvalidBits",
                                BnrDecodeError::SsmSpare => "BNR:SsmSpare",
                            };
                            *errors_by_type.entry(tag).or_insert(0) += 1;
                        }
                        DecodeResult::BcdError(e) => {
                            stats.decode_errors += 1;
                            let tag = match e {
                                BcdDecodeError::InvalidDigit { .. } => "BCD:InvalidDigit",
                                BcdDecodeError::TooManyDigits { .. } => "BCD:TooManyDigits",
                            };
                            *errors_by_type.entry(tag).or_insert(0) += 1;
                        }
                        DecodeResult::FormatNotSupported(_) => {
                            stats.decode_errors += 1;
                            *errors_by_type.entry("FormatNotSupported").or_insert(0) += 1;
                        }
                        DecodeResult::SpareData => {
                            stats.spare_data += 1;
                        }
                    }
                } else {
                    stats.skipped_parity += 1;
                }
                result
            })
            .collect();

        let sorted_labels = {
            let mut v: Vec<_> = by_label.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            v.truncate(15);
            v
        };

        stats.top_labels = sorted_labels;
        stats.format_breakdown = by_format;
        stats.error_breakdown = errors_by_type;

        (decoded, stats)
    }
}

impl Default for WordPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub total_words: usize,
    pub processed: usize,
    pub decoded_successfully: usize,
    pub unknown_labels: usize,
    pub decode_errors: usize,
    pub skipped_parity: usize,
    pub spare_data: usize,
    pub top_labels: Vec<(u16, usize)>,
    pub format_breakdown: HashMap<&'static str, usize>,
    pub error_breakdown: HashMap<&'static str, usize>,
}

impl PipelineStats {
    pub fn new() -> Self {
        PipelineStats {
            total_words: 0,
            processed: 0,
            decoded_successfully: 0,
            unknown_labels: 0,
            decode_errors: 0,
            skipped_parity: 0,
            spare_data: 0,
            top_labels: Vec::new(),
            format_breakdown: HashMap::new(),
            error_breakdown: HashMap::new(),
        }
    }

    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str("  ───── Pipeline Summary ─────\n");
        s.push_str(&format!("  Total Words:     {}\n", self.total_words));
        s.push_str(&format!("  Decoded OK:      {} ({:.1}%)\n",
            self.decoded_successfully,
            self.pct(self.decoded_successfully)));
        s.push_str(&format!("  Unknown Labels:  {} ({:.1}%)\n",
            self.unknown_labels,
            self.pct(self.unknown_labels)));
        s.push_str(&format!("  Decode Errors:   {} ({:.1}%)\n",
            self.decode_errors,
            self.pct(self.decode_errors)));
        s.push_str(&format!("  Parity Skipped:  {}\n", self.skipped_parity));

        if !self.format_breakdown.is_empty() {
            s.push_str("\n  Format Breakdown:\n");
            let mut items: Vec<_> = self.format_breakdown.iter().collect();
            items.sort_by(|a, b| b.1.cmp(a.1));
            for (fmt, cnt) in items {
                s.push_str(&format!("    {:>12} : {}\n", fmt, cnt));
            }
        }

        if !self.top_labels.is_empty() {
            s.push_str("\n  Top 15 Labels by Frequency:\n");
            for (label_octal, count) in &self.top_labels {
                s.push_str(&format!("    LBL {:03o} (0x{:02X}) : {}\n",
                    label_octal, *label_octal as u8, count));
            }
        }

        if !self.error_breakdown.is_empty() {
            s.push_str("\n  Error Breakdown:\n");
            let mut items: Vec<_> = self.error_breakdown.iter().collect();
            items.sort_by(|a, b| b.1.cmp(a.1));
            for (tag, cnt) in items {
                s.push_str(&format!("    {:>20} : {}\n", tag, cnt));
            }
        }

        s
    }

    fn pct(&self, part: usize) -> f64 {
        if self.total_words == 0 { 0.0 } else { part as f64 / self.total_words as f64 * 100.0 }
    }
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::word::WordEndianness;

    fn make_word(raw: u32) -> ArincWord {
        ArincWord::from_u32(raw, WordEndianness::Standard)
    }

    #[test]
    fn test_known_bnr_label() {
        let pipeline = WordPipeline::new();

        let data = 400u32;
        let raw = (data << 10) | 0o001;
        let word = make_word(raw | 0x80000000);

        let decoded = pipeline.process_word(word).unwrap();
        assert!(decoded.label_def.is_some());
        assert_eq!(decoded.label_def.unwrap().param_name, "RADIO_ALTITUDE");
        assert_eq!(decoded.decode_result, DecodeResult::Success);
        assert!(decoded.eng_value.is_some());
    }

    #[test]
    fn test_unknown_label() {
        let pipeline = WordPipeline::new();
        let word = make_word(0o177);

        let decoded = pipeline.process_word(word).unwrap();
        assert!(decoded.label_def.is_none());
        assert_eq!(decoded.decode_result, DecodeResult::UnknownLabel);
    }

    #[test]
    fn test_skip_parity() {
        let pipeline = WordPipeline::new().skip_parity_invalid(true);
        let bad_parity_word = make_word(0x00000003);
        assert!(pipeline.process_word(bad_parity_word).is_none());
    }

    #[test]
    fn test_only_known_labels() {
        let pipeline = WordPipeline::new().only_known_labels(true);
        let unknown = make_word(0o177 | 0x80000000);
        assert!(pipeline.process_word(unknown).is_none());
    }

    #[test]
    fn test_batch_processing_stats() {
        let pipeline = WordPipeline::new();
        let mut words: Vec<ArincWord> = Vec::new();

        for _ in 0..10 {
            words.push(make_word(0x80000001u32 | 0o001));
        }
        for _ in 0..5 {
            words.push(make_word(0x80000000u32 | 0o002));
        }
        for _ in 0..3 {
            words.push(make_word(0x80000000u32 | 0o177));
        }

        let (decoded, stats) = pipeline.process_all(words);
        assert_eq!(decoded.len(), 18);
        assert_eq!(stats.total_words, 18);
        assert!(stats.decoded_successfully >= 15);
        assert_eq!(stats.unknown_labels, 3);
    }

    #[test]
    fn test_discrete_gear_decoding() {
        let pipeline = WordPipeline::new();

        let data = (1 << 0) | (1 << 3) | (1 << 6);
        let raw = (data << 10) | 0o102 | 0x80000000;
        let word = make_word(raw);

        let decoded = pipeline.process_word(word).unwrap();
        assert!(decoded.eng_value.is_some());
        let disp = decoded.eng_value.unwrap().display;
        assert!(disp.contains("NOSE_GEAR_DOWN"));
        assert!(disp.contains("LEFT_GEAR_DOWN"));
    }
}
