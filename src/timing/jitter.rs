use std::collections::HashMap;
use std::sync::RwLock;

use crate::timing::ringbuf::LockFreeRingBuffer;
use crate::timing::timestamp::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitterGrade {
    Subatomic,
    AerospaceGrade,
    Acceptable,
    Noticeable,
    Degraded,
    Anomalous,
    FaultSuspected,
}

#[derive(Debug, Clone, Default)]
pub struct JitterStats {
    pub label_octal: u16,
    pub sample_count: u64,
    pub mean_interval_us: f64,
    pub min_interval_us: u64,
    pub max_interval_us: u64,
    pub stddev_interval_us: f64,
    pub period_us: f64,
    pub jitter_ppm: f64,
    pub jitter_peak_ppm: f64,
    pub last_arrival: Option<Timestamp>,
}

impl JitterStats {
    pub fn is_periodic(&self) -> bool {
        self.sample_count >= 5 && self.mean_interval_us > 1.0
    }

    #[inline]
    pub fn mean_us(&self) -> f64 { self.mean_interval_us }
    #[inline]
    pub fn std_dev_us(&self) -> f64 { self.stddev_interval_us }
    #[inline]
    pub fn period_us(&self) -> f64 { self.period_us }
    #[inline]
    pub fn ppm_jitter(&self) -> f64 { self.jitter_ppm }
    #[inline]
    pub fn peak_ppm(&self) -> f64 { self.jitter_peak_ppm }
    #[inline]
    pub fn sample_count(&self) -> u64 { self.sample_count }

    pub fn grade(&self) -> JitterGrade {
        if !self.is_periodic() {
            return JitterGrade::FaultSuspected;
        }
        let ppm = self.jitter_ppm;
        if ppm < 1.0 { JitterGrade::Subatomic }
        else if ppm < 50.0 { JitterGrade::AerospaceGrade }
        else if ppm < 200.0 { JitterGrade::Acceptable }
        else if ppm < 1000.0 { JitterGrade::Noticeable }
        else if ppm < 5000.0 { JitterGrade::Degraded }
        else if ppm < 20_000.0 { JitterGrade::Anomalous }
        else { JitterGrade::FaultSuspected }
    }

    pub fn jitter_classification(&self) -> &'static str {
        if !self.is_periodic() {
            "ASYNC/SPORADIC"
        } else if self.jitter_ppm < 100.0 {
            "STABLE (<100ppm)"
        } else if self.jitter_ppm < 1000.0 {
            "MODERATE (<0.1%)"
        } else if self.jitter_ppm < 10_000.0 {
            "HIGH (<1%)"
        } else {
            "CRITICAL (>1%)"
        }
    }
}

impl std::fmt::Display for JitterStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.sample_count == 0 {
            return write!(f, "LABEL={:03o}: NO DATA", self.label_octal);
        }
        write!(
            f,
            "LABEL={:03o} | N={:<6} | T={:>9.2}µs | µ={:>9.2}µs | σ={:>8.2}µs | MIN/MAX={}/{}µs | JITTER={:>8.2} ppm [{}] PEAK={:.2} ppm",
            self.label_octal,
            self.sample_count,
            self.period_us,
            self.mean_interval_us,
            self.stddev_interval_us,
            self.min_interval_us,
            self.max_interval_us,
            self.jitter_ppm,
            self.jitter_classification(),
            self.jitter_peak_ppm
        )
    }
}

pub struct JitterCalculator {
    window_size: usize,
    stats: JitterStats,
    intervals: LockFreeRingBuffer<u32>,
    sum: f64,
    sum_sq: f64,
}

