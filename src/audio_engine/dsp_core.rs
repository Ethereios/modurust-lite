/// DSP Core - Audio processing modules for real-time audio
/// 
/// This module contains all the fundamental audio processing building blocks
/// used in the HexoDSP node graph system.

use std::f32::consts::PI;

/// Generic DSP module trait that all audio processors implement
/// Send is required for thread-safe audio processing across threads
pub trait DSPModule: Send {
    /// Process a block of audio samples
    fn process(&mut self, input: &[f32], output: &mut [f32]);
    
    /// Set a parameter value (e.g., frequency, gain)
    fn set_parameter(&mut self, param_id: &str, value: f32);
    
    /// Get a parameter value
    fn get_parameter(&self, param_id: &str) -> Option<f32>;
    
    /// Reset internal state (for clearing filters, etc.)
    fn reset(&mut self);
    
    /// Module name for identification
    fn name(&self) -> &str;
}

/// Basic oscillator implementation
pub struct Oscillator {
    sample_rate: f32,
    frequency: f32,
    phase: f32,
    wave_type: WaveType,
}

#[derive(Clone, Copy, Debug)]
pub enum WaveType {
    Sine,
    Square,
    Saw,
    Triangle,
    Noise,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            frequency: 440.0,
            phase: 0.0,
            wave_type: WaveType::Sine,
        }
    }
    
    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.max(20.0).min(20000.0);
    }
    
    pub fn set_wave_type(&mut self, wave_type: WaveType) {
        self.wave_type = wave_type;
    }
    
    /// Generate next sample
    fn next_sample(&mut self) -> f32 {
        let phase_inc = self.frequency / self.sample_rate;
        self.phase = (self.phase + phase_inc) % 1.0;
        
        match self.wave_type {
            WaveType::Sine => (2.0 * PI * self.phase).sin(),
            WaveType::Square => if self.phase < 0.5 { 1.0 } else { -1.0 },
            WaveType::Saw => 2.0 * self.phase - 1.0,
            WaveType::Triangle => {
                if self.phase < 0.25 {
                    4.0 * self.phase
                } else if self.phase < 0.75 {
                    2.0 - 4.0 * self.phase
                } else {
                    4.0 * self.phase - 4.0
                }
            }
            WaveType::Noise => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                use std::time::SystemTime;
                
                // Simple hash-based pseudo-random generation
                let time = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                let mut hasher = DefaultHasher::new();
                time.hash(&mut hasher);
                ((hasher.finish() & 0xFFFF) as f32 / 32767.0) * 2.0 - 1.0
            }
        }
    }
}

impl DSPModule for Oscillator {
    fn process(&mut self, _input: &[f32], output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.next_sample();
        }
    }
    
    fn set_parameter(&mut self, param_id: &str, value: f32) {
        match param_id {
            "frequency" => self.set_frequency(value),
            "phase" => self.phase = value % 1.0,
            _ => {},
        }
    }
    
    fn get_parameter(&self, param_id: &str) -> Option<f32> {
        match param_id {
            "frequency" => Some(self.frequency),
            "phase" => Some(self.phase),
            _ => None,
        }
    }
    
    fn reset(&mut self) {
        self.phase = 0.0;
    }
    
    fn name(&self) -> &str {
        "Oscillator"
    }
}

/// Digital filter implementation
pub struct Filter {
    sample_rate: f32,
    cutoff_freq: f32,
    resonance: f32,
    filter_type: FilterType,
    // Internal state for biquad filter
    x1: f32, x2: f32, y1: f32, y2: f32,
    b0: f32, b1: f32, b2: f32, a1: f32, a2: f32, a0: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
}

