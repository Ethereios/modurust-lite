pub mod extra_nodes;
pub mod graph;

pub trait DspNode: Send {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

pub struct OscillatorNode {
    pub waveform: Waveform,
    pub freq: f32,
    pub phase: f32,
    pub sample_rate: f32,
}

impl OscillatorNode {
    pub fn new(waveform: Waveform, freq: f32, sample_rate: f32) -> Self {
        Self {
            waveform,
            freq,
            phase: 0.0,
            sample_rate,
        }
    }
}

impl DspNode for OscillatorNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [f32]) {
        let phase_inc = self.freq / self.sample_rate;
        for out in outputs.iter_mut() {
            *out = match self.waveform {
                Waveform::Sine => (self.phase * 2.0 * std::f32::consts::PI).sin(),
                Waveform::Saw => 2.0 * self.phase - 1.0,
                Waveform::Square => if self.phase < 0.5 { 1.0 } else { -1.0 },
                Waveform::Triangle => 2.0 * (2.0 * (self.phase - (self.phase + 0.5).floor())).abs() - 1.0,
            };
            self.phase = (self.phase + phase_inc) % 1.0;
        }
    }
}

/// Filter modes supported by the biquad (RBJ cookbook).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

pub struct BiquadFilterNode {
    pub cutoff: f32,
    pub resonance: f32,
    pub mode: FilterMode,
    pub sample_rate: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadFilterNode {
    pub fn new(cutoff: f32, resonance: f32, sample_rate: f32) -> Self {
        Self {
            cutoff,
            resonance,
            mode: FilterMode::LowPass,
            sample_rate,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }

    pub fn set_mode(&mut self, mode: FilterMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> FilterMode {
        self.mode
    }
}

impl DspNode for BiquadFilterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        // Clamp cutoff to avoid NaNs.
        let cutoff = self.cutoff.clamp(20.0, self.sample_rate * 0.45);
        let w0 = 2.0 * std::f32::consts::PI * cutoff / self.sample_rate;
        let alpha = w0.sin() / (2.0 * self.resonance.max(0.1));
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2) = match self.mode {
            FilterMode::LowPass => {
                let b0 = (1.0 - cos_w0) / 2.0;
                let b1 = 1.0 - cos_w0;
                let b2 = (1.0 - cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterMode::HighPass => {
                let b0 = (1.0 + cos_w0) / 2.0;
                let b1 = -(1.0 + cos_w0);
                let b2 = (1.0 + cos_w0) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterMode::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterMode::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            let y = (b0/a0)*x + (b1/a0)*self.x1 + (b2/a0)*self.x2 - (a1/a0)*self.y1 - (a2/a0)*self.y2;
            self.x2 = self.x1;
            self.x1 = x;
            self.y2 = self.y1;
            self.y1 = y;
            outputs[i] = y;
        }
    }
}

/// Noise generator node (white / pink / brown).
pub struct NoiseNode {
    pub color: NoiseColor,
    pub sample_rate: f32,
    // Pink & brown filter state.
    b0: f32, b1: f32, b2: f32,
    last_out: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseColor {
    White,
    Pink,
    Brown,
}

impl NoiseNode {
    pub fn new(color: NoiseColor, sample_rate: f32) -> Self {
        Self { color, sample_rate, b0: 0.0, b1: 0.0, b2: 0.0, last_out: 0.0 }
    }

    pub fn set_color(&mut self, color: NoiseColor) {
        self.color = color;
    }
}

impl DspNode for NoiseNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [f32]) {
        // Deterministic LCG seeded per call; cheap pseudo-random source.
        let mut state = self.sample_rate as u64 as u32;
        for out in outputs.iter_mut() {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let white = (state >> 8) as f32 / 8388607.0 * 2.0 - 1.0;
            match self.color {
                NoiseColor::White => *out = white,
                NoiseColor::Pink => {
                    // Paul Kellet's economical pink noise approximation.
                    self.b0 = 0.99886 * self.b0 + white * 0.0555179;
                    self.b1 = 0.99332 * self.b1 + white * 0.0750759;
                    self.b2 = 0.96900 * self.b2 + white * 0.1538520;
                    let pink = (self.b0 + self.b1 + self.b2 + white * 0.5362) * 0.11;
                    *out = pink;
                }
                NoiseColor::Brown => {
                    let brown = (self.last_out + 0.02 * white) / 1.02;
                    self.last_out = brown;
                    *out = brown * 3.5;
                }
            }
        }
    }
}

/// Simple gain / volume node.
pub struct GainNode {
    pub gain: f32,
}

impl GainNode {
    pub fn new(gain: f32) -> Self {
        Self { gain }
    }
}

impl DspNode for GainNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            outputs[i] = x * self.gain;
        }
    }
}

