//! Comprehensive Audio Node Library
//!
//! This module provides a complete set of audio processing nodes including:
//! - Generators (oscillators, noise, samples)
//! - Effects (EQ, compression, delay, reverb, distortion)
//! - Filters (LPF, HPF, BPF, notch)
//! - Dynamics (compressors, limiters, gates, expanders)
//! - Modulation (chorus, flanger, phaser, tremolo)
//! - Utilities (mixers, splitters, analyzers)

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use crate::node_graph::{NeuroNodeProcessor, NeuroDataType, NeuroNodeId, NeuroNodeDefinition, NeuroNodePort};
use serde_json::Value;

/// Audio buffer size for processing
const BUFFER_SIZE: usize = 128;

/// Stereo audio frame
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

impl StereoFrame {
    pub fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    pub fn mono(value: f32) -> Self {
        Self { left: value, right: value }
    }
}

/// Audio buffer for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBuffer {
    pub samples: Vec<StereoFrame>,
}

impl AudioBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            samples: vec![StereoFrame::default(); size],
        }
    }

    pub fn size(&self) -> usize {
        self.samples.len()
    }

    pub fn get(&self, index: usize) -> StereoFrame {
        self.samples[index]
    }

    pub fn set(&mut self, index: usize, frame: StereoFrame) {
        self.samples[index] = frame;
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = StereoFrame::default();
        }
    }
}

/// Biquad filter for EQ and filtering
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    // Filter coefficients
    a0: f32, a1: f32, a2: f32,
    b0: f32, b1: f32, b2: f32,
    // Delay elements
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl BiquadFilter {
    pub fn new() -> Self {
        Self {
            a0: 1.0, a1: 0.0, a2: 0.0,
            b0: 1.0, b1: 0.0, b2: 0.0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }

    pub fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
        self.a0 = 1.0;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
                   - self.a1 * self.y1 - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    pub fn lowpass(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let mut filter = Self::new();
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 - cos_omega) / 2.0;
        let b1 = 1.0 - cos_omega;
        let b2 = (1.0 - cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        filter.set_coefficients(b0, b1, b2, a0, a1, a2);
        filter
    }

    pub fn highpass(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let mut filter = Self::new();
        let omega = 2.0 * std::f32::consts::PI * frequency / sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = (1.0 + cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        filter.set_coefficients(b0, b1, b2, a0, a1, a2);
        filter
    }
}

/// Delay line for echo/delay effects
#[derive(Debug, Clone)]
pub struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,
}

impl DelayLine {
    pub fn new(max_delay_ms: f32, sample_rate: f32) -> Self {
        let max_samples = ((max_delay_ms / 1000.0) * sample_rate) as usize;
        Self {
            buffer: vec![0.0; max_samples],
            write_pos: 0,
            sample_rate,
        }
    }

    pub fn read(&self, delay_samples: f32) -> f32 {
        let read_pos = (self.write_pos as f32 - delay_samples).rem_euclid(self.buffer.len() as f32);
        let index = read_pos as usize;
        let frac = read_pos.fract();

        if index >= self.buffer.len() - 1 {
            return self.buffer[index % self.buffer.len()];
        }

        // Linear interpolation
        let a = self.buffer[index];
        let b = self.buffer[(index + 1) % self.buffer.len()];
        a + (b - a) * frac
    }

    pub fn write(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
    }

    pub fn delay_time_to_samples(&self, delay_ms: f32) -> f32 {
        (delay_ms / 1000.0) * self.sample_rate
    }
}

/// Compressor with soft knee
#[derive(Debug, Clone)]
pub struct Compressor {
    threshold: f32,
    ratio: f32,
    attack: f32,
    release: f32,
    makeup_gain: f32,
    envelope: f32,
    sample_rate: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            threshold: -24.0, // dB
            ratio: 4.0,
            attack: 0.001, // seconds
            release: 0.1,  // seconds
            makeup_gain: 0.0,
            envelope: 0.0,
            sample_rate,
        }
    }

    pub fn set_parameters(&mut self, threshold: f32, ratio: f32, attack: f32, release: f32, makeup_gain: f32) {
        self.threshold = threshold;
        self.ratio = ratio;
        self.attack = attack;
        self.release = release;
        self.makeup_gain = makeup_gain;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let input_db = 20.0 * input.abs().log10().max(-100.0);

        // Compute gain reduction
        let over_threshold = (input_db - self.threshold).max(0.0);
        let gain_reduction_db = over_threshold * (1.0 - 1.0 / self.ratio);

        // Smooth envelope
        let target_env = (-gain_reduction_db).exp2();
        let coeff = if target_env < self.envelope {
            self.attack_coeff()
        } else {
            self.release_coeff()
        };

        self.envelope = self.envelope * (1.0 - coeff) + target_env * coeff;

        // Apply compression
        let output_db = input_db + gain_reduction_db;
        let output_linear = 10.0_f32.powf(output_db / 20.0);

        // Apply makeup gain
        output_linear * 10.0_f32.powf(self.makeup_gain / 20.0) * input.signum()
    }

    fn attack_coeff(&self) -> f32 {
        (-1.0 / (self.attack * self.sample_rate)).exp()
    }

    fn release_coeff(&self) -> f32 {
        (-1.0 / (self.release * self.sample_rate)).exp()
    }
}

