use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::core::word::{ArincWord, WordEndianness};
use crate::timing::{Timestamp, global_precise_clock};
use crate::TimedWord;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid file size: {0} bytes (must be multiple of 4)")]
    InvalidFileSize(usize),
    #[error("Read only {read} of {expected} bytes at word {word_idx}")]
    ShortRead { read: usize, expected: usize, word_idx: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpFormat {
    RawBinary,
    HexText,
    PcapLe,
    PcapBe,
}

pub struct ArincDumpReader {
    path: PathBuf,
    format: DumpFormat,
    endianness: WordEndianness,
}

impl ArincDumpReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ArincDumpReader {
            path: path.as_ref().to_path_buf(),
            format: DumpFormat::RawBinary,
            endianness: WordEndianness::Standard,
        }
    }

    pub fn with_format(mut self, format: DumpFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_endianness(mut self, endianness: WordEndianness) -> Self {
        self.endianness = endianness;
        self
    }

    pub fn read_all(&self) -> Result<(Vec<ArincWord>, ReaderStats), ReaderError> {
        let metadata = std::fs::metadata(&self.path)?;
        let file_size = metadata.len() as usize;

        match self.format {
            DumpFormat::RawBinary => self.read_raw_binary(file_size, self.endianness),
            DumpFormat::HexText => self.read_hex_text(),
            DumpFormat::PcapLe => self.read_raw_binary(file_size, WordEndianness::Standard),
            DumpFormat::PcapBe => self.read_raw_binary(file_size, WordEndianness::Reversed),
        }
    }

    pub fn read_all_timed(&self) -> Result<(Vec<TimedWord>, ReaderStats), ReaderError> {
        let (words, stats) = self.read_all()?;
        let clock = global_precise_clock();
        let start_ts = clock.now();
        let capture_duration_us = if stats.processed_words > 1 {
            (stats.elapsed_ns as u64 / 1000).max(1)
        } else {
            1
        };
        let step_us = capture_duration_us / stats.processed_words.max(1) as u64;

        let timed: Vec<TimedWord> = words
            .into_iter()
            .enumerate()
            .map(|(i, w)| {
                let synthetic_ts = Timestamp::from_micros(
                    start_ts.as_micros() + (i as u64) * step_us
                );
                TimedWord::new(w, synthetic_ts)
            })
            .collect();

        Ok((timed, stats))
    }

    fn read_raw_binary(
        &self,
        file_size: usize,
        endianness: WordEndianness
    ) -> Result<(Vec<ArincWord>, ReaderStats), ReaderError> {
        if file_size % 4 != 0 {
            return Err(ReaderError::InvalidFileSize(file_size));
        }

        let total_words = file_size / 4;
        let mut words: Vec<ArincWord> = Vec::with_capacity(total_words);
        let mut stats = ReaderStats::new(self.path.clone(), file_size, total_words);
        let start = Instant::now();

        let file = File::open(&self.path)?;
        let mut reader = BufReader::with_capacity(64 * 1024, file);

        let mut buf = [0u8; 4];
        let mut word_idx = 0usize;

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if n != 4 {
                return Err(ReaderError::ShortRead {
                    read: n,
                    expected: 4,
                    word_idx,
                });
            }

            let word = match endianness {
                WordEndianness::Standard => ArincWord::from_bytes_le(&buf),
                WordEndianness::Reversed => ArincWord::from_bytes_be(&buf),
            };

            if !word.parity_valid() {
                stats.parity_errors += 1;
            }

            words.push(word);
            word_idx += 1;
        }

        stats.processed_words = words.len();
        stats.elapsed_ns = start.elapsed().as_nanos();
        Ok((words, stats))
    }

    fn read_hex_text(&self) -> Result<(Vec<ArincWord>, ReaderStats), ReaderError> {
        let content = std::fs::read_to_string(&self.path)?;
        let file_size = content.len();
        let mut words: Vec<ArincWord> = Vec::new();
        let mut stats = ReaderStats::new(self.path.clone(), file_size, 0);
        let start = Instant::now();

        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            let clean: String = trimmed
                .chars()
                .filter(|c| c.is_ascii_hexdigit())
                .collect();

            if clean.len() != 8 {
                stats.parse_errors += 1;
                eprintln!("WARN: Line {} has {} hex chars (expected 8): {}",
                    line_no + 1, clean.len(), trimmed);
                continue;
            }

            match u32::from_str_radix(&clean, 16) {
                Ok(raw) => {
                    let word = ArincWord::from_u32(raw, self.endianness);
                    if !word.parity_valid() {
                        stats.parity_errors += 1;
                    }
                    words.push(word);
                }
                Err(_) => {
                    stats.parse_errors += 1;
                }
            }
        }

        stats.total_words = words.len();
        stats.processed_words = words.len();
        stats.elapsed_ns = start.elapsed().as_nanos();
        Ok((words, stats))
    }

    pub fn iter(&self) -> Result<ArincWordIterator, ReaderError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::with_capacity(128 * 1024, file);
        Ok(ArincWordIterator {
            reader,
            endianness: self.endianness,
            word_idx: 0,
            stats: ReaderStats::new(self.path.clone(), 0, 0),
            finished: false,
        })
    }
}

