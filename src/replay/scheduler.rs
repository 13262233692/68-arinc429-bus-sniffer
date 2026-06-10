use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use colored::Colorize;

use crate::timing::{Timestamp, PreciseClock, precise_sleep_micros};
use crate::core::word::ArincWord;
use crate::TimedWord;
use crate::replay::injector::{BusInjector, InjectionStrategy, HardwarePort};

#[derive(Debug, Clone)]
pub struct ReplayConfig {
    pub speed_multiplier: f64,
    pub loop_count: usize,
    pub max_jitter_compensation_us: u64,
    pub port_ids: Vec<u8>,
    pub dry_run: bool,
    pub strategy: InjectionStrategy,
    pub stop_on_key: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        ReplayConfig {
            speed_multiplier: 1.0,
            loop_count: 1,
            max_jitter_compensation_us: 50,
            port_ids: vec![0],
            dry_run: false,
            strategy: InjectionStrategy::RoundRobin,
            stop_on_key: true,
        }
    }
}

impl ReplayConfig {
    pub fn with_speed_multiplier(mut self, s: f64) -> Self {
        self.speed_multiplier = s.max(0.001);
        self
    }
    pub fn with_cycles(mut self, n: u32) -> Self {
        self.loop_count = n.max(1) as usize;
        self
    }
    pub fn with_strategy(mut self, s: InjectionStrategy) -> Self {
        self.strategy = s;
        self
    }
    pub fn with_dry_run(mut self, dry: bool) -> Self {
        self.dry_run = dry;
        self
    }
    pub fn with_ports(mut self, ids: &[u8]) -> Self {
        self.port_ids = ids.to_vec();
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct ReplayResult {
    pub total_words: u64,
    pub total_bytes_injected: u64,
    pub loops_executed: usize,
    pub elapsed_us: u64,
    pub timing_errors: u64,
    pub ports_used: Vec<u8>,
    pub average_latency_us: f64,
    pub peak_latency_us: u64,
}

impl ReplayResult {
    pub fn throughput_words_per_sec(&self) -> f64 {
        if self.elapsed_us == 0 {
            return 0.0;
        }
        self.total_words as f64 / (self.elapsed_us as f64 / 1_000_000.0)
    }

    pub fn throughput_mbps(&self) -> f64 {
        if self.elapsed_us == 0 {
            return 0.0;
        }
        (self.total_bytes_injected as f64 * 8.0) / (self.elapsed_us as f64 / 1_000_000.0) / 1_000_000.0
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} words · {:.0} w/s · {:.2} Mbps · {} loops · avg {:.1}µs lat",
            self.total_words,
            self.throughput_words_per_sec(),
            self.throughput_mbps(),
            self.loops_executed,
            self.average_latency_us
        )
    }

    pub fn detailed_report(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("    Words Injected:     {}\n", self.total_words));
        s.push_str(&format!("    Bytes Written:      {} ({:.2} KB)\n",
            self.total_bytes_injected, self.total_bytes_injected as f64 / 1024.0));
        s.push_str(&format!("    Loops Completed:    {}\n", self.loops_executed));
        s.push_str(&format!("    Wall Clock Time:    {:.3} ms\n", self.elapsed_us as f64 / 1_000.0));
        s.push_str(&format!("    Throughput:         {:.0} words/sec  |  {:.2} Mbps\n",
            self.throughput_words_per_sec(), self.throughput_mbps()));
        s.push_str(&format!("    Injection Latency:  avg {:.2}µs  |  peak {}µs\n",
            self.average_latency_us, self.peak_latency_us));
        s.push_str(&format!("    Timing Violations:  {} (> max jitter comp)\n", self.timing_errors));
        s.push_str(&format!("    HW Ports Targeted:  {:?}", self.ports_used));
        s
    }
}