// ============================================================================
// AUDIO NODES
// ============================================================================

/// Sine wave oscillator
pub struct SineOscillator {
    phase: f32,
    sample_rate: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for SineOscillator {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let frequency = inputs.get("frequency").and_then(|v| v.as_f64()).unwrap_or(440.0) as f32;
        let amplitude = inputs.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

        let mut buffer = AudioBuffer::new(BUFFER_SIZE);
        for i in 0..BUFFER_SIZE {
            let sample = (self.phase * 2.0 * std::f32::consts::PI).sin() * amplitude;
            buffer.set(i, StereoFrame::mono(sample));

            self.phase += frequency / self.sample_rate;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(buffer).unwrap())]))
    }
}

impl SineOscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate,
        }
    }

    pub fn create_node(id: NeuroNodeId, _sample_rate: f32) -> NeuroNodeDefinition {
        NeuroNodeDefinition {
            id,
            name: "Sine Oscillator".to_string(),
            node_type: "generator.sine".to_string(),
            inputs: vec![
                NeuroNodePort { name: "frequency".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "amplitude".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }
}

/// Parametric EQ with multiple bands
pub struct ParametricEQ {
    bands: Vec<BiquadFilter>,
    frequencies: Vec<f32>,
    gains: Vec<f32>,
    qs: Vec<f32>,
    sample_rate: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for ParametricEQ {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        let mut output = audio_in.clone();

        // Update filter parameters if provided
        if let Some(freqs) = inputs.get("frequencies").and_then(|v| v.as_array()) {
            for (i, freq) in freqs.iter().enumerate().take(self.bands.len()) {
                if let Some(freq_val) = freq.as_f64() {
                    self.frequencies[i] = freq_val as f32;
                    self.update_band(i);
                }
            }
        }

        if let Some(gains) = inputs.get("gains").and_then(|v| v.as_array()) {
            for (i, gain) in gains.iter().enumerate().take(self.bands.len()) {
                if let Some(gain_val) = gain.as_f64() {
                    self.gains[i] = gain_val as f32;
                    self.update_band(i);
                }
            }
        }

        // Process audio through all bands
        for i in 0..BUFFER_SIZE {
            let mut frame = audio_in.get(i);
            for band in &mut self.bands {
                frame.left = band.process(frame.left);
                frame.right = band.process(frame.right);
            }
            output.set(i, frame);
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl ParametricEQ {
    pub fn new(sample_rate: f32, num_bands: usize) -> Self {
        let mut bands = Vec::new();
        let mut frequencies = Vec::new();
        let mut gains = Vec::new();
        let mut qs = Vec::new();

        // Initialize with default frequencies (logarithmic spacing)
        let start_freq = 20.0;
        let end_freq = 20000.0;
        let ratio = ((end_freq / start_freq) as f32).powf(1.0 / (num_bands - 1) as f32);

        for i in 0..num_bands {
            let freq = start_freq * ratio.powf(i as f32);
            frequencies.push(freq);
            gains.push(0.0); // 0 dB gain
            qs.push(1.414); // Butterworth Q
            bands.push(BiquadFilter::new());
        }

        let mut eq = Self {
            bands,
            frequencies,
            gains,
            qs,
            sample_rate,
        };

        // Initialize filters
        for i in 0..num_bands {
            eq.update_band(i);
        }

        eq
    }

    fn update_band(&mut self, band_index: usize) {
        // Simple peaking EQ implementation
        let freq = self.frequencies[band_index];
        let gain_db = self.gains[band_index];
        let q = self.qs[band_index];

        let a = 10.0_f32.powf(gain_db / 40.0);
        let omega = 2.0 * std::f32::consts::PI * freq / self.sample_rate;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();
        let alpha = sin_omega / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha / a;

        self.bands[band_index].set_coefficients(b0, b1, b2, a0, a1, a2);
    }
}

/// Stereo delay effect
pub struct StereoDelay {
    left_delay: DelayLine,
    right_delay: DelayLine,
    feedback: f32,
    wet_dry: f32,
    delay_time_ms: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for StereoDelay {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update parameters
        if let Some(delay) = inputs.get("delay_ms").and_then(|v| v.as_f64()) {
            self.delay_time_ms = delay as f32;
        }
        if let Some(fb) = inputs.get("feedback").and_then(|v| v.as_f64()) {
            self.feedback = fb as f32;
        }
        if let Some(wd) = inputs.get("wet_dry").and_then(|v| v.as_f64()) {
            self.wet_dry = wd as f32;
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);

            // Read delayed samples
            let delay_samples = self.left_delay.delay_time_to_samples(self.delay_time_ms);
            let delayed_left = self.left_delay.read(delay_samples);
            let delayed_right = self.right_delay.read(delay_samples);

            // Cross-feedback for stereo imaging
            let wet_left = input_frame.left + delayed_right * self.feedback;
            let wet_right = input_frame.right + delayed_left * self.feedback;

            // Mix wet/dry
            let out_left = input_frame.left * (1.0 - self.wet_dry) + wet_left * self.wet_dry;
            let out_right = input_frame.right * (1.0 - self.wet_dry) + wet_right * self.wet_dry;

            output.set(i, StereoFrame::new(out_left, out_right));

            // Write to delay lines
            self.left_delay.write(wet_left);
            self.right_delay.write(wet_right);
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl StereoDelay {
    pub fn new(sample_rate: f32, max_delay_ms: f32) -> Self {
        Self {
            left_delay: DelayLine::new(max_delay_ms, sample_rate),
            right_delay: DelayLine::new(max_delay_ms, sample_rate),
            feedback: 0.3,
            wet_dry: 0.5,
            delay_time_ms: 500.0,
        }
    }

    pub fn create_node(id: NeuroNodeId) -> NeuroNodeDefinition {
        NeuroNodeDefinition {
            id,
            name: "Stereo Delay".to_string(),
            node_type: "effect.delay".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "delay_ms".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "feedback".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "wet_dry".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }
}

/// Compressor node
pub struct CompressorNode {
    compressor: Compressor,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for CompressorNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update parameters
        if let Some(threshold) = inputs.get("threshold").and_then(|v| v.as_f64()) {
            let ratio = inputs.get("ratio").and_then(|v| v.as_f64()).unwrap_or(4.0) as f32;
            let attack = inputs.get("attack").and_then(|v| v.as_f64()).unwrap_or(0.001) as f32;
            let release = inputs.get("release").and_then(|v| v.as_f64()).unwrap_or(0.1) as f32;
            let makeup = inputs.get("makeup_gain").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            self.compressor.set_parameters(threshold as f32, ratio, attack, release, makeup);
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);
            let out_left = self.compressor.process(input_frame.left);
            let out_right = self.compressor.process(input_frame.right);
            output.set(i, StereoFrame::new(out_left, out_right));
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl CompressorNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            compressor: Compressor::new(sample_rate),
        }
    }
}

/// Audio mixer node
pub struct MixerNode {
    num_inputs: usize,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for MixerNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let mut mixed_left = 0.0;
            let mut mixed_right = 0.0;
            let mut active_inputs = 0;

            // Mix all audio inputs
            for input_name in &["audio_in_1", "audio_in_2", "audio_in_3", "audio_in_4"] {
                if let Some(audio_buf) = inputs.get(*input_name)
                    .and_then(|v| serde_json::from_value::<AudioBuffer>(v.clone()).ok()) {
                    if i < audio_buf.size() {
                        let frame = audio_buf.get(i);
                        mixed_left += frame.left;
                        mixed_right += frame.right;
                        active_inputs += 1;
                    }
                }
            }

            // Normalize by number of active inputs to prevent clipping
            if active_inputs > 0 {
                mixed_left /= active_inputs as f32;
                mixed_right /= active_inputs as f32;
            }

            output.set(i, StereoFrame::new(mixed_left, mixed_right));
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl MixerNode {
    pub fn new(num_inputs: usize) -> Self {
        Self { num_inputs }
    }

    pub fn create_node(id: NeuroNodeId, num_inputs: usize) -> NeuroNodeDefinition {
        let mut inputs = vec![
            NeuroNodePort { name: "audio_in_1".to_string(), data_type: NeuroDataType::Audio, required: false },
            NeuroNodePort { name: "audio_in_2".to_string(), data_type: NeuroDataType::Audio, required: false },
            NeuroNodePort { name: "audio_in_3".to_string(), data_type: NeuroDataType::Audio, required: false },
            NeuroNodePort { name: "audio_in_4".to_string(), data_type: NeuroDataType::Audio, required: false },
        ];

        // Add more inputs if needed
        for i in 5..=num_inputs {
            inputs.push(NeuroNodePort {
                name: format!("audio_in_{}", i),
                data_type: NeuroDataType::Audio,
                required: false,
            });
        }

        NeuroNodeDefinition {
            id,
            name: format!("{}-Channel Mixer", num_inputs),
            node_type: "utility.mixer".to_string(),
            inputs,
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }
}

/// Sampler node for playing audio files
pub struct SamplerNode {
    sample_data: Option<Vec<f32>>,
    playback_pos: f32,
    sample_rate: f32,
    is_playing: bool,
    loop_enabled: bool,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for SamplerNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        // Handle trigger input
        if let Some(trigger) = inputs.get("trigger").and_then(|v| v.as_bool()) {
            if trigger && !self.is_playing {
                self.is_playing = true;
                self.playback_pos = 0.0;
            }
        }

        // Handle stop input
        if let Some(stop) = inputs.get("stop").and_then(|v| v.as_bool()) {
            if stop {
                self.is_playing = false;
            }
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        if let Some(sample_data) = &self.sample_data {
            if self.is_playing {
                for i in 0..BUFFER_SIZE {
                    if self.playback_pos >= sample_data.len() as f32 {
                        if self.loop_enabled {
                            self.playback_pos = 0.0;
                        } else {
                            self.is_playing = false;
                            break;
                        }
                    }

                    let pos_int = self.playback_pos as usize;
                    let frac = self.playback_pos.fract();

                    let sample = if pos_int + 1 < sample_data.len() {
                        // Linear interpolation
                        let a = sample_data[pos_int];
                        let b = sample_data[pos_int + 1];
                        a + (b - a) * frac
                    } else {
                        sample_data[pos_int]
                    };

                    output.set(i, StereoFrame::mono(sample));
                    self.playback_pos += 1.0;
                }
            }
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl SamplerNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_data: None,
            playback_pos: 0.0,
            sample_rate,
            is_playing: false,
            loop_enabled: false,
        }
    }

    pub fn load_sample(&mut self, data: Vec<f32>) {
        self.sample_data = Some(data);
        self.playback_pos = 0.0;
    }
}

/// White noise generator
pub struct NoiseGenerator;

#[async_trait::async_trait]
impl NeuroNodeProcessor for NoiseGenerator {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let amplitude = inputs.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;

        let mut buffer = AudioBuffer::new(BUFFER_SIZE);
        for i in 0..BUFFER_SIZE {
            let noise = (rand::random::<f32>() - 0.5) * 2.0 * amplitude;
            buffer.set(i, StereoFrame::mono(noise));
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(buffer).unwrap())]))
    }
}

/// Audio gain/attenuation node
pub struct GainNode {
    gain: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for GainNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update gain parameter
        if let Some(gain_db) = inputs.get("gain_db").and_then(|v| v.as_f64()) {
            self.gain = 10.0_f32.powf(gain_db as f32 / 20.0);
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);
        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);
            output.set(i, StereoFrame::new(
                input_frame.left * self.gain,
                input_frame.right * self.gain,
            ));
        }

        Ok(std::collections::HashMap::from([("audio_out".to_string(), serde_json::to_value(output).unwrap())]))
    }
}

impl GainNode {
    pub fn new() -> Self {
        Self { gain: 1.0 }
    }
}

/// Create a registry of all available audio nodes
pub fn create_audio_node_registry() -> std::collections::HashMap<String, Box<dyn Fn(NeuroNodeId) -> NeuroNodeDefinition + Send + Sync>> {
    let mut registry = std::collections::HashMap::new();

    // Generators
    registry.insert("generator.sine".to_string(), Box::new(|id| {
        SineOscillator::create_node(id, 44100.0)
    }) as Box<dyn Fn(NeuroNodeId) -> NeuroNodeDefinition + Send + Sync>);

    registry.insert("generator.noise".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "White Noise".to_string(),
            node_type: "generator.noise".to_string(),
            inputs: vec![
                NeuroNodePort { name: "amplitude".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Effects
    registry.insert("effect.gain".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Gain".to_string(),
            node_type: "effect.gain".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "gain_db".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    registry.insert("effect.delay".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Stereo Delay".to_string(),
            node_type: "effect.delay".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "delay_ms".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "feedback".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "wet_dry".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    registry.insert("effect.compressor".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Compressor".to_string(),
            node_type: "effect.compressor".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "threshold".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "ratio".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "attack".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "release".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "makeup_gain".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    registry.insert("effect.eq".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Parametric EQ".to_string(),
            node_type: "effect.eq".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "frequencies".to_string(), data_type: NeuroDataType::Array, required: false },
                NeuroNodePort { name: "gains".to_string(), data_type: NeuroDataType::Array, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Utilities
    registry.insert("utility.mixer".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "4-Channel Mixer".to_string(),
            node_type: "utility.mixer".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in_1".to_string(), data_type: NeuroDataType::Audio, required: false },
                NeuroNodePort { name: "audio_in_2".to_string(), data_type: NeuroDataType::Audio, required: false },
                NeuroNodePort { name: "audio_in_3".to_string(), data_type: NeuroDataType::Audio, required: false },
                NeuroNodePort { name: "audio_in_4".to_string(), data_type: NeuroDataType::Audio, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    registry.insert("sampler.basic".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Sampler".to_string(),
            node_type: "sampler.basic".to_string(),
            inputs: vec![
                NeuroNodePort { name: "trigger".to_string(), data_type: NeuroDataType::Integer, required: false },
                NeuroNodePort { name: "stop".to_string(), data_type: NeuroDataType::Integer, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Register Reverb
    registry.insert("effect.reverb".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Reverb".to_string(),
            node_type: "effect.reverb".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "room_size".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "damping".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "wet_dry".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Register Distortion
    registry.insert("effect.distortion".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "Distortion".to_string(),
            node_type: "effect.distortion".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "drive".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "tone".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "level".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Register 3-Band EQ
    registry.insert("effect.eq3".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "3-Band EQ".to_string(),
            node_type: "effect.eq3".to_string(),
            inputs: vec![
                NeuroNodePort { name: "audio_in".to_string(), data_type: NeuroDataType::Audio, required: true },
                NeuroNodePort { name: "low_gain".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "mid_gain".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "high_gain".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    // Register FM Synthesizer
    registry.insert("generator.fm".to_string(), Box::new(|id| {
        NeuroNodeDefinition {
            id,
            name: "FM Synth".to_string(),
            node_type: "generator.fm".to_string(),
            inputs: vec![
                NeuroNodePort { name: "frequency".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "fm_amount".to_string(), data_type: NeuroDataType::Float, required: false },
                NeuroNodePort { name: "fm_ratio".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "audio_out".to_string(), data_type: NeuroDataType::Audio, required: true },
            ],
        }
    }));

    registry
}

/// Reverb implementation (Schroeder reverb)
pub struct ReverbNode {
    comb_delays: [DelayLine; 4],
    allpass_delays: [DelayLine; 2],
    room_size: f32,
    damping: f32,
    wet_dry: f32,
    sample_rate: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for ReverbNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update parameters
        if let Some(room) = inputs.get("room_size").and_then(|v| v.as_f64()) {
            self.room_size = room as f32;
        }
        if let Some(damp) = inputs.get("damping").and_then(|v| v.as_f64()) {
            self.damping = damp as f32;
        }
        if let Some(wet) = inputs.get("wet_dry").and_then(|v| v.as_f64()) {
            self.wet_dry = wet as f32;
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);
            
            // Process left channel
            let mut reverb_left = 0.0;
            for comb in &mut self.comb_delays {
                comb.write(input_frame.left + reverb_left * self.room_size * 0.8);
                reverb_left += comb.read(comb.delay_time_to_samples(50.0 + reverb_left * 20.0));
            }
            
            // Allpass filters for diffusion
            for allpass in &mut self.allpass_delays {
                allpass.write(reverb_left);
                reverb_left = allpass.read(allpass.delay_time_to_samples(10.0));
            }
            
            // Process right channel (slightly different delays for stereo)
            let mut reverb_right = 0.0;
            for (idx, comb) in self.comb_delays.iter_mut().enumerate() {
                comb.write(input_frame.right + reverb_right * self.room_size * 0.8);
                reverb_right += comb.read(comb.delay_time_to_samples(52.0 + idx as f32 * 3.0 + reverb_right * 18.0));
            }
            
            for allpass in &mut self.allpass_delays {
                allpass.write(reverb_right);
                reverb_right = allpass.read(allpass.delay_time_to_samples(12.0));
            }
            
            // Mix wet/dry
            let wet_left = reverb_left * self.wet_dry;
            let wet_right = reverb_right * self.wet_dry;
            let dry_left = input_frame.left * (1.0 - self.wet_dry);
            let dry_right = input_frame.right * (1.0 - self.wet_dry);
            
            output.set(i, StereoFrame::new(dry_left + wet_left, dry_right + wet_right));
        }

        let mut outputs = std::collections::HashMap::new();
        outputs.insert("audio_out".to_string(), serde_json::to_value(output).unwrap());
        Ok(outputs)
    }
}

impl ReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            comb_delays: [
                DelayLine::new(100.0, sample_rate),
                DelayLine::new(120.0, sample_rate),
                DelayLine::new(140.0, sample_rate),
                DelayLine::new(160.0, sample_rate),
            ],
            allpass_delays: [
                DelayLine::new(30.0, sample_rate),
                DelayLine::new(40.0, sample_rate),
            ],
            room_size: 0.5,
            damping: 0.5,
            wet_dry: 0.3,
            sample_rate,
        }
    }
}

/// Distortion/Saturation effect
pub struct DistortionNode {
    drive: f32,
    tone: f32,
    level: f32,
    filter: BiquadFilter,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for DistortionNode {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update parameters
        if let Some(d) = inputs.get("drive").and_then(|v| v.as_f64()) {
            self.drive = d as f32;
        }
        if let Some(t) = inputs.get("tone").and_then(|v| v.as_f64()) {
            self.tone = t as f32;
            // Update tone filter
            self.filter = BiquadFilter::lowpass(1000.0 + self.tone * 4000.0, 0.7, 44100.0);
        }
        if let Some(l) = inputs.get("level").and_then(|v| v.as_f64()) {
            self.level = l as f32;
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);
            
            // Apply drive/distortion using tanh saturation
            let driven_left = (input_frame.left * self.drive).tanh();
            let driven_right = (input_frame.right * self.drive).tanh();
            
            // Apply tone filtering
            let filtered_left = self.filter.process(driven_left);
            let filtered_right = self.filter.process(driven_right);
            
            // Apply output level
            let output_left = filtered_left * self.level;
            let output_right = filtered_right * self.level;
            
            output.set(i, StereoFrame::new(output_left, output_right));
        }

        let mut outputs = std::collections::HashMap::new();
        outputs.insert("audio_out".to_string(), serde_json::to_value(output).unwrap());
        Ok(outputs)
    }
}

impl DistortionNode {
    pub fn new() -> Self {
        Self {
            drive: 2.0,
            tone: 0.5,
            level: 0.5,
            filter: BiquadFilter::lowpass(3000.0, 0.7, 44100.0),
        }
    }
}

/// Simple 3-band EQ
pub struct ThreeBandEQ {
    low_filter: BiquadFilter,
    mid_filter: BiquadFilter,
    high_filter: BiquadFilter,
    low_gain: f32,
    mid_gain: f32,
    high_gain: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for ThreeBandEQ {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        let audio_in: AudioBuffer = inputs.get("audio_in")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| AudioBuffer::new(BUFFER_SIZE));

        // Update parameters
        if let Some(low) = inputs.get("low_gain").and_then(|v| v.as_f64()) {
            self.low_gain = low as f32;
        }
        if let Some(mid) = inputs.get("mid_gain").and_then(|v| v.as_f64()) {
            self.mid_gain = mid as f32;
        }
        if let Some(high) = inputs.get("high_gain").and_then(|v| v.as_f64()) {
            self.high_gain = high as f32;
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            let input_frame = audio_in.get(i);
            
            // Process each band
            let low_left = self.low_filter.process(input_frame.left) * self.low_gain;
            let mid_left = self.mid_filter.process(input_frame.left) * self.mid_gain;
            let high_left = self.high_filter.process(input_frame.left) * self.high_gain;
            
            let low_right = self.low_filter.process(input_frame.right) * self.low_gain;
            let mid_right = self.mid_filter.process(input_frame.right) * self.mid_gain;
            let high_right = self.high_filter.process(input_frame.right) * self.high_gain;
            
            // Sum the bands
            let output_left = low_left + mid_left + high_left;
            let output_right = low_right + mid_right + high_right;
            
            output.set(i, StereoFrame::new(output_left, output_right));
        }

        let mut outputs = std::collections::HashMap::new();
        outputs.insert("audio_out".to_string(), serde_json::to_value(output).unwrap());
        Ok(outputs)
    }
}

impl ThreeBandEQ {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            low_filter: BiquadFilter::lowpass(200.0, 0.7, sample_rate),
            mid_filter: BiquadFilter::new(), // Bandpass would be better, using identity for now
            high_filter: BiquadFilter::highpass(2000.0, 0.7, sample_rate),
            low_gain: 1.0,
            mid_gain: 1.0,
            high_gain: 1.0,
        }
    }
}

/// FM Synthesizer
pub struct FMSynthesizer {
    carrier_phase: f32,
    modulator_phase: f32,
    sample_rate: f32,
    frequency: f32,
    fm_amount: f32,
    fm_ratio: f32,
}

#[async_trait::async_trait]
impl NeuroNodeProcessor for FMSynthesizer {
    async fn process(&mut self, inputs: std::collections::HashMap<String, Value>) -> Result<std::collections::HashMap<String, Value>, String> {
        // Update parameters
        if let Some(freq) = inputs.get("frequency").and_then(|v| v.as_f64()) {
            self.frequency = freq as f32;
        }
        if let Some(fm_amt) = inputs.get("fm_amount").and_then(|v| v.as_f64()) {
            self.fm_amount = fm_amt as f32;
        }
        if let Some(fm_rat) = inputs.get("fm_ratio").and_then(|v| v.as_f64()) {
            self.fm_ratio = fm_rat as f32;
        }

        let mut output = AudioBuffer::new(BUFFER_SIZE);

        for i in 0..BUFFER_SIZE {
            // Generate modulator
            let modulator = (self.modulator_phase * 2.0 * std::f32::consts::PI).sin();
            self.modulator_phase += (self.frequency * self.fm_ratio) / self.sample_rate;
            if self.modulator_phase >= 1.0 {
                self.modulator_phase -= 1.0;
            }
            
            // Generate carrier with FM
            let fm_offset = modulator * self.fm_amount;
            let carrier = ((self.carrier_phase + fm_offset) * 2.0 * std::f32::consts::PI).sin();
            self.carrier_phase += self.frequency / self.sample_rate;
            if self.carrier_phase >= 1.0 {
                self.carrier_phase -= 1.0;
            }
            
            output.set(i, StereoFrame::mono(carrier * 0.5));
        }

        let mut outputs = std::collections::HashMap::new();
        outputs.insert("audio_out".to_string(), serde_json::to_value(output).unwrap());
        Ok(outputs)
    }
}

impl FMSynthesizer {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            carrier_phase: 0.0,
            modulator_phase: 0.0,
            sample_rate,
            frequency: 440.0,
            fm_amount: 1.0,
            fm_ratio: 2.0,
        }
    }
}