/// Tanh soft-clipping distortion / saturation node.
pub struct DistortionNode {
    pub drive: f32,
    pub tone: f32,
    x1: f32,
}

impl DistortionNode {
    pub fn new(drive: f32, tone: f32) -> Self {
        Self { drive: drive.max(1.0), tone: tone.clamp(0.0, 1.0), x1: 0.0 }
    }
}

impl DspNode for DistortionNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            let driven = x * self.drive;
            let shaped = driven.tanh();
            // Simple one-pole lowpass tone control.
            let filtered = self.x1 + self.tone * (shaped - self.x1);
            self.x1 = filtered;
            outputs[i] = filtered;
        }
    }
}

pub struct AdsrEnvelopeNode {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub sample_rate: f32,
    pub stage: EnvelopeStage,
    pub current_level: f32,
}

pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl AdsrEnvelopeNode {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32, sample_rate: f32) -> Self {
        Self {
            attack, decay, sustain, release, sample_rate,
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
        }
    }

    pub fn trigger_gate(&mut self, gate: bool) {
        if gate {
            self.stage = EnvelopeStage::Attack;
        } else if matches!(self.stage, EnvelopeStage::Attack | EnvelopeStage::Decay | EnvelopeStage::Sustain) {
            self.stage = EnvelopeStage::Release;
        }
    }
}

impl DspNode for AdsrEnvelopeNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        for (i, &in_val) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            match self.stage {
                EnvelopeStage::Idle => {
                    self.current_level = 0.0;
                }
                EnvelopeStage::Attack => {
                    let rate = 1.0 / (self.attack * self.sample_rate).max(1.0);
                    self.current_level += rate;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.stage = EnvelopeStage::Decay;
                    }
                }
                EnvelopeStage::Decay => {
                    let rate = (1.0 - self.sustain) / (self.decay * self.sample_rate).max(1.0);
                    self.current_level -= rate;
                    if self.current_level <= self.sustain {
                        self.current_level = self.sustain;
                        self.stage = EnvelopeStage::Sustain;
                    }
                }
                EnvelopeStage::Sustain => {
                    self.current_level = self.sustain;
                }
                EnvelopeStage::Release => {
                    let rate = self.sustain / (self.release * self.sample_rate).max(1.0);
                    self.current_level -= rate;
                    if self.current_level <= 0.0 {
                        self.current_level = 0.0;
                        self.stage = EnvelopeStage::Idle;
                    }
                }
            }
            outputs[i] = in_val * self.current_level;
        }
    }
}

pub struct SamplerNode {
    pub buffer: Vec<f32>,
    pub position: f32,
    pub playback_rate: f32,
    pub is_playing: bool,
}

impl SamplerNode {
    pub fn new(buffer: Vec<f32>) -> Self {
        Self {
            buffer,
            position: 0.0,
            playback_rate: 1.0,
            is_playing: false,
        }
    }

    pub fn trigger(&mut self) {
        self.position = 0.0;
        self.is_playing = true;
    }
}

impl DspNode for SamplerNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [f32]) {
        if !self.is_playing || self.buffer.is_empty() {
            outputs.fill(0.0);
            return;
        }
        for out in outputs.iter_mut() {
            let idx = self.position as usize;
            if idx < self.buffer.len() {
                *out = self.buffer[idx];
                self.position += self.playback_rate;
            } else {
                *out = 0.0;
                self.is_playing = false;
                break;
            }
        }
    }
}

pub struct SimpleDelayNode {
    buffer: Vec<f32>,
    write_pos: usize,
    pub delay_samples: usize,
    pub feedback: f32,
}

impl SimpleDelayNode {
    pub fn new(max_delay_samples: usize, delay_samples: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
            delay_samples,
            feedback,
        }
    }

    /// Max delay buffer length in samples.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl DspNode for SimpleDelayNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        let buf_len = self.buffer.len();
        for (i, &in_val) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            let read_pos = (self.write_pos + buf_len - self.delay_samples) % buf_len;
            let delayed = self.buffer[read_pos];
            self.buffer[self.write_pos] = in_val + delayed * self.feedback;
            outputs[i] = in_val + delayed;
            self.write_pos = (self.write_pos + 1) % buf_len;
        }
    }
}
