use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

/// Shared internal storage for the single-producer single-consumer circular ring buffer.
struct RingBufferShared {
    buffer: Box<[AtomicU32]>,
    capacity: usize,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    overflow_count: AtomicUsize,
}

/// Producer handle for writing audio samples into the ring buffer.
/// Safe to send across threads and call from real-time audio capture callbacks.
pub struct AudioProducer {
    shared: Arc<RingBufferShared>,
    cached_tail: usize,
    local_head: usize,
}

/// Consumer handle for reading audio samples from the ring buffer.
/// Safe to send across threads and call from real-time audio playback or DSP threads.
pub struct AudioConsumer {
    shared: Arc<RingBufferShared>,
    cached_head: usize,
    local_tail: usize,
}

/// Factory for pre-allocated lock-free Single-Producer Single-Consumer (SPSC) ring buffers.
pub struct AudioRingBuffer;

impl AudioRingBuffer {
    /// Default capacity: 32,768 f32 samples (~340 ms buffer at 96 kHz stereo or ~740 ms at 44.1 kHz).
    pub const DEFAULT_CAPACITY: usize = 32768;

    /// Creates a new pre-allocated lock-free SPSC ring buffer.
    /// The capacity will automatically be rounded up to the nearest power of two (minimum 64).
    #[allow(clippy::new_ret_no_self)]
    pub fn new(capacity: usize) -> (AudioProducer, AudioConsumer) {
        let cap = capacity.max(64).next_power_of_two();
        let mask = cap - 1;

        let mut raw = Vec::with_capacity(cap);
        for _ in 0..cap {
            raw.push(AtomicU32::new(0));
        }

        let shared = Arc::new(RingBufferShared {
            buffer: raw.into_boxed_slice(),
            capacity: cap,
            mask,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            overflow_count: AtomicUsize::new(0),
        });

        let producer = AudioProducer {
            shared: Arc::clone(&shared),
            cached_tail: 0,
            local_head: 0,
        };

        let consumer = AudioConsumer {
            shared,
            cached_head: 0,
            local_tail: 0,
        };

        (producer, consumer)
    }
}

