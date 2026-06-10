use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub const ZERO: Timestamp = Timestamp(0);

    #[inline]
    pub fn from_micros(micros: u64) -> Self {
        Timestamp(micros)
    }

    #[inline]
    pub fn as_micros(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn as_nanos(&self) -> u128 {
        self.0 as u128 * 1_000
    }

    #[inline]
    pub fn as_duration(&self) -> Duration {
        Duration::from_micros(self.0)
    }

    #[inline]
    pub fn saturating_sub(&self, other: Timestamp) -> Duration {
        if self.0 >= other.0 {
            Duration::from_micros(self.0 - other.0)
        } else {
            Duration::from_micros(0)
        }
    }

    #[inline]
    pub fn delta_micros(&self, other: Timestamp) -> i128 {
        self.0 as i128 - other.0 as i128
    }
}

impl std::ops::Sub for Timestamp {
    type Output = Duration;
    fn sub(self, rhs: Timestamp) -> Duration {
        self.saturating_sub(rhs)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let micros = self.0;
        if micros < 1_000 {
            write!(f, "{}µs", micros)
        } else if micros < 1_000_000 {
            write!(f, "{:.3}ms", micros as f64 / 1_000.0)
        } else {
            write!(f, "{:.6}s", micros as f64 / 1_000_000.0)
        }
    }
}

pub struct PreciseClock {
    start_instant: Instant,
    start_raw: AtomicU64,
}

impl PreciseClock {
    pub fn new() -> Self {
        let start_instant = Instant::now();
        PreciseClock {
            start_instant,
            start_raw: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn now(&self) -> Timestamp {
        let elapsed = self.start_instant.elapsed();
        let micros = elapsed.as_micros() as u64;
        Timestamp::from_micros(
            self.start_raw.load(Ordering::Relaxed).saturating_add(micros)
        )
    }

    #[inline]
    pub fn elapsed_micros(&self, since: Timestamp) -> u64 {
        let now = self.now();
        now.0.saturating_sub(since.0)
    }

    pub fn set_epoch_offset(&self, offset_micros: u64) {
        self.start_raw.store(offset_micros, Ordering::SeqCst);
    }
}

impl Default for PreciseClock {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
pub fn precise_sleep_micros(micros: u64) {
    if micros == 0 {
        return;
    }
    if micros < 50 {
        spin_wait_micros(micros);
        return;
    }
    let target = Instant::now() + Duration::from_micros(micros);
    let threshold = Duration::from_micros(30);
    loop {
        let now = Instant::now();
        let remaining = target.saturating_duration_since(now);
        if remaining <= threshold {
            spin_wait(target);
            return;
        }
        std::thread::sleep(remaining.saturating_sub(threshold));
    }
}

fn spin_wait(target: Instant) {
    while Instant::now() < target {
        std::hint::spin_loop();
    }
}

fn spin_wait_micros(micros: u64) {
    let target = Instant::now() + Duration::from_micros(micros);
    spin_wait(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_basic() {
        let t1 = Timestamp::from_micros(1000);
        let t2 = Timestamp::from_micros(500);
        assert_eq!(t1 - t2, Duration::from_micros(500));
        assert_eq!(t2 - t1, Duration::from_micros(0));
        assert_eq!(t1.as_micros(), 1000);
    }

    #[test]
    fn test_precise_clock_monotonic() {
        let clock = PreciseClock::new();
        let mut prev = clock.now();
        for _ in 0..100 {
            let now = clock.now();
            assert!(now >= prev, "Clock must be monotonically increasing");
            prev = now;
        }
    }

    #[test]
    fn test_timestamp_delta() {
        let a = Timestamp::from_micros(1500);
        let b = Timestamp::from_micros(1000);
        assert_eq!(a.delta_micros(b), 500);
        assert_eq!(b.delta_micros(a), -500);
    }
}
