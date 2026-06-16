//! Bucketed histogram — Prometheus semantics (cumulative buckets).
//!
//! Buckets are caller-provided, **sorted ascending** boundaries.  Each
//! observation increments every bucket whose upper bound is ≥ value
//! (cumulative encoding).  Plus a `+Inf` total + sum-of-observations.
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Histogram {
    buckets:    &'static [f64],
    counts:     &'static [AtomicU64],
    sum_bits:   AtomicU64,
    inf_count:  AtomicU64,
}

impl Histogram {
    /// SAFETY: `counts.len() == buckets.len()`.
    pub const fn new(buckets: &'static [f64], counts: &'static [AtomicU64]) -> Self {
        Self {
            buckets, counts,
            sum_bits:  AtomicU64::new(0),
            inf_count: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn observe(&self, v: f64) {
        for (i, &bound) in self.buckets.iter().enumerate() {
            if v <= bound { self.counts[i].fetch_add(1, Ordering::Relaxed); }
        }
        self.inf_count.fetch_add(1, Ordering::Relaxed);
        self.add_to_sum(v);
    }

    pub fn snapshot(&self) -> (f64, u64, alloc::vec::Vec<(f64, u64)>) {
        let sum = f64::from_bits(self.sum_bits.load(Ordering::Relaxed));
        let total = self.inf_count.load(Ordering::Relaxed);
        let bs: alloc::vec::Vec<(f64, u64)> = self.buckets.iter().enumerate()
            .map(|(i, &b)| (b, self.counts[i].load(Ordering::Relaxed)))
            .collect();
        (sum, total, bs)
    }

    fn add_to_sum(&self, v: f64) {
        // CAS-based float add — rare, OK for a histogram fast path that
        // is mostly bucket increments.
        let mut cur = self.sum_bits.load(Ordering::Relaxed);
        loop {
            let new = (f64::from_bits(cur) + v).to_bits();
            match self.sum_bits.compare_exchange_weak(cur, new,
                Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_)        => return,
                Err(actual)  => cur = actual,
            }
        }
    }
}
