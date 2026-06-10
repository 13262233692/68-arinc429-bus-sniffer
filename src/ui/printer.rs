use colored::*;

use crate::core::types::PayloadFormat;
use crate::io::pipeline::{DecodedWord, DecodeResult, PipelineStats};
use crate::io::reader::ReaderStats;
use crate::core::dictionary::get_avionics_dictionary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Pretty,
    Compact,
    Json,
    RawHex,
    Bitwise,
    Table,
}

#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub mode: OutputMode,
    pub show_raw: bool,
    pub show_bits: bool,
    pub show_desc: bool,
    pub color: bool,
    pub line_limit: Option<usize>,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        DisplayOptions {
            mode: OutputMode::Pretty,
            show_raw: true,
            show_bits: false,
            show_desc: true,
            color: true,
            line_limit: None,
        }
    }
}

pub struct OutputPrinter {
    options: DisplayOptions,
    line_count: usize,
}

impl OutputPrinter {
    pub fn new(options: DisplayOptions) -> Self {
        OutputPrinter {
            options,
            line_count: 0,
        }
    }

    pub fn should_stop(&self) -> bool {
        if let Some(limit) = self.options.line_limit {
            self.line_count >= limit
        } else {
            false
        }
    }

    pub fn print_banner(&self) {
        let banner = r"
    _    ____ ___ _  _  _   _   ___ ___ __  __ _____ ____  _____ ___ ____  
   / \  |  _ \_ _| \/ || | | | |_ _|_ _|  \/  | ____|  _ \|  ___|_ _|  _ \ 
  / _ \ | |_) | || |\/ || |_| |  | | | || |\/| |  _| | |_) | |_   | || |_) |
 / ___ \|  _ <| || |  ||  _  |  | | | || |  | | |___|  _ <|  _|  | ||  _ < 