impl Filter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            cutoff_freq: 1000.0,
            resonance: 0.7,
            filter_type: FilterType::LowPass,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
            b0: 1.0, b1: 0.0, b2: 0.0, a0: 1.0, a1: 0.0, a2: 0.0,
        };
        filter.update_coeffs();
        filter
    }
    
    pub fn set_cutoff(&mut self, freq: f32) {
        self.cutoff_freq = freq.max(20.0).min(self.sample_rate * 0.45);
        self.update_coeffs();
    }
    
    pub fn set_resonance(&mut self, q: f32) {
        self.resonance = q.max(0.1).min(20.0);
        self.update_coeffs();
    }
    
    pub fn set_filter_type(&mut self, filter_type: FilterType) {
        self.filter_type = filter_type;
        self.update_coeffs();
    }
    
    /// Update biquad coefficients based on current parameters
    fn update_coeffs(&mut self) {
        let omega = 2.0 * PI * self.cutoff_freq / self.sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.resonance);
        
        match self.filter_type {
            FilterType::LowPass => {
                self.b0 = (1.0 - cos_omega) * 0.5;
                self.b1 = 1.0 - cos_omega;
                self.b2 = (1.0 - cos_omega) * 0.5;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_omega;
                self.a2 = 1.0 - alpha;
            }
            FilterType::HighPass => {
                self.b0 = (1.0 + cos_omega) * 0.5;
                self.b1 = -(1.0 + cos_omega);
                self.b2 = (1.0 + cos_omega) * 0.5;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_omega;
                self.a2 = 1.0 - alpha;
            }
            FilterType::BandPass => {
                self.b0 = sin_omega * 0.5;
                self.b1 = 0.0;
                self.b2 = -sin_omega * 0.5;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_omega;
                self.a2 = 1.0 - alpha;
            }
            FilterType::Notch => {
                self.b0 = 1.0;
                self.b1 = -2.0 * cos_omega;
                self.b2 = 1.0;
                self.a0 = 1.0 + alpha;
                self.a1 = -2.0 * cos_omega;
                self.a2 = 1.0 - alpha;
            }
        }
        
        // Normalize coefficients
        let a0_inv = 1.0 / self.a0;
        self.b0 *= a0_inv;
        self.b1 *= a0_inv;
        self.b2 *= a0_inv;
        self.a1 *= a0_inv;
        self.a2 *= a0_inv;
    }
}

impl DSPModule for Filter {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (i, sample) in input.iter().enumerate() {
            // Biquad filter implementation
            let x0 = *sample;
            let y0 = self.b0 * x0 + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
            
            // Update delay lines
            self.x2 = self.x1;
            self.x1 = x0;
            self.y2 = self.y1;
            self.y1 = y0;
            
            output[i] = y0;
        }
    }
    
    fn set_parameter(&mut self, param_id: &str, value: f32) {
        match param_id {
            "cutoff" => self.set_cutoff(value),
            "resonance" => self.set_resonance(value),
            _ => {},
        }
    }
    
    fn get_parameter(&self, param_id: &str) -> Option<f32> {
        match param_id {
            "cutoff" => Some(self.cutoff_freq),
            "resonance" => Some(self.resonance),
            _ => None,
        }
    }
    
    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
    
    fn name(&self) -> &str {
        "Filter"
    }
}

/// Delay line implementation
pub struct Delay {
    sample_rate: f32,
    delay_time: f32, // in seconds
    feedback: f32,
    mix: f32,
    buffer: Vec<f32>,
    write_index: usize,
    max_delay_samples: usize,
}

impl Delay {
    pub fn new(sample_rate: f32, max_delay_time: f32) -> Self {
        let max_delay_samples = (max_delay_time * sample_rate) as usize;
        Self {
            sample_rate,
            delay_time: 0.1, // 100ms default
            feedback: 0.3,
            mix: 0.5,
            buffer: vec![0.0; max_delay_samples],
            write_index: 0,
            max_delay_samples,
        }
    }
    
    pub fn set_delay_time(&mut self, time: f32) {
        self.delay_time = time.max(0.0).min(self.max_delay_samples as f32 / self.sample_rate);
    }
    
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.max(0.0).min(0.99); // Prevent infinite feedback
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.max(0.0).min(1.0);
    }
}

impl DSPModule for Delay {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        let delay_samples = (self.delay_time * self.sample_rate) as usize;
        let max_index = self.max_delay_samples;
        
        for (i, &sample) in input.iter().enumerate() {
            // Read from delay line
            let read_index = (self.write_index + max_index - delay_samples) % max_index;
            let delayed_sample = self.buffer[read_index];
            
            // Calculate output: mix of dry and wet signal
            output[i] = sample * (1.0 - self.mix) + delayed_sample * self.mix;
            
            // Write to delay line with feedback
            self.buffer[self.write_index] = sample + delayed_sample * self.feedback;
            
            self.write_index = (self.write_index + 1) % max_index;
        }
    }
    
    fn set_parameter(&mut self, param_id: &str, value: f32) {
        match param_id {
            "delay_time" => self.set_delay_time(value),
            "feedback" => self.set_feedback(value),
            "mix" => self.set_mix(value),
            _ => {},
        }
    }
    
    fn get_parameter(&self, param_id: &str) -> Option<f32> {
        match param_id {
            "delay_time" => Some(self.delay_time),
            "feedback" => Some(self.feedback),
            "mix" => Some(self.mix),
            _ => None,
        }
    }
    
    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_index = 0;
    }
    
    fn name(&self) -> &str {
        "Delay"
    }
}

/// Reverb implementation (simplified Schroeder reverb)
pub struct Reverb {
    _sample_rate: f32,
    room_size: f32,
    damping: f32,
    mix: f32,
    // Comb filter delays
    comb_delays: [usize; 4],
    comb_feedbacks: [f32; 4],
    comb_buffers: [Vec<f32>; 4],
    // Allpass delays  
    allpass_delays: [usize; 2],
    allpass_buffers: [Vec<f32>; 2],
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let comb_delays = [1116, 1188, 1277, 1356];
        let comb_feedbacks = [0.84, 0.84, 0.84, 0.84];
        