impl AudioProducer {
    /// Maximum capacity of the ring buffer in samples.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.shared.capacity
    }

    /// Number of samples currently available to write before buffer fills.
    #[inline]
    pub fn available_write(&mut self) -> usize {
        let occupied = self.local_head.wrapping_sub(self.cached_tail);
        if occupied >= self.shared.capacity {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
        }
        self.shared
            .capacity
            .saturating_sub(self.local_head.wrapping_sub(self.cached_tail))
    }

    /// Approximate count of samples currently stored in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.local_head;
        let tail = self.shared.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail)
    }

    /// Returns true if the buffer currently contains 0 samples.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the buffer has no free space left.
    #[inline]
    pub fn is_full(&mut self) -> bool {
        self.available_write() == 0
    }

    /// Total count of samples dropped due to buffer overflow since creation or last reset.
    #[inline]
    pub fn overflow_count(&self) -> usize {
        self.shared.overflow_count.load(Ordering::Relaxed)
    }

    /// Resets the dropped sample overflow counter to zero, returning the previous count.
    #[inline]
    pub fn reset_overflow_count(&self) -> usize {
        self.shared.overflow_count.swap(0, Ordering::Relaxed)
    }

    /// Pushes a slice of f32 samples into the buffer.
    /// Returns the number of samples actually written.
    /// If the buffer is full, excess samples are dropped and the overflow count is incremented.
    pub fn push_slice(&mut self, samples: &[f32]) -> usize {
        let mut avail = self.available_write();
        if avail < samples.len() {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
            let occupied = self.local_head.wrapping_sub(self.cached_tail);
            avail = self.shared.capacity.saturating_sub(occupied);
        }

        let to_write = samples.len().min(avail);
        for &sample in &samples[..to_write] {
            let idx = self.local_head & self.shared.mask;
            self.shared.buffer[idx].store(sample.to_bits(), Ordering::Relaxed);
            self.local_head = self.local_head.wrapping_add(1);
        }

        if to_write > 0 {
            self.shared.head.store(self.local_head, Ordering::Release);
        }

        to_write
    }

    /// Pushes a slice of f32 samples with denormal flush-to-zero applied to every sample.
    pub fn push_slice_with_denormal_flush(&mut self, samples: &[f32]) -> usize {
        let mut avail = self.available_write();
        if avail < samples.len() {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
            let occupied = self.local_head.wrapping_sub(self.cached_tail);
            avail = self.shared.capacity.saturating_sub(occupied);
        }

        let to_write = samples.len().min(avail);
        if to_write < samples.len() {
            let dropped = samples.len() - to_write;
            self.shared.overflow_count.fetch_add(dropped, Ordering::Relaxed);
        }

        for &sample in &samples[..to_write] {
            let clean = crate::audio_safety::flush_denormal(sample);
            let idx = self.local_head & self.shared.mask;
            self.shared.buffer[idx].store(clean.to_bits(), Ordering::Relaxed);
            self.local_head = self.local_head.wrapping_add(1);
        }

        if to_write > 0 {
            self.shared.head.store(self.local_head, Ordering::Release);
        }

        to_write
    }

    /// Converts and pushes a slice of 16-bit signed integer samples normalized to [-1.0, 1.0] f32.
    pub fn push_i16_slice(&mut self, samples: &[i16]) -> usize {
        let mut avail = self.available_write();
        if avail < samples.len() {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
            let occupied = self.local_head.wrapping_sub(self.cached_tail);
            avail = self.shared.capacity.saturating_sub(occupied);
        }

        let to_write = samples.len().min(avail);
        if to_write < samples.len() {
            let dropped = samples.len() - to_write;
            self.shared.overflow_count.fetch_add(dropped, Ordering::Relaxed);
        }

        for &s in &samples[..to_write] {
            let sample_f32 = s as f32 / 32768.0;
            let clean = crate::audio_safety::flush_denormal(sample_f32);
            let idx = self.local_head & self.shared.mask;
            self.shared.buffer[idx].store(clean.to_bits(), Ordering::Relaxed);
            self.local_head = self.local_head.wrapping_add(1);
        }

        if to_write > 0 {
            self.shared.head.store(self.local_head, Ordering::Release);
        }

        to_write
    }

    /// Converts and pushes a slice of 16-bit unsigned integer samples normalized to [-1.0, 1.0] f32.
    pub fn push_u16_slice(&mut self, samples: &[u16]) -> usize {
        let mut avail = self.available_write();
        if avail < samples.len() {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
            let occupied = self.local_head.wrapping_sub(self.cached_tail);
            avail = self.shared.capacity.saturating_sub(occupied);
        }

        let to_write = samples.len().min(avail);
        if to_write < samples.len() {
            let dropped = samples.len() - to_write;
            self.shared.overflow_count.fetch_add(dropped, Ordering::Relaxed);
        }

        for &s in &samples[..to_write] {
            let sample_f32 = (s as f32 - 32768.0) / 32768.0;
            let clean = crate::audio_safety::flush_denormal(sample_f32);
            let idx = self.local_head & self.shared.mask;
            self.shared.buffer[idx].store(clean.to_bits(), Ordering::Relaxed);
            self.local_head = self.local_head.wrapping_add(1);
        }

        if to_write > 0 {
            self.shared.head.store(self.local_head, Ordering::Release);
        }

        to_write
    }

    /// Pushes a single f32 sample. Returns true on success, or false if buffer is full.
    pub fn push(&mut self, sample: f32) -> bool {
        let mut avail = self.available_write();
        if avail == 0 {
            self.cached_tail = self.shared.tail.load(Ordering::Acquire);
            let occupied = self.local_head.wrapping_sub(self.cached_tail);
            avail = self.shared.capacity.saturating_sub(occupied);
        }

        if avail == 0 {
            self.shared.overflow_count.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        let clean = crate::audio_safety::flush_denormal(sample);
        let idx = self.local_head & self.shared.mask;
        self.shared.buffer[idx].store(clean.to_bits(), Ordering::Relaxed);
        self.local_head = self.local_head.wrapping_add(1);
        self.shared.head.store(self.local_head, Ordering::Release);
        true
    }
}