impl std::fmt::Display for ReplayResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  {} {}", "▸".cyan(), "REPLAY INJECTION SUMMARY".cyan().bold())?;
        writeln!(f, "     Words:       {} ({} bytes)", self.total_words, self.total_bytes_injected)?;
        writeln!(f, "     Loops:       {}", self.loops_executed)?;
        writeln!(f, "     Elapsed:     {:.3} ms", self.elapsed_us as f64 / 1_000.0)?;
        writeln!(f, "     Throughput:  {:.0} w/s | {:.2} Mbps",
            self.throughput_words_per_sec(), self.throughput_mbps())?;
        writeln!(f, "     Latency:     avg {:.2}µs | peak {}µs",
            self.average_latency_us, self.peak_latency_us)?;
        writeln!(f, "     Timing Err:  {}", self.timing_errors)?;
        write!(f,   "     Ports Used:  {:?}", self.ports_used)?;
        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct BusPortPool {
    ports: Vec<HardwarePort>,
    round_robin_idx: Arc<AtomicU64>,
}

#[derive(Debug, Default, Clone)]
pub struct BusPortPoolBuilder {
    port_ids: Vec<u8>,
    strategy: Option<InjectionStrategy>,
    dry_run: bool,
    sink_file: Option<std::path::PathBuf>,
}

impl BusPortPool {
    pub fn builder() -> BusPortPoolBuilder {
        BusPortPoolBuilder::default()
    }