pub struct ArincWordIterator {
    reader: BufReader<File>,
    endianness: WordEndianness,
    word_idx: usize,
    stats: ReaderStats,
    finished: bool,
}

impl Iterator for ArincWordIterator {
    type Item = ArincWord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let mut buf = [0u8; 4];
        match self.reader.read(&mut buf) {
            Ok(0) => {
                self.finished = true;
                None
            }
            Ok(4) => {
                let word = match self.endianness {
                    WordEndianness::Standard => ArincWord::from_bytes_le(&buf),
                    WordEndianness::Reversed => ArincWord::from_bytes_be(&buf),
                };
                self.word_idx += 1;
                self.stats.processed_words += 1;
                Some(word)
            }
            Ok(_) => {
                self.finished = true;
                None
            }
            Err(_) => {
                self.finished = true;
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

#[derive(Debug, Clone)]
pub struct ReaderStats {
    pub file_path: PathBuf,
    pub file_size_bytes: usize,
    pub total_words: usize,
    pub processed_words: usize,
    pub parity_errors: usize,
    pub parse_errors: usize,
    pub elapsed_ns: u128,
}

impl ReaderStats {
    pub fn new(path: PathBuf, file_size: usize, total: usize) -> Self {
        ReaderStats {
            file_path: path,
            file_size_bytes: file_size,
            total_words: total,
            processed_words: 0,
            parity_errors: 0,
            parse_errors: 0,
            elapsed_ns: 0,
        }
    }

    pub fn words_per_second(&self) -> f64 {
        if self.elapsed_ns == 0 {
            return 0.0;
        }
        (self.processed_words as f64 * 1_000_000_000.0) / self.elapsed_ns as f64
    }

    pub fn mb_per_second(&self) -> f64 {
        if self.elapsed_ns == 0 {
            return 0.0;
        }
        let bytes_per_ns = self.file_size_bytes as f64 / self.elapsed_ns as f64;
        bytes_per_ns * 1e9 / 1048576.0
    }
}

impl std::fmt::Display for ReaderStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  📁 File: {}", self.file_path.display())?;
        writeln!(f, "     Size:       {:.2} MB ({} B)",
            self.file_size_bytes as f64 / 1048576.0, self.file_size_bytes)?;
        writeln!(f, "     Words:      {} processed / {} total",
            self.processed_words, self.total_words)?;
        writeln!(f, "     Errors:     {} parity / {} parse",
            self.parity_errors, self.parse_errors)?;
        writeln!(f, "     Speed:      {:.0} words/s | {:.2} MB/s",
            self.words_per_second(), self.mb_per_second())?;
        writeln!(f, "     Time:       {:.3} ms",
            self.elapsed_ns as f64 / 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_binary(path: &Path, words: &[u32]) {
        let mut file = File::create(path).unwrap();
        for w in words {
            file.write_all(&w.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn test_read_raw_binary_valid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");
        create_test_binary(&path, &[0x12345678, 0xABCDEF01, 0x00000001]);

        let reader = ArincDumpReader::new(&path);
        let (words, stats) = reader.read_all().unwrap();

        assert_eq!(words.len(), 3);
        assert_eq!(words[0].raw(), 0x12345678);
        assert_eq!(words[1].raw(), 0xABCDEF01);
        assert_eq!(words[2].raw(), 0x00000001);
        assert_eq!(stats.processed_words, 3);
    }

    #[test]
    fn test_invalid_file_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.bin");
        let mut f = File::create(&path).unwrap();
        f.write_all(&[0, 1, 2]).unwrap();

        let reader = ArincDumpReader::new(&path);
        let result = reader.read_all();
        assert!(matches!(result, Err(ReaderError::InvalidFileSize(3))));
    }

    #[test]
    fn test_read_hex_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hex.txt");
        let content = "\
            # comment line\n\
            12345678\n\
            ABCDEF01\n\
            // another comment\n\
            00000001\n";
        std::fs::write(&path, content).unwrap();

        let reader = ArincDumpReader::new(&path).with_format(DumpFormat::HexText);
        let (words, _) = reader.read_all().unwrap();

        assert_eq!(words.len(), 3);
        assert_eq!(words[0].raw(), 0x12345678);
        assert_eq!(words[1].raw(), 0xABCDEF01);
    }

    #[test]
    fn test_parity_error_counting() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("parity.bin");
        create_test_binary(&path, &[
            0x00000001,
            0x00000003,
            0x00000007,
        ]);

        let reader = ArincDumpReader::new(&path);
        let (_, stats) = reader.read_all().unwrap();
        assert!(stats.parity_errors > 0);
    }

    #[test]
    fn test_iterator() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("iter.bin");
        create_test_binary(&path, &[0x01, 0x02, 0x03, 0x04]);

        let reader = ArincDumpReader::new(&path);
        let count = reader.iter().unwrap().count();
        assert_eq!(count, 4);
    }
}
