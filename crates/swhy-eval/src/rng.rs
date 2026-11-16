//! Deterministic sampling. The generator is SplitMix64: the same seed gives
//! the same sample on every machine and with every version of the toolchain.

/// SplitMix64 (Steele, Lea, Flood, 2014).
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (rejection sampling, no modulo bias).
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "bound must be positive");
        let zone = u64::MAX - (u64::MAX % bound);
        loop {
            let value = self.next_u64();
            if value < zone {
                return value % bound;
            }
        }
    }
}

/// `k` distinct indices out of `0..n`, in the order they were drawn. When
/// `k >= n` the result is the whole range in ascending order: a sample of
/// everything does not depend on the seed.
pub fn sample(n: usize, k: usize, seed: u64) -> Vec<usize> {
    if k >= n {
        return (0..n).collect();
    }
    let mut rng = SplitMix64::new(seed);
    let mut pool: Vec<usize> = (0..n).collect();
    for i in 0..k {
        let j = i + rng.below((n - i) as u64) as usize;
        pool.swap(i, j);
    }
    pool.truncate(k);
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_matches_the_reference_values() {
        // Reference output for seed 1234567 from the public-domain
        // implementation by Sebastiano Vigna (splitmix64.c).
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(rng.next_u64(), 6_457_827_717_110_365_317);
        assert_eq!(rng.next_u64(), 3_203_168_211_198_807_973);
        assert_eq!(rng.next_u64(), 9_817_491_932_198_370_423);
    }

    #[test]
    fn sample_is_distinct_stable_and_depends_on_the_seed() {
        let a = sample(1000, 50, 7);
        let b = sample(1000, 50, 7);
        let c = sample(1000, 50, 8);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 50);
        let mut sorted = a.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 50);
        assert!(sorted.iter().all(|&i| i < 1000));
    }

    #[test]
    fn sample_of_everything_is_the_whole_range() {
        assert_eq!(sample(4, 4, 1), vec![0, 1, 2, 3]);
        assert_eq!(sample(4, 9, 2), vec![0, 1, 2, 3]);
        assert_eq!(sample(0, 3, 2), Vec::<usize>::new());
    }
}
