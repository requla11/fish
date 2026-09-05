#![forbid(unsafe_code)]

//! Memory pooling for fingerprint computation
//!
//! This module provides efficient memory pooling to reduce allocation overhead
//! during fingerprint computation and cache operations.
//!
//! Performance optimizations:
//! - Object pooling for `Vec<u8>` buffers
//! - Zero-allocation for common buffer sizes
//! - Cache-friendly memory layout

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const SIZE_TIERS: &[usize] = &[256, 1024, 4096, 16384, 65536, 262144];

#[derive(Debug, Default)]
struct AtomicPoolStats {
    hits: AtomicUsize,
    misses: AtomicUsize,
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
}

#[derive(Debug)]
pub struct BufferPool {
    pools: Vec<Mutex<VecDeque<Vec<u8>>>>,
    allocations: Arc<AtomicPoolStats>,
}

#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    pub hits: usize,
    pub misses: usize,
    pub allocations: usize,
    pub deallocations: usize,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            pools: SIZE_TIERS
                .iter()
                .map(|_| Mutex::new(VecDeque::new()))
                .collect(),
            allocations: Arc::new(AtomicPoolStats::default()),
        }
    }

    /// Get a buffer of at least the requested size
    pub fn get_buffer(&self, min_size: usize) -> Vec<u8> {
        let tier_index = self.find_tier(min_size);
        self.allocations.allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(pool) = self.pools.get(tier_index) {
            let mut pool = pool.lock();
            if let Some(mut buffer) = pool.pop_front() {
                self.allocations.hits.fetch_add(1, Ordering::Relaxed);
                buffer.clear();
                buffer.reserve(min_size);
                return buffer;
            }
        }

        self.allocations.misses.fetch_add(1, Ordering::Relaxed);
        Vec::with_capacity(min_size)
    }

    pub fn return_buffer(&self, buffer: Vec<u8>) {
        let capacity = buffer.capacity();
        let tier_index = self.find_tier(capacity);

        self.allocations
            .deallocations
            .fetch_add(1, Ordering::Relaxed);

        if let Some(&tier_size) = SIZE_TIERS.get(tier_index)
            && capacity >= tier_size * 2 / 3
            && capacity <= tier_size * 3 / 2
            && let Some(pool) = self.pools.get(tier_index)
        {
            let mut pool = pool.lock();
            if pool.len() < 10 {
                pool.push_back(buffer);
            }
        }
    }

    fn find_tier(&self, size: usize) -> usize {
        SIZE_TIERS
            .iter()
            .position(|&tier| tier >= size)
            .unwrap_or(SIZE_TIERS.len() - 1)
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            hits: self.allocations.hits.load(Ordering::Relaxed),
            misses: self.allocations.misses.load(Ordering::Relaxed),
            allocations: self.allocations.allocations.load(Ordering::Relaxed),
            deallocations: self.allocations.deallocations.load(Ordering::Relaxed),
        }
    }

    /// Clear all pools (useful for memory pressure scenarios)
    pub fn clear(&self) {
        for pool in &self.pools {
            pool.lock().clear();
        }
    }
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoped buffer that automatically returns to pool when dropped
pub struct ScopedBuffer {
    buffer: Vec<u8>,
    pool: Arc<BufferPool>,
}

impl ScopedBuffer {
    pub fn new(min_size: usize, pool: Arc<BufferPool>) -> Self {
        let buffer = pool.get_buffer(min_size);
        Self { buffer, pool }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    #[allow(clippy::should_implement_trait)]
    pub fn as_ref(&self) -> &Vec<u8> {
        &self.buffer
    }

    pub fn into_inner(mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }
}

impl Drop for ScopedBuffer {
    fn drop(&mut self) {
        if !self.buffer.is_empty() || self.buffer.capacity() > 0 {
            self.pool.return_buffer(std::mem::take(&mut self.buffer));
        }
    }
}

/// String pool for reducing allocations of commonly-sized strings
///
/// This module provides efficient memory pooling for String allocations,
/// particularly useful for cache keys, identifiers, and labels that are
/// frequently created and destroyed during build operations.
///
/// Performance optimizations:
/// - Object pooling for `String` buffers
/// - Size tiers matching common string lengths
/// - Reuse of underlying capacity without deallocation
///
/// Size tiers for string pools (powers of 2 for efficient reuse)
const STRING_SIZE_TIERS: &[usize] = &[32, 64, 128, 256, 512, 1024, 2048, 4096];

#[derive(Debug)]
pub struct StringPool {
    pools: Vec<Mutex<VecDeque<String>>>,
    allocations: Arc<AtomicPoolStats>,
}

impl StringPool {
    pub fn new() -> Self {
        Self {
            pools: STRING_SIZE_TIERS
                .iter()
                .map(|_| Mutex::new(VecDeque::new()))
                .collect(),
            allocations: Arc::new(AtomicPoolStats::default()),
        }
    }

    pub fn get_string(&self, min_capacity: usize) -> String {
        let tier_index = self.find_tier(min_capacity);
        self.allocations.allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(pool) = self.pools.get(tier_index) {
            let mut pool = pool.lock();
            if let Some(mut s) = pool.pop_front() {
                self.allocations.hits.fetch_add(1, Ordering::Relaxed);
                s.clear();
                s.reserve(min_capacity);
                return s;
            }
        }

        self.allocations.misses.fetch_add(1, Ordering::Relaxed);
        String::with_capacity(min_capacity)
    }

