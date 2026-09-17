pub struct Random {
    state: u64,
}

impl Random {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn range(&mut self, upper_bound: usize) -> usize {
        if upper_bound == 0 {
            return 0;
        }
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.state >> 32) as usize) % upper_bound
    }

    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.range(index + 1);
            values.swap(index, other);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_seeds_produce_equal_sequences() {
        let mut first = Random::new(1234);
        let mut second = Random::new(1234);

        for _ in 0..20 {
            assert_eq!(first.range(7), second.range(7));
        }
    }

    #[test]
    fn generated_values_stay_inside_the_requested_range() {
        let mut random = Random::new(9876);
        for _ in 0..100 {
            assert!(random.range(7) < 7);
        }
    }
}