impl AudioConsumer {
    /// Maximum capacity of the ring buffer in samples.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.shared.capacity
    }

    /// Number of readable samples currently waiting in the buffer.
    #[inline]
    pub fn available_read(&mut self) -> usize {
        let mut avail = self.cached_head.wrapping_sub(self.local_tail);
        if avail == 0 {
            self.cached_head = self.shared.head.load(Ordering::Acquire);
            avail = self.cached_head.wrapping_sub(self.local_tail);
        }
        avail
    }

    /// Approximate count of samples waiting in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.shared.head.load(Ordering::Acquire);
        let tail = self.local_tail;
        head.wrapping_sub(tail)
    }

    /// Returns true if no samples are currently waiting to be read.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns true if the buffer is completely filled to capacity.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() >= self.shared.capacity
    }

    /// Reads up to `dest.len()` samples from the buffer into `dest`.
    /// Returns the number of samples actually read.
    pub fn pop_slice(&mut self, dest: &mut [f32]) -> usize {
        let mut avail = self.available_read();
        if avail < dest.len() {
            self.cached_head = self.shared.head.load(Ordering::Acquire);
            avail = self.cached_head.wrapping_sub(self.local_tail);
        }

        let to_read = dest.len().min(avail);
        for out in dest.iter_mut().take(to_read) {
            let idx = self.local_tail & self.shared.mask;
            let bits = self.shared.buffer[idx].load(Ordering::Relaxed);
            *out = f32::from_bits(bits);
            self.local_tail = self.local_tail.wrapping_add(1);
        }

        if to_read > 0 {
            self.shared.tail.store(self.local_tail, Ordering::Release);
        }

        to_read
    }

    /// Pops a single f32 sample from the buffer, or returns None if empty.
    pub fn pop(&mut self) -> Option<f32> {
        if self.available_read() == 0 {
            return None;
        }

        let idx = self.local_tail & self.shared.mask;
        let bits = self.shared.buffer[idx].load(Ordering::Relaxed);
        self.local_tail = self.local_tail.wrapping_add(1);
        self.shared.tail.store(self.local_tail, Ordering::Release);
        Some(f32::from_bits(bits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_ring_buffer_capacity_power_of_two() {
        let (producer, consumer) = AudioRingBuffer::new(100);
        assert_eq!(producer.capacity(), 128);
        assert_eq!(consumer.capacity(), 128);
        assert_eq!(producer.len(), 0);
        assert!(producer.is_empty());
    }

    #[test]
    fn test_ring_buffer_push_pop_single() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(64);
        assert!(producer.push(0.75));
        assert!(producer.push(-0.5));
        assert_eq!(producer.len(), 2);

        assert_eq!(consumer.pop(), Some(0.75));
        assert_eq!(consumer.pop(), Some(-0.5));
        assert_eq!(consumer.pop(), None);
        assert!(consumer.is_empty());
    }

    #[test]
    fn test_ring_buffer_push_pop_slice() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(64);
        let input = [0.1f32, 0.2, 0.3, 0.4, 0.5];
        let written = producer.push_slice(&input);
        assert_eq!(written, 5);

        let mut output = [0.0f32; 5];
        let read = consumer.pop_slice(&mut output);
        assert_eq!(read, 5);
        assert_eq!(output, input);
    }

    #[test]
    fn test_ring_buffer_wrap_around_multiple_cycles() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(64);
        let cap = producer.capacity();

        for cycle in 0..10 {
            let offset = (cycle * 50) as f32;
            let samples: Vec<f32> = (0..50).map(|i| offset + i as f32).collect();
            let written = producer.push_slice(&samples);
            assert_eq!(written, 50);

            let mut read_buf = vec![0.0f32; 50];
            let read = consumer.pop_slice(&mut read_buf);
            assert_eq!(read, 50);
            assert_eq!(read_buf, samples);
        }
        assert_eq!(producer.capacity(), cap);
        assert_eq!(producer.overflow_count(), 0);
    }

    #[test]
    fn test_ring_buffer_overflow_handling() {
        let (mut producer, _consumer) = AudioRingBuffer::new(64);
        let cap = producer.capacity(); // 64
        let large_input = vec![1.0f32; cap + 20];
        let written = producer.push_slice_with_denormal_flush(&large_input);
        assert_eq!(written, cap);
        assert_eq!(producer.overflow_count(), 20);

        let reset = producer.reset_overflow_count();
        assert_eq!(reset, 20);
        assert_eq!(producer.overflow_count(), 0);
    }

    #[test]
    fn test_ring_buffer_denormal_flush() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(64);
        let denormal_samples = [1e-20f32, -1e-25f32, 0.5f32];
        producer.push_slice_with_denormal_flush(&denormal_samples);

        let mut out = [0.0f32; 3];
        consumer.pop_slice(&mut out);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 0.0);
        assert_eq!(out[2], 0.5);
    }

    #[test]
    fn test_ring_buffer_i16_and_u16_conversion() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(64);
        let i16_samples = [0i16, 16384, -32768];
        producer.push_i16_slice(&i16_samples);

        let mut out = [0.0f32; 3];
        consumer.pop_slice(&mut out);
        assert!((out[0] - 0.0).abs() < 1e-5);
        assert!((out[1] - 0.5).abs() < 1e-4);
        assert!((out[2] - (-1.0)).abs() < 1e-4);

        let u16_samples = [32768u16, 49152, 0];
        producer.push_u16_slice(&u16_samples);
        consumer.pop_slice(&mut out);
        assert!((out[0] - 0.0).abs() < 1e-5);
        assert!((out[1] - 0.5).abs() < 1e-4);
        assert!((out[2] - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn test_ring_buffer_multithreaded_stress() {
        let (mut producer, mut consumer) = AudioRingBuffer::new(512);
        const TOTAL_SAMPLES: usize = 20_000;

        let producer_handle = thread::spawn(move || {
            let mut sent = 0;
            let mut val = 1.0f32;

            while sent < TOTAL_SAMPLES {
                let to_send = (TOTAL_SAMPLES - sent).min(64);
                let mut chunk = [0.0f32; 64];
                for (i, slot) in chunk.iter_mut().take(to_send).enumerate() {
                    *slot = val + i as f32;
                }
                let written = producer.push_slice(&chunk[..to_send]);
                val += written as f32;
                sent += written;
                if written == 0 {
                    thread::yield_now();
                }
            }
        });

        let consumer_handle = thread::spawn(move || {
            let mut received = 0;
            let mut expected = 1.0f32;
            let mut chunk = [0.0f32; 64];

            while received < TOTAL_SAMPLES {
                let read = consumer.pop_slice(&mut chunk);
                for &sample in &chunk[..read] {
                    assert_eq!(sample, expected);
                    expected += 1.0;
                }
                received += read;
                if read == 0 {
                    thread::yield_now();
                }
            }
        });

        producer_handle.join().expect("Producer thread failed");
        consumer_handle.join().expect("Consumer thread failed");
    }
}