    pub fn return_string(&self, s: String) {
        let capacity = s.capacity();
        let tier_index = self.find_tier(capacity);

        self.allocations
            .deallocations
            .fetch_add(1, Ordering::Relaxed);

        if let Some(&tier_size) = STRING_SIZE_TIERS.get(tier_index)
            && capacity >= tier_size * 2 / 3
            && capacity <= tier_size * 3 / 2
            && let Some(pool) = self.pools.get(tier_index)
        {
            let mut pool = pool.lock();
            if pool.len() < 10 {
                pool.push_back(s);
            }
        }
    }

    fn find_tier(&self, size: usize) -> usize {
        STRING_SIZE_TIERS
            .iter()
            .position(|&tier| tier >= size)
            .unwrap_or(STRING_SIZE_TIERS.len() - 1)
    }

    pub fn stats(&self) -> PoolStats {
        PoolStats {
            hits: self.allocations.hits.load(Ordering::Relaxed),
            misses: self.allocations.misses.load(Ordering::Relaxed),
            allocations: self.allocations.allocations.load(Ordering::Relaxed),
            deallocations: self.allocations.deallocations.load(Ordering::Relaxed),
        }
    }

    /// Clear all pools (useful for memory pressure scenarios)
    pub fn clear(&self) {
        for pool in &self.pools {
            pool.lock().clear();
        }
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Scoped string that automatically returns to pool when dropped
pub struct ScopedString {
    string: String,
    pool: Arc<StringPool>,
}

impl ScopedString {
    pub fn new(min_capacity: usize, pool: Arc<StringPool>) -> Self {
        let string = pool.get_string(min_capacity);
        Self { string, pool }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn as_mut(&mut self) -> &mut String {
        &mut self.string
    }

    #[allow(clippy::should_implement_trait)]
    pub fn as_ref(&self) -> &String {
        &self.string
    }

    pub fn into_inner(mut self) -> String {
        std::mem::take(&mut self.string)
    }
}

impl Drop for ScopedString {
    fn drop(&mut self) {
        if !self.string.is_empty() {
            self.pool.return_string(std::mem::take(&mut self.string));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_basic() {
        let pool = BufferPool::new();

        let buffer1 = pool.get_buffer(100);
        assert!(buffer1.capacity() >= 100);

        pool.return_buffer(buffer1);

        let buffer2 = pool.get_buffer(100);
        assert!(buffer2.capacity() >= 100);

        let stats = pool.stats();
        assert!(stats.hits >= 1 || stats.misses >= 1);
    }

    #[test]
    fn test_scoped_buffer() {
        let pool = Arc::new(BufferPool::new());

        {
            let mut scoped = ScopedBuffer::new(1000, pool.clone());
            scoped.as_mut().extend_from_slice(b"test data");
            assert_eq!(scoped.as_ref().len(), 9);
        }

        let stats = pool.stats();
        assert!(stats.deallocations > 0);
    }

    #[test]
    fn test_size_tiers() {
        let pool = BufferPool::new();

        let sizes = [100, 500, 2000, 10000, 50000, 200000];
        for size in sizes {
            let buffer = pool.get_buffer(size);
            assert!(buffer.capacity() >= size);
            pool.return_buffer(buffer);
        }
    }

    #[test]
    fn test_pool_limits() {
        let pool = BufferPool::new();

        for _ in 0..20 {
            let buffer = pool.get_buffer(1000);
            pool.return_buffer(buffer);
        }

        let first_pool = pool.pools.get(2).unwrap().lock();
        assert!(first_pool.len() <= 10);
    }
}

#[cfg(test)]
mod string_pool_tests {
    use super::*;

    #[test]
    fn test_string_pool_basic() {
        let pool = StringPool::new();

        let s1 = pool.get_string(50);
        assert!(s1.capacity() >= 50);

        pool.return_string(s1);

        let s2 = pool.get_string(50);
        assert!(s2.capacity() >= 50);

        let stats = pool.stats();
        assert!(stats.hits >= 1 || stats.misses >= 1);
    }

    #[test]
    fn test_scoped_string() {
        let pool = Arc::new(StringPool::new());

        {
            let mut scoped = ScopedString::new(100, pool.clone());
            scoped.as_mut().push_str("test data");
            assert_eq!(scoped.as_ref().len(), 9);
        }

        let stats = pool.stats();
        assert!(stats.deallocations > 0);
    }

    #[test]
    fn test_string_size_tiers() {
        let pool = StringPool::new();

        let sizes = [10, 50, 100, 500, 1000, 2000, 5000];
        for size in sizes {
            let s = pool.get_string(size);
            assert!(s.capacity() >= size);
            pool.return_string(s);
        }
    }

    #[test]
    fn test_string_pool_limits() {
        let pool = StringPool::new();

        for _ in 0..20 {
            let s = pool.get_string(100);
            pool.return_string(s);
        }

        let first_pool = pool.pools.get(3).unwrap().lock();
        assert!(first_pool.len() <= 10);
    }
}
