/// Prime number sequence clock and pattern generator.
///
/// The scheduler advances a global `step` counter. `is_prime_step(prime_index)`
/// returns true when `step % primes[prime_index] == 0`, producing polyrhythmic
/// gates derived from the prime sequence:
///     p ∈ {2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, ...}
pub struct PrimeScheduler {
    primes: Vec<u64>,
    pub step: u64,
    base: u32,
}

impl PrimeScheduler {
    pub fn new() -> Self {
        Self {
            primes: vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47],
            step: 0,
            base: 2,
        }
    }

    /// Set the base prime value used for gate/pattern generation.
    /// `base` is treated as the actual prime (2, 3, 5, 7, ...).
    pub fn set_base(&mut self, base: u32) {
        self.base = base.max(2);
    }

    pub fn base(&self) -> u32 {
        self.base
    }

    /// True when the current step is divisible by the prime at `prime_index`.
    /// The prime list index is clamped into range.
    pub fn is_prime_step(&self, prime_index: usize) -> bool {
        if let Some(&p) = self.primes.get(prime_index % self.primes.len()) {
            self.step % p == 0
        } else {
            false
        }
    }

    /// The current prime value at the given index.
    pub fn prime_at(&self, index: usize) -> Option<u64> {
        self.primes.get(index % self.primes.len()).copied()
    }

    /// The current base prime value (as configured in `prime_base`).
    pub fn base_prime(&self) -> u64 {
        self.base as u64
    }

    /// Advance the step counter (called once per audio block).
    pub fn tick(&mut self) {
        self.step = self.step.wrapping_add(1);
    }

    /// A Euclidean-style gate pattern derived from the base prime.
    /// Returns `true` on every `base_prime`-th step (i.e. a `1 / p` rhythm).
    pub fn gate_signal(&self) -> bool {
        let p = self.base_prime();
        if p == 0 {
            return false;
        }
        self.step % p == 0
    }

    /// Compute a 0..=1 phase position within the current prime cycle.
    pub fn cycle_phase(&self) -> f32 {
        let p = self.base_prime();
        if p == 0 {
            return 0.0;
        }
        (self.step % p) as f32 / p as f32
    }

    /// Generate a full pattern buffer (e.g. 16 or 32 steps) of gate values for
    /// the given prime index. Useful for UI pattern visualization.
    pub fn pattern(&self, prime_index: usize, steps: usize) -> Vec<bool> {
        let p = self.primes.get(prime_index % self.primes.len()).copied().unwrap_or(2);
        (0..steps).map(|i| i as u64 % p == 0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prime_sequence() {
        let s = PrimeScheduler::new();
        assert_eq!(s.prime_at(0), Some(2));
        assert_eq!(s.prime_at(1), Some(3));
        assert_eq!(s.prime_at(2), Some(5));
        assert_eq!(s.prime_at(3), Some(7));
    }

    #[test]
    fn test_gate_signal() {
        let mut s = PrimeScheduler::new();
        s.set_base(2); // prime 2
        assert!(s.gate_signal()); // step 0 % 2 == 0
        s.tick();
        assert!(!s.gate_signal()); // step 1 % 2 == 1
        s.tick();
        assert!(s.gate_signal()); // step 2 % 2 == 0
    }

    #[test]
    fn test_pattern_length() {
        let s = PrimeScheduler::new();
        let pattern = s.pattern(1, 16); // prime 3 at index 1
        assert_eq!(pattern.len(), 16);
        // 0, 3, 6, 9, 12, 15 => 6 hits.
        assert_eq!(pattern.iter().filter(|&&b| b).count(), 6);
    }
}

