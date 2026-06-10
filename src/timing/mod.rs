pub mod timestamp;
pub mod ringbuf;
pub mod jitter;

pub use timestamp::{Timestamp, PreciseClock, precise_sleep_micros};
pub use ringbuf::LockFreeRingBuffer;
pub use jitter::{JitterStats, LabelTimingRegistry, JitterCalculator, JitterGrade};

use std::sync::Arc;

pub fn global_precise_clock() -> Arc<PreciseClock> {
    use std::sync::OnceLock;
    static CLOCK: OnceLock<Arc<PreciseClock>> = OnceLock::new();
    CLOCK.get_or_init(|| Arc::new(PreciseClock::new())).clone()
}