    pub fn new() -> Self {
        BusPortPool {
            ports: Vec::new(),
            round_robin_idx: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn port_ids(&self) -> Vec<u8> {
        self.ports.iter().map(|p| p.id).collect()
    }

    pub fn from_port_ids(ids: &[u8]) -> Result<Self> {
        let mut pool = Self::new();
        for &id in ids {
            pool.add_port(HardwarePort::open(id)?);
        }
        Ok(pool)
    }

    pub fn add_port(&mut self, port: HardwarePort) {
        self.ports.push(port);
    }

    pub fn len(&self) -> usize {
        self.ports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ports.is_empty()
    }

    pub fn next_port_rr(&self) -> Option<&HardwarePort> {
        if self.ports.is_empty() {
            return None;
        }
        let idx = self.round_robin_idx.fetch_add(1, Ordering::Relaxed) % self.ports.len() as u64;
        Some(&self.ports[idx as usize])
    }
}

impl BusPortPoolBuilder {
    pub fn ports(mut self, ids: Vec<u8>) -> Self {
        self.port_ids = ids;
        self
    }
    pub fn strategy(mut self, s: InjectionStrategy) -> Self {
        self.strategy = Some(s);
        self
    }
    pub fn dry_run(mut self, d: bool) -> Self {
        self.dry_run = d;
        self
    }
    pub fn sink_file<P: Into<std::path::PathBuf>>(mut self, p: Option<P>) -> Self {
        self.sink_file = p.map(Into::into);
        self
    }
    pub fn build(self) -> Result<BusPortPool> {
        let mut pool = BusPortPool::new();
        if let Some(sink) = self.sink_file {
            for (i, &id) in self.port_ids.iter().enumerate() {
                let p = if self.port_ids.len() == 1 {
                    sink.clone()
                } else {
                    let ext = sink.extension().map(|e| e.to_os_string()).unwrap_or_default();
                    let stem = sink.file_stem().map(|s| s.to_os_string()).unwrap_or_default();
                    let parent = sink.parent().map(|p| p.to_path_buf()).unwrap_or_default();
                    let mut f = std::ffi::OsString::new();
                    f.push(stem);
                    f.push(format!("_p{}", id));
                    if !ext.is_empty() {
                        f.push(".");
                        f.push(ext);
                    }
                    let _ = i;
                    parent.join(f)
                };
                pool.add_port(HardwarePort::open_sink(id, &p)?);
            }
        } else if self.dry_run {
            for &id in self.port_ids.iter() {
                let tmp = std::env::temp_dir().join(format!("arinc429_dry_tx{}.bin", id));
                pool.add_port(HardwarePort::open_sink(id, &tmp)?);
            }
        } else {
            for &id in self.port_ids.iter() {
                pool.add_port(HardwarePort::open(id)?);
            }
        }
        let _ = self.strategy;
        Ok(pool)
    }
}

pub struct ReplayScheduler {
    config: ReplayConfig,
    injector: BusInjector,
    port_pool: BusPortPool,
    stop_flag: Arc<AtomicBool>,
}

impl ReplayScheduler {
    pub fn new(config: ReplayConfig, port_pool: BusPortPool, stop_flag: Arc<AtomicBool>) -> Self {
        ReplayScheduler {
            injector: BusInjector::new(config.strategy, stop_flag.clone()),
            config,
            port_pool,
            stop_flag,
        }
    }

    pub fn request_stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    pub fn run_with_timestamps(
        &self,
        timed_words: &[TimedWord],
    ) -> Result<ReplayResult> {
        if timed_words.is_empty() {
            return Err(anyhow!("Empty capture - no words to replay"));
        }

        let refs: Vec<(ArincWord, Timestamp)> = timed_words
            .iter()
            .map(|tw| (tw.word.clone(), tw.timestamp))
            .collect();
        self.run_impl(&refs)
    }

    fn run_impl(
        &self,
        timed_words: &[(ArincWord, Timestamp)],
    ) -> Result<ReplayResult> {

        let mut result = ReplayResult::default();
        result.ports_used = self.config.port_ids.clone();

        let start_instant = Instant::now();
        let capture_start_ts = timed_words[0].1;
        let start_epoc_us = PreciseClock::new();

        let mut latency_sum: u128 = 0;
        let mut peak_latency = 0u64;
        let mut last_print_at = Instant::now();

        for loop_idx in 0..self.config.loop_count.max(1) {
            if self.stop_flag.load(Ordering::Relaxed) {
                eprintln!("  {} Stop requested by user", "⚠".yellow());
                break;
            }

            let loop_start_epoc = start_epoc_us.now().as_micros();

            for (seq, (word, capture_ts)) in timed_words.iter().enumerate() {
                if self.stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                let relative_us = capture_ts.0.saturating_sub(capture_start_ts.0);
                let scheduled_relative_us = (relative_us as f64 / self.config.speed_multiplier) as u64;
                let target_ts_us = loop_start_epoc + scheduled_relative_us;

                let _ = loop_idx;

                let now_us = start_epoc_us.now().as_micros();
                loop {
                    let cur = start_epoc_us.now().as_micros();
                    let target = target_ts_us;
                    if cur >= target {
                        break;
                    }
                    let remain = target.saturating_sub(cur);
                    if remain > 50 {
                        precise_sleep_micros(remain.saturating_sub(30));
                    } else {
                        std::hint::spin_loop();
                    }
                }

                let inject_before = Instant::now();
                let scheduled_end = capture_start_ts.0 + scheduled_relative_us;
                let _ = scheduled_end;

                let inject_ok = if self.config.dry_run {
                    true
                } else {
                    let port = self.port_pool.next_port_rr();
                    self.injector.inject_word(word, port)?;
                    true
                };

                let latency_us = inject_before.elapsed().as_micros() as u64;
                latency_sum += latency_us as u128;
                if latency_us > peak_latency {
                    peak_latency = latency_us;
                }

                if latency_us > self.config.max_jitter_compensation_us {
                    result.timing_errors += 1;
                }

                if inject_ok {
                    result.total_words += 1;
                    result.total_bytes_injected += 4;
                }

                let _ = now_us;

                if last_print_at.elapsed() > Duration::from_millis(500) {
                    eprint!(
                        "\r  {} Loop {} | [{}/{}] injected | avg {:.1}µs lat  ",
                        "⟳".cyan(),
                        loop_idx + 1,
                        seq + 1,
                        timed_words.len(),
                        if result.total_words > 0 {
                            latency_sum as f64 / result.total_words as f64
                        } else {
                            0.0
                        }
                    );
                    last_print_at = Instant::now();
                }
            }
            result.loops_executed = loop_idx + 1;
        }
        eprintln!();

        result.elapsed_us = start_instant.elapsed().as_micros() as u64;
        if result.total_words > 0 {
            result.average_latency_us = latency_sum as f64 / result.total_words as f64;
        }
        result.peak_latency_us = peak_latency;

        Ok(result)
    }
}
