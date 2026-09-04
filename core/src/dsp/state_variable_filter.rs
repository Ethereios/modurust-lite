/// State-variable filter (4-mode simultaneous output).
///
/// Based on the classic analog state-variable topology that produces
/// low-pass, high-pass, band-pass, and notch outputs simultaneously from
/// a single input. This is ideal for sound design where you want to
/// morph between filter types or use multiple outputs in parallel.
///
/// Reference: Chamberlin, Charles. "Musical Applications of Microprocessors"
pub struct StateVariableFilter {
    pub cutoff: f32,
    pub resonance: f32,
    pub sample_rate: f32,
    // State variables for the filter
    lp: f32, // low-pass output
    bp: f32, // band-pass output
    hp: f32, // high-pass output
    // Previous inputs for the state update
    x1: f32,
    x2: f32,
}

impl StateVariableFilter {
    pub fn new(cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            cutoff: cutoff.clamp(20.0, 20000.0),
            resonance: resonance.clamp(0.0, 1.0),
            sample_rate: sample_rate.max(1.0),
            lp: 0.0,
            bp: 0.0,
            hp: 0.0,
            x1: 0.0,
            x2: 0.0,
        }
    }

    /// Process a single sample through the filter.
    /// Returns (low_pass, band_pass, high_pass, notch) outputs.
    pub fn process_sample(&mut self, input: f32) -> (f32, f32, f32, f32) {
        // Frequency and damping
        let f = 2.0 * (std::f32::consts::PI * self.cutoff / self.sample_rate).sin();
        let d = 1.0 / (2.0 * (self.resonance.max(0.1) * 2.0).max(0.1));

        // State-variable update (Chamberlin style)
        let hp = input - d * self.bp - self.lp;
        self.bp = self.bp + f * hp;
        self.lp = self.lp + f * self.bp;
        self.hp = hp;

        // Notch = LP + HP
        let notch = self.lp + self.hp;

        (self.lp, self.bp, self.hp, notch)
    }

    /// Get the current low-pass output.
    pub fn low_pass(&self) -> f32 {
        self.lp
    }

    /// Get the current band-pass output.
    pub fn band_pass(&self) -> f32 {
        self.bp
    }

    /// Get the current high-pass output.
    pub fn high_pass(&self) -> f32 {
        self.hp
    }

    /// Get the current notch output.
    pub fn notch(&self) -> f32 {
        self.lp + self.hp
    }
}

/// A DSP node that wraps the state-variable filter.
///
/// The node takes a single input and produces a single output based on
/// the `output_mode` parameter. The four simultaneous outputs are mixed
/// according to the mode weights to produce a single output signal.
pub struct StateVariableFilterNode {
    filter: StateVariableFilter,
    /// Mix weights for the four outputs: (lp, bp, hp, notch)
    pub mix: [f32; 4],
}

impl StateVariableFilterNode {
    pub fn new(cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            filter: StateVariableFilter::new(cutoff, resonance, sample_rate),
            mix: [1.0, 0.0, 0.0, 0.0], // Default to low-pass
        }
    }

    /// Set the mix weights for the four filter outputs.
    /// `mix[0]` = low-pass, `mix[1]` = band-pass, `mix[2]` = high-pass, `mix[3]` = notch
    pub fn set_mix(&mut self, mix: [f32; 4]) {
        self.mix = mix;
    }

    /// Set the mix from normalized 0..=1 values (each independently).
    pub fn set_mix_normalized(&mut self, lp: f32, bp: f32, hp: f32, notch: f32) {
        self.mix = [lp.clamp(0.0, 1.0), bp.clamp(0.0, 1.0), hp.clamp(0.0, 1.0), notch.clamp(0.0, 1.0)];
    }
}

impl crate::dsp::DspNode for StateVariableFilterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() {
                break;
            }
            let (lp, bp, hp, notch) = self.filter.process_sample(x);
            outputs[i] = lp * self.mix[0] + bp * self.mix[1] + hp * self.mix[2] + notch * self.mix[3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_variable_filter_basic() {
        let mut f = StateVariableFilter::new(1000.0, 0.5, 44100.0);
        let (lp, bp, hp, notch) = f.process_sample(1.0);
        // All outputs should be finite
        assert!(lp.is_finite());
        assert!(bp.is_finite());
        assert!(hp.is_finite());
        assert!(notch.is_finite());
    }

    #[test]
    fn test_state_variable_filter_lowpass() {
        let mut f = StateVariableFilter::new(1000.0, 0.7, 44100.0);
        // Feed a constant signal - low-pass should pass it
        for _ in 0..1000 {
            f.process_sample(1.0);
        }
        let lp = f.low_pass();
        assert!(lp > 0.5, "low-pass should pass DC, got {}", lp);
    }

    #[test]
    fn test_state_variable_filter_highpass() {
        let mut f = StateVariableFilter::new(1000.0, 0.7, 44100.0);
        // Feed a constant signal - high-pass should reject DC
        for _ in 0..1000 {
            f.process_sample(1.0);
        }
        let hp = f.high_pass();
        assert!(hp.abs() < 0.1, "high-pass should reject DC, got {}", hp);
    }

    #[test]
    fn test_state_variable_filter_node() {
        let mut node = StateVariableFilterNode::new(1000.0, 0.5, 44100.0);
        let input = vec![1.0; 100];
        let mut output = vec![0.0; 100];
        node.process(&input, &mut output);
        assert!(output.iter().any(|&v| v.is_finite()));
    }
}