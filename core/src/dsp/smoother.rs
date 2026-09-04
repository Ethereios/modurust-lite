/// A one-pole parameter smoother that smooths parameter changes to prevent
/// zipper noise and clicks when parameters are updated in real-time.
///
/// The smoother is initialized with a `time_constant` (in seconds) that
/// determines how quickly the smoothed value approaches the target.
pub struct ParameterSmoother {
    current: f32,
    target: f32,
    time_constant: f32,
    sample_rate: f32,
}

impl ParameterSmoother {
    /// Create a new parameter smoother.
    ///
    /// `time_constant` is the time in seconds for the smoother to reach
    /// ~63% of the way to a step change. Typical values are 0.001-0.05s
    /// (1ms to 50ms) for audio parameters.
    pub fn new(initial: f32, time_constant: f32, sample_rate: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            time_constant: time_constant.max(0.0),
            sample_rate: sample_rate.max(1.0),
        }
    }

    /// Set the target value (the value being smoothed towards).
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Get the current smoothed value.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Get the target value.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Set both current and target to the same value (no smoothing).
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    /// Process one sample: advances the smoother by one step and returns
    /// the current value.
    pub fn process(&mut self) -> f32 {
        if self.time_constant <= 0.0 {
            self.current = self.target;
            return self.current;
        }

        let coefficient = 1.0 - (-1.0 / (self.time_constant * self.sample_rate)).exp();
        self.current += (self.target - self.current) * coefficient;
        self.current
    }

    /// Process a block of samples, filling `output` with smoothed values.
    pub fn process_block(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.process();
        }
    }

    /// Reset the smoother to a new value immediately.
    pub fn reset(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoother_initial() {
        let s = ParameterSmoother::new(0.5, 0.01, 44100.0);
        assert_eq!(s.current(), 0.5);
        assert_eq!(s.target(), 0.5);
    }

    #[test]
    fn test_smoother_set_target() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_target(1.0);
        assert_eq!(s.target(), 1.0);
        assert_eq!(s.current(), 0.0);
    }

    #[test]
    fn test_smoother_smooths_to_target() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_target(1.0);

        let mut max = 0.0;
        for _ in 0..44100 {
            let v = s.process();
            if v > max {
                max = v;
            }
        }
        assert!(max > 0.99, "should reach near target, got {}", max);
    }

    #[test]
    fn test_smoother_no_zipper_noise() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_target(1.0);

        let mut prev = s.process();
        let mut max_delta: f32 = 0.0;
        for _ in 0..4410 {
            let v = s.process();
            let delta = (v - prev).abs();
            if delta > max_delta {
                max_delta = delta;
            }
            prev = v;
        }
        assert!(max_delta < 0.01, "max delta should be small for smooth ramp, got {}", max_delta);
    }

    #[test]
    fn test_smoother_immediate() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_immediate(1.0);
        assert_eq!(s.current(), 1.0);
        assert_eq!(s.target(), 1.0);
    }

    #[test]
    fn test_smoother_reset() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_target(1.0);
        for _ in 0..100 {
            s.process();
        }
        s.reset(0.5);
        assert_eq!(s.current(), 0.5);
        assert_eq!(s.target(), 0.5);
    }

    #[test]
    fn test_smoother_process_block() {
        let mut s = ParameterSmoother::new(0.0, 0.01, 44100.0);
        s.set_target(1.0);
        let mut output = vec![0.0; 100];
        s.process_block(&mut output);
        assert!(output[0] > 0.0);
        assert!(output[99] > output[0]);
    }
}