/_/   \_\_| \_\___|_|  ||_| |_| |___|___|_|  |_|_____|_| \_\_|   |___|_| \_\
        🛩  ARINC 429 AVIATION BUS SNIFFER  |  v0.1.0 [Geek Edition]
    ───────────────────────────────────────────────────────────────────";

        if self.options.color {
            println!("{}", banner.cyan().bold());
        } else {
            println!("{}", banner);
        }
        println!();
    }

    pub fn print_decoded(&mut self, dw: &DecodedWord) {
        if self.should_stop() {
            return;
        }
        self.line_count += 1;

        match self.options.mode {
            OutputMode::Pretty => self.print_pretty(dw),
            OutputMode::Compact => self.print_compact(dw),
            OutputMode::RawHex => self.print_raw_hex(dw),
            OutputMode::Bitwise => self.print_bitwise(dw),
            _ => self.print_pretty(dw),
        }
    }

    fn print_pretty(&self, dw: &DecodedWord) {
        let w = &dw.word;
        let idx = self.line_count;

        let idx_str = if self.options.color {
            format!("[{:>6}]", idx).dimmed().to_string()
        } else {
            format!("[{:>6}]", idx)
        };

        let parity_marker = if w.parity_valid() {
            if self.options.color { "✓".green().to_string() } else { "✓".to_string() }
        } else {
            if self.options.color { "✗".red().bold().to_string() } else { "✗".to_string() }
        };

        let label_oct = w.label_octal_str();
        let label_part = if let Some(def) = dw.label_def {
            let fmt_tag = match def.format {
                PayloadFormat::Bnr => if self.options.color { "[BNR]".blue().to_string() } else { "[BNR]".to_string() },
                PayloadFormat::Bcd => if self.options.color { "[BCD]".magenta().to_string() } else { "[BCD]".to_string() },
                PayloadFormat::Discrete => if self.options.color { "[DSC]".yellow().to_string() } else { "[DSC]".to_string() },
                _ => if self.options.color { "[SYS]".dimmed().to_string() } else { "[SYS]".to_string() },
            };
            if self.options.color {
                format!("LBL={:<3} {} {} {}",
                    label_oct.yellow().bold(),
                    fmt_tag,
                    def.param_name.bold().white(),
                    def.equipment.dimmed())
            } else {
                format!("LBL={:<3} {} {} {}", label_oct, fmt_tag, def.param_name, def.equipment)
            }
        } else {
            if self.options.color {
                format!("LBL={:<3} {}", label_oct.red(), "UNKNOWN".red().italic())
            } else {
                format!("LBL={:<3} UNKNOWN", label_oct)
            }
        };

        let sdi_str = format!("SDI={}", w.sdi());
        let ssm_str = format!("SSM={}", w.ssm().as_str());

        let eng_str = match &dw.decode_result {
            DecodeResult::Success => {
                if let Some(ev) = &dw.eng_value {
                    if self.options.color {
                        format!("▶ {}", ev.display.green().bold())
                    } else {
                        format!("▶ {}", ev.display)
                    }
                } else {
                    if self.options.color { "⚠ No value".yellow().to_string() } else { "⚠ No value".to_string() }
                }
            }
            DecodeResult::UnknownLabel => {
                if self.options.color { "⊘ UNKNOWN LABEL".dimmed().to_string() } else { "⊘ UNKNOWN LABEL".to_string() }
            }
            DecodeResult::BnrError(e) => {
                if self.options.color { format!("✗ BNR ERR: {}", e).red().to_string() } else { format!("✗ BNR ERR: {}", e) }
            }
            DecodeResult::BcdError(e) => {
                if self.options.color { format!("✗ BCD ERR: {}", e).red().to_string() } else { format!("✗ BCD ERR: {}", e) }
            }
            DecodeResult::FormatNotSupported(f) => {
                if self.options.color { format!("⚠ Format {:?} not decoded", f).yellow().to_string() } else { format!("⚠ Format {:?} not decoded", f) }
            }
            DecodeResult::SpareData => {
                if self.options.color { "⚠ SSM=SPARE (no data)".dimmed().to_string() } else { "⚠ SSM=SPARE (no data)".to_string() }
            }
        };

        println!("{} {} {}  {} {}  P[{}]  {}",
            idx_str, label_part, sdi_str, ssm_str, parity_marker,
            if w.parity_bit() { 1 } else { 0 },
            eng_str);

        if self.options.show_raw {
            if self.options.color {
                println!("         ↳ HEX: 0x{}  DATA: 0x{:05X}",
                    w.to_hex_str().cyan(), w.data());
            } else {
                println!("         ↳ HEX: 0x{}  DATA: 0x{:05X}", w.to_hex_str(), w.data());
            }
        }

        if self.options.show_bits {
            if self.options.color {
                println!("         ↳ BIN: {}", w.to_binary_str().dimmed());
            } else {
                println!("         ↳ BIN: {}", w.to_binary_str());
            }
        }

        if self.options.show_desc {
            if let Some(def) = dw.label_def {
                if self.options.color {
                    println!("         ↳ 💡 {}", def.description.dimmed().italic());
                } else {
                    println!("         ↳ 💡 {}", def.description);
                }
            }
        }
    }

    fn print_compact(&self, dw: &DecodedWord) {
        let w = &dw.word;
        let label_oct = w.label_octal_str();

        let param = match &dw.decode_result {
            DecodeResult::Success => dw.eng_value.as_ref().map(|e| e.display.clone()).unwrap_or_else(|| "OK".to_string()),
            DecodeResult::UnknownLabel => "?".to_string(),
            DecodeResult::BnrError(e) => format!("ERR:{}", e),
            DecodeResult::BcdError(e) => format!("ERR:{}", e),
            _ => "N/A".to_string(),
        };

        let eq = dw.label_def.as_ref().map(|d| d.equipment.clone()).unwrap_or_else(|| "???".to_string());

        println!("{:>6} | {:>3} | {:1} | {:>6} | {:>8X} | {:>2} | {:<10} | {}",
            self.line_count,
            label_oct,
            w.sdi(),
            w.ssm().as_str(),
            w.data(),
            if w.parity_valid() { "OK" } else { "PE" },
            eq,
            param);
    }

    fn print_raw_hex(&self, dw: &DecodedWord) {
        println!("{}", dw.word.to_hex_str());
    }

    fn print_bitwise(&self, dw: &DecodedWord) {
        let w = &dw.word;
        println!("WORD #{:06} = 0x{} = 0b{}",
            self.line_count,
            w.to_hex_str(),
            w.to_binary_str());
        println!("  ├─ Label:  bits 1-8  = 0x{:02X} (octal {})",
            w.label(), w.label_octal_str());
        println!("  ├─ SDI:    bits 9-10 = 0b{:02b} ({})",
            w.sdi(), w.sdi());
        println!("  ├─ DATA:   bits 11-29 = 0x{:05X} (unsigned {})",
            w.data(), w.data());
        println!("  ├─ SSM:    bits 30-31 = 0b{:02b} ({})",
            w.ssm().to_bits(), w.ssm().as_str());
        println!("  └─ PARITY: bit 32     = {} (valid: {})",
            if w.parity_bit() { 1 } else { 0 }, w.parity_valid());
    }

    pub fn print_reader_stats(&self, stats: &ReaderStats) {
        println!();
        if self.options.color {
            println!("{}", "📊 READER STATISTICS".bold().underline());
            println!("{}", stats.to_string().cyan());
        } else {
            println!("📊 READER STATISTICS");
            println!("{}", stats);
        }
    }

    pub fn print_pipeline_stats(&self, stats: &PipelineStats) {
        println!();
        if self.options.color {
            println!("{}", "📈 DECODE PIPELINE STATISTICS".bold().underline());
            println!("{}", stats.summary().green());
        } else {
            println!("📈 DECODE PIPELINE STATISTICS");
            println!("{}", stats.summary());
        }
    }

    pub fn print_dictionary_size(&self) {
        let dict = get_avionics_dictionary();
        if self.options.color {
            println!("  📚 Avionics Dictionary: {} label definitions loaded",
                dict.len().to_string().bold().yellow());
        } else {
            println!("  📚 Avionics Dictionary: {} label definitions loaded", dict.len());
        }
    }

    pub fn print_decoded_all(&mut self, words: &[DecodedWord]) {
        for dw in words {
            if self.should_stop() {
                break;
            }
            self.print_decoded(dw);
        }
    }
}
