use std::sync::atomic::{AtomicUsize, Ordering};
use std::cell::UnsafeCell;

pub struct LockFreeRingBuffer<T: Copy + Default> {
    buffer: UnsafeCell<Box<[T]>>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    count: AtomicUsize,
}

unsafe impl<T: Copy + Default + Send> Send for LockFreeRingBuffer<T> {}
unsafe impl<T: Copy + Default + Sync> Sync for LockFreeRingBuffer<T> {}

impl<T: Copy + Default> LockFreeRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be positive");
        let actual_cap = capacity.next_power_of_two();
        let mut buf = Vec::with_capacity(actual_cap);
        buf.resize(actual_cap, T::default());
        LockFreeRingBuffer {
            buffer: UnsafeCell::new(buf.into_boxed_slice()),
            capacity: actual_cap,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn push(&self, value: T) -> Option<T> {
        let mut old_count = self.count.load(Ordering::Acquire);
        loop {
            if old_count >= self.capacity {
                let idx = self.head.load(Ordering::Relaxed);
                let buf = unsafe { &mut *self.buffer.get() };
                let oldest = unsafe { *buf.get_unchecked(idx) };
                buf[idx] = value;
                self.head.store((idx + 1) & (self.capacity - 1), Ordering::Release);
                return Some(oldest);
            }
            match self.count.compare_exchange_weak(
                old_count,
                old_count + 1,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(x) => old_count = x,
            }
        }
        let idx = self.tail.load(Ordering::Relaxed);
        let buf = unsafe { &mut *self.buffer.get() };
        buf[idx] = value;
        self.tail.store((idx + 1) & (self.capacity - 1), Ordering::Release);
        None
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        let mut old_count = self.count.load(Ordering::Acquire);
        loop {
            if old_count == 0 {
                return None;
            }
            match self.count.compare_exchange_weak(
                old_count,
                old_count - 1,
                Ordering::SeqCst,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(x) => old_count = x,
            }
        }
        let idx = self.head.load(Ordering::Relaxed);
        let buf = unsafe { &*self.buffer.get() };
        let val = unsafe { *buf.get_unchecked(idx) };
        self.head.store((idx + 1) & (self.capacity - 1), Ordering::Release);
        Some(val)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn snapshot(&self) -> Vec<T> {
        let _ = self.count.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        let count = self.count.load(Ordering::Acquire);
        let mut result = Vec::with_capacity(count);
        let buf = unsafe { &*self.buffer.get() };
        for i in 0..count {
            let idx = (head + i) & (self.capacity - 1);
            result.push(unsafe { *buf.get_unchecked(idx) });
        }
        let _ = (head, tail);
        result
    }

    pub fn clear(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        self.count.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ringbuf_push_pop() {
        let rb: LockFreeRingBuffer<u32> = LockFreeRingBuffer::new(4);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());

        assert!(rb.push(1).is_none());
        assert!(rb.push(2).is_none());
        assert_eq!(rb.len(), 2);

        assert_eq!(rb.pop(), Some(1));
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), None);
    }

    #[test]
    fn test_ringbuf_overflow_overwrites_oldest() {
        let rb: LockFreeRingBuffer<u32> = LockFreeRingBuffer::new(4);
        for i in 0..4 {
            rb.push(i);
        }
        assert_eq!(rb.len(), 4);

        let evicted = rb.push(100);
        assert_eq!(evicted, Some(0));
        assert_eq!(rb.len(), 4);

        assert_eq!(rb.pop(), Some(1));
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), Some(3));
        assert_eq!(rb.pop(), Some(100));
    }

    #[test]
    fn test_ringbuf_snapshot() {
        let rb: LockFreeRingBuffer<u32> = LockFreeRingBuffer::new(8);
        for i in 0..5 {
            rb.push(i);
        }
        let snap = rb.snapshot();
        assert_eq!(snap, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_ringbuf_wraparound() {
        let rb: LockFreeRingBuffer<u32> = LockFreeRingBuffer::new(4);
        for _ in 0..100 {
            for i in 0..4 {
                rb.push(i);
            }
            for i in 0..4 {
                assert_eq!(rb.pop(), Some(i));
            }
        }
    }
}
