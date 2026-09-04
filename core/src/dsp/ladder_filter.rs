/// Ladder filter (Moog-style resonant low-pass filter).
///
/// Implementation based on the classic 24dB/octave Moog transistor ladder
/// filter topology. This is a non-linear filter that produces the characteristic
/// "Moog" sound when driven into resonance.
///
/// Reference: Zölzer, Udo. "DAFX: Digital Audio Effects" (Moog VCF section)
/// and Vadim Zavalishin's "The Art of VA Filter Design"
pub struct LadderFilter {
    cutoff: f32,
    resonance: f32,
    sample_rate: f32,
    // The four cascaded one-pole sections
    t1: f32,
    t2: f32,
    t3: f32,
    t4: f32,
    // Non-linear tanh stages for the ladder
    v1: f32,
    v2: f32,
    v3: f32,
    v4: f32,
}

impl LadderFilter {
    pub fn new(cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            cutoff: cutoff.clamp(20.0, 20000.0),
            resonance: resonance.clamp(0.0, 1.0),
            sample_rate: sample_rate.max(1.0),
            t1: 0.0,
            t2: 0.0,
            t3: 0.0,
            t4: 0.0,
            v1: 0.0,
            v2: 0.0,
            v3: 0.0,
            v4: 0.0,
        }
    }

    /// Process a single sample through the ladder filter.
    pub fn process_sample(&mut self, input: f32) -> f32 {
        // Moog ladder coefficients
        let f = 2.0 * (std::f32::consts::PI * self.cutoff / self.sample_rate).min(1.0);
        let r = self.resonance;
        let fb = r * (1.0 - 0.15 * f * f); // Feedback with nonlinearity compensation

        // Input with feedback
        let vin = input - fb * self.t4;

        // Non-linear stages (tanh approximation)
        self.t1 = (vin + self.t1 * 0.8).tanh();
        self.t2 = (self.t1 + self.t2 * 0.8).tanh();
        self.t3 = (self.t2 + self.t3 * 0.8).tanh();
        self.t4 = (self.t3 + self.t4 * 0.8).tanh();

        self.t4
    }
}

impl crate::dsp::DspNode for LadderFilter {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() {
                break;
            }
            outputs[i] = self.process_sample(x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ladder_filter_basic() {
        let mut f = LadderFilter::new(1000.0, 0.5, 44100.0);
        let out = f.process_sample(1.0);
        assert!(out.is_finite());
    }

    #[test]
    fn test_ladder_filter_lowpass() {
        let mut f = LadderFilter::new(500.0, 0.3, 44100.0);
        // DC signal should pass through
        for _ in 0..1000 {
            f.process_sample(1.0);
        }
        // Output should be near input for low frequencies
        let out = f.process_sample(1.0);
        assert!(out > 0.5, "should pass low frequencies, got {}", out);
    }

    #[test]
    fn test_ladder_filter_highfreq_attenuation() {
        let mut f = LadderFilter::new(200.0, 0.3, 44100.0);
        // Very high frequency should be attenuated
        let mut out = 0.0;
        for i in 0..100 {
            out = f.process_sample((i as f32 * 20.0).sin()); // High freq sine
        }
        assert!(out.abs() < 0.5, "should attenuate high frequencies, got {}", out);
    }

    #[test]
    fn test_ladder_filter_resonance() {
        let mut f_low_q = LadderFilter::new(500.0, 0.1, 44100.0);
        let mut f_high_q = LadderFilter::new(500.0, 0.9, 44100.0);

        // Ringing test with impulse
        let mut ringing_low = 0.0;
        let mut ringing_high = 0.0;

        for i in 0..100 {
            let impulse = if i == 0 { 1.0 } else { 0.0 };
            ringing_low += f_low_q.process_sample(impulse).abs();
            ringing_high += f_high_q.process_sample(impulse).abs();
        }

        // Higher resonance should ring more
        assert!(ringing_high > ringing_low, "higher Q should ring more");
    }
}