        let comb_buffers = [
            vec![0.0; comb_delays[0]],
            vec![0.0; comb_delays[1]],
            vec![0.0; comb_delays[2]],
            vec![0.0; comb_delays[3]],
        ];
        
        let allpass_delays = [556, 441];
        let allpass_buffers = [
            vec![0.0; allpass_delays[0]],
            vec![0.0; allpass_delays[1]],
        ];
        
        Self {
            _sample_rate: sample_rate,
            room_size: 1.0,
            damping: 0.5,
            mix: 0.3,
            comb_delays,
            comb_feedbacks,
            comb_buffers,
            allpass_delays,
            allpass_buffers,
        }
    }
}

impl DSPModule for Reverb {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (i, &sample) in input.iter().enumerate() {
            let mut reverb_signal = 0.0;
            
            // Process comb filters in parallel
            for (comb_idx, (&delay, &feedback)) in self.comb_delays.iter().zip(self.comb_feedbacks.iter()).enumerate() {
                let buffer = &mut self.comb_buffers[comb_idx];
                let write_pos = i % delay;
                let read_pos = (write_pos + delay - 1) % delay;
                
                let delayed = buffer[read_pos];
                let new_value = sample + delayed * feedback;
                buffer[write_pos] = new_value;
                
                reverb_signal += new_value;
            }
            
            // Scale the reverb signal
            reverb_signal *= 0.25; // Average of 4 comb filters
            
            // Process allpass filters in series
            for (ap_idx, &delay) in self.allpass_delays.iter().enumerate() {
                let buffer = &mut self.allpass_buffers[ap_idx];
                let write_pos = i % delay;
                let read_pos = (write_pos + delay - 1) % delay;
                
                let buffered = buffer[read_pos];
                let new_value = reverb_signal * 0.5 + buffered * 0.5;
                buffer[write_pos] = new_value;
                
                reverb_signal = new_value;
            }
            
            // Mix dry and wet signals
            output[i] = sample * (1.0 - self.mix) + reverb_signal * self.mix;
        }
    }
    
    fn set_parameter(&mut self, param_id: &str, value: f32) {
        match param_id {
            "room_size" => self.room_size = value.max(0.1).min(10.0),
            "damping" => self.damping = value.max(0.0).min(1.0),
            "mix" => self.mix = value.max(0.0).min(1.0),
            _ => {},
        }
    }
    
    fn get_parameter(&self, param_id: &str) -> Option<f32> {
        match param_id {
            "room_size" => Some(self.room_size),
            "damping" => Some(self.damping),
            "mix" => Some(self.mix),
            _ => None,
        }
    }
    
    fn reset(&mut self) {
        for buffer in &mut self.comb_buffers {
            buffer.fill(0.0);
        }
        for buffer in &mut self.allpass_buffers {
            buffer.fill(0.0);
        }
    }
    
    fn name(&self) -> &str {
        "Reverb"
    }
}

/// Main DSP processing core
pub struct DSPCore {
    sample_rate: u32,
    _buffer_size: usize,
    nodes: Vec<Box<dyn DSPModule>>,
}

impl DSPCore {
    pub fn new(sample_rate: u32, buffer_size: u32) -> Self {
        Self {
            sample_rate,
            _buffer_size: buffer_size as usize,
            nodes: Vec::new(),
        }
    }
    
    pub fn add_oscillator(&mut self) -> usize {
        let oscillator = Oscillator::new(self.sample_rate as f32);
        self.nodes.push(Box::new(oscillator));
        self.nodes.len() - 1
    }
    
    pub fn add_filter(&mut self) -> usize {
        let filter = Filter::new(self.sample_rate as f32);
        self.nodes.push(Box::new(filter));
        self.nodes.len() - 1
    }
    
    pub fn add_delay(&mut self) -> usize {
        let delay = Delay::new(self.sample_rate as f32, 2.0); // 2 seconds max
        self.nodes.push(Box::new(delay));
        self.nodes.len() - 1
    }
    
    pub fn add_reverb(&mut self) -> usize {
        let reverb = Reverb::new(self.sample_rate as f32);
        self.nodes.push(Box::new(reverb));
        self.nodes.len() - 1
    }
    
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        // For now, simply pass through (in a real implementation,
        // this would process through the node graph)
        output.copy_from_slice(input);
    }
    
    pub fn get_node(&mut self, node_id: usize) -> Option<&mut Box<dyn DSPModule>> {
        self.nodes.get_mut(node_id)
    }
}