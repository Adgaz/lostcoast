use std::collections::VecDeque;

pub struct Stats {
    samples: VecDeque<f32>,
    capacity: usize,
}

impl Stats {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(dt);
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    pub fn avg_ms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.samples.iter().sum();
        sum / self.samples.len() as f32 * 1000.0
    }

    pub fn min_ms(&self) -> f32 {
        self.samples.iter().copied().fold(f32::INFINITY, f32::min) * 1000.0
    }

    pub fn max_ms(&self) -> f32 {
        self.samples.iter().copied().fold(0.0_f32, f32::max) * 1000.0
    }

    pub fn fps(&self) -> f32 {
        let avg = self.avg_ms();
        if avg <= 0.0 {
            0.0
        } else {
            1000.0 / avg
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Stats;

    #[test]
    fn tick_caps_capacity() {
        let mut s = Stats::new(3);
        for _ in 0..5 {
            s.tick(0.016);
        }
        assert_eq!(s.count(), 3);
    }

    #[test]
    fn avg_ms_matches() {
        let mut s = Stats::new(4);
        s.tick(0.010);
        s.tick(0.020);
        s.tick(0.030);
        s.tick(0.040);
        assert!((s.avg_ms() - 25.0).abs() < 1e-3);
    }
}
