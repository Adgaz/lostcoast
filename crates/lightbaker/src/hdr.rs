pub struct Histogram {
    pub bins: [u32; 256],
    pub min_log: f32,
    pub max_log: f32,
}

impl Histogram {
    pub fn new(min_log: f32, max_log: f32) -> Self {
        Self {
            bins: [0; 256],
            min_log,
            max_log,
        }
    }
    pub fn add(&mut self, lum: f32) {
        if lum <= 0.0 {
            self.bins[0] = self.bins[0].saturating_add(1);
            return;
        }
        let l = lum.log2();
        let t = (l - self.min_log) / (self.max_log - self.min_log);
        let bin = (t * 255.0).clamp(0.0, 255.0) as usize;
        self.bins[bin] = self.bins[bin].saturating_add(1);
    }
    pub fn average_log(&self) -> f32 {
        let mut total: u64 = 0;
        let mut weighted: f64 = 0.0;
        for (i, &c) in self.bins.iter().enumerate() {
            total += c as u64;
            let t = i as f64 / 255.0;
            let log_lum = self.min_log as f64 + t * (self.max_log - self.min_log) as f64;
            weighted += log_lum * c as f64;
        }
        if total == 0 {
            return (self.min_log + self.max_log) * 0.5;
        }
        (weighted / total as f64) as f32
    }
}

pub struct AutoExposure {
    pub current_log: f32,
    pub time_constant: f32,
    pub key: f32,
}

impl AutoExposure {
    pub fn new(initial_log: f32, time_constant: f32, key: f32) -> Self {
        Self {
            current_log: initial_log,
            time_constant,
            key,
        }
    }
    pub fn update(&mut self, target_log: f32, dt: f32) -> f32 {
        let alpha = 1.0 - (-dt / self.time_constant.max(1e-3)).exp();
        self.current_log += (target_log - self.current_log) * alpha;
        let avg = 2.0_f32.powf(self.current_log);
        self.key / avg.max(1e-6)
    }
}

pub fn step_response_frames(
    initial_lum: f32,
    target_lum: f32,
    time_constant: f32,
    dt: f32,
    tolerance: f32,
) -> u32 {
    let mut exp = AutoExposure::new(initial_lum.log2(), time_constant, 1.0);
    let target_log = target_lum.log2();
    for i in 0..1024 {
        exp.update(target_log, dt);
        if (exp.current_log - target_log).abs() < tolerance {
            return i + 1;
        }
    }
    u32::MAX
}