impl JitterCalculator {
    pub fn new(label_octal: u16, window_size: usize) -> Self {
        JitterCalculator {
            window_size,
            stats: JitterStats {
                label_octal,
                min_interval_us: u64::MAX,
                ..Default::default()
            },
            intervals: LockFreeRingBuffer::new(window_size.max(16)),
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    pub fn record_arrival(&mut self, timestamp: Timestamp) {
        if let Some(prev) = self.stats.last_arrival.take() {
            let delta = timestamp.0.saturating_sub(prev.0) as u32;
            if delta > 0 {
                let evicted = self.intervals.push(delta);
                self.sum += delta as f64;
                self.sum_sq += (delta as f64) * (delta as f64);
                if let Some(old) = evicted {
                    self.sum -= old as f64;
                    self.sum_sq -= (old as f64) * (old as f64);
                }

                let d64 = delta as u64;
                if d64 < self.stats.min_interval_us {
                    self.stats.min_interval_us = d64;
                }
                if d64 > self.stats.max_interval_us {
                    self.stats.max_interval_us = d64;
                }
                self.stats.sample_count += 1;
                self.recalc_statistics();
            }
        }
        self.stats.last_arrival = Some(timestamp);
    }

    fn recalc_statistics(&mut self) {
        let n = self.intervals.len().max(1) as f64;
        let mean = self.sum / n;
        let variance = (self.sum_sq / n) - (mean * mean);
        let stddev = variance.max(0.0).sqrt();

        self.stats.mean_interval_us = mean;
        self.stats.stddev_interval_us = stddev;

        if self.stats.sample_count >= 3 && mean > 1.0 {
            self.stats.period_us = mean;
            self.stats.jitter_ppm = (stddev / mean) * 1_000_000.0;
            let max_deviation = self
                .stats
                .max_interval_us
                .saturating_sub(self.stats.min_interval_us) as f64;
            self.stats.jitter_peak_ppm = (max_deviation / mean) * 1_000_000.0;
        }
    }

    pub fn stats(&self) -> &JitterStats {
        &self.stats
    }

    pub fn intervals_snapshot(&self) -> Vec<u32> {
        self.intervals.snapshot()
    }
}

pub struct LabelTimingRegistry {
    calculators: RwLock<HashMap<u16, JitterCalculator>>,
    window_size: usize,
    capture_start: std::sync::Mutex<Option<Timestamp>>,
    capture_end: std::sync::Mutex<Option<Timestamp>>,
}

impl LabelTimingRegistry {
    pub fn new() -> Self {
        LabelTimingRegistry {
            calculators: RwLock::new(HashMap::new()),
            window_size: 4096,
            capture_start: std::sync::Mutex::new(None),
            capture_end: std::sync::Mutex::new(None),
        }
    }

    pub fn with_window(window_size: usize) -> Self {
        LabelTimingRegistry {
            calculators: RwLock::new(HashMap::new()),
            window_size: window_size.max(16),
            capture_start: std::sync::Mutex::new(None),
            capture_end: std::sync::Mutex::new(None),
        }
    }

    pub fn record(&self, label_octal: u16, timestamp: Timestamp) {
        let mut start = self.capture_start.lock().unwrap();
        if start.is_none() {
            *start = Some(timestamp);
        }
        drop(start);
        let mut end = self.capture_end.lock().unwrap();
        *end = Some(timestamp);
        drop(end);

        let write_needed = {
            let read = self.calculators.read().unwrap();
            !read.contains_key(&label_octal)
        };

        if write_needed {
            let mut write = self.calculators.write().unwrap();
            write.entry(label_octal)
                .or_insert_with(|| JitterCalculator::new(label_octal, self.window_size));
        }

        let mut write = self.calculators.write().unwrap();
        if let Some(calc) = write.get_mut(&label_octal) {
            calc.record_arrival(timestamp);
        }
    }

    pub fn get_stats(&self, label_octal: u16) -> Option<JitterStats> {
        let read = self.calculators.read().unwrap();
        read.get(&label_octal).map(|c| c.stats().clone())
    }

    pub fn all_stats(&self) -> Vec<JitterStats> {
        let read = self.calculators.read().unwrap();
        read.values().map(|c| c.stats().clone()).collect()
    }

    pub fn unique_labels_seen(&self) -> usize {
        self.calculators.read().unwrap().len()
    }

    pub fn capture_duration_us(&self) -> u64 {
        let start = *self.capture_start.lock().unwrap();
        let end = *self.capture_end.lock().unwrap();
        match (start, end) {
            (Some(s), Some(e)) => e.0.saturating_sub(s.0),
            _ => 0,
        }
    }

    pub fn top_n_by_jitter(&self, n: usize) -> Vec<JitterStats> {
        let mut all = self.all_stats();
        all.sort_by(|a, b| {
            let pa = a.is_periodic() as u8;
            let pb = b.is_periodic() as u8;
            pb.cmp(&pa)
                .then_with(|| b.jitter_ppm.partial_cmp(&a.jitter_ppm).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| b.sample_count.cmp(&a.sample_count))
        });
        all.truncate(n);
        all
    }

    pub fn top_n_by_frequency(&self, n: usize) -> Vec<JitterStats> {
        let mut all = self.all_stats();
        all.sort_by(|a, b| b.sample_count.cmp(&a.sample_count));
        all.truncate(n);
        all
    }

    pub fn top_jitter_labels(&self, n: usize) -> Vec<(u16, JitterStats)> {
        let sorted = self.top_n_by_jitter(n);
        sorted.into_iter().map(|s| (s.label_octal, s)).collect()
    }

    pub fn clear(&self) {
        self.calculators.write().unwrap().clear();
        *self.capture_start.lock().unwrap() = None;
        *self.capture_end.lock().unwrap() = None;
    }

    pub fn total_words(&self) -> u64 {
        let read = self.calculators.read().unwrap();
        read.values().map(|c| c.stats().sample_count.max(1) - 1 + if c.stats().sample_count > 0 { 1 } else { 0 }).sum()
    }
}

impl Default for LabelTimingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jitter_calc_stable_periodic() {
        let mut calc = JitterCalculator::new(0o001, 64);
        let period = 10_000u64;
        for i in 0..10 {
            let ts = Timestamp::from_micros(i * period);
            calc.record_arrival(ts);
        }
        let stats = calc.stats();
        assert!(stats.sample_count >= 5);
        assert!((stats.mean_interval_us - period as f64).abs() < 1.0,
            "mean should be ~10000µs, got {}", stats.mean_interval_us);
        assert!(stats.jitter_ppm < 100.0,
            "perfect periodic should have near-zero jitter, got {} ppm", stats.jitter_ppm);
    }

    #[test]
    fn test_jitter_calc_high_jitter() {
        let mut calc = JitterCalculator::new(0o002, 256);
        let base = 10_000u64;
        let mut t = 0u64;
        for i in 0..20 {
            let jitter = (i % 3) * 5000;
            t += base + jitter;
            calc.record_arrival(Timestamp::from_micros(t));
        }
        let stats = calc.stats();
        assert!(stats.sample_count >= 10);
        assert!(stats.jitter_ppm > 10_000.0,
            "variable interval should register HIGH jitter, got {} ppm", stats.jitter_ppm);
    }

    #[test]
    fn test_registry_multi_labels() {
        let reg = LabelTimingRegistry::with_window(128);
        let mut t3 = 0u64;
        for i in 0..60 {
            reg.record(0o001, Timestamp::from_micros(i * 5000));
            reg.record(0o002, Timestamp::from_micros(i * 10000));
            t3 += 5000 + (i as u64 % 5) * 2000;
            reg.record(0o003, Timestamp::from_micros(t3));
        }
        assert_eq!(reg.unique_labels_seen(), 3);
        let s1 = reg.get_stats(0o001).unwrap();
        let s3 = reg.get_stats(0o003).unwrap();
        eprintln!("s1: label={:o} N={} mean={} stddev={} ppm={}",
            s1.label_octal, s1.sample_count, s1.mean_interval_us, s1.stddev_interval_us, s1.jitter_ppm);
        eprintln!("s3: label={:o} N={} mean={} stddev={} ppm={}",
            s3.label_octal, s3.sample_count, s3.mean_interval_us, s3.stddev_interval_us, s3.jitter_ppm);
        assert!(s1.jitter_ppm < s3.jitter_ppm.max(1.0),
            "label 1 should have lower jitter than label 3: s1={}ppm, s3={}ppm",
            s1.jitter_ppm, s3.jitter_ppm);
        assert!(s3.jitter_ppm > 1000.0,
            "label 3 with variable intervals should register >1000ppm jitter, got {}ppm",
            s3.jitter_ppm);
    }
}
