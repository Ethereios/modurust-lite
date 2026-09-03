/// Transport and Clock System - Sample-accurate timing and synchronization
/// 
/// This module provides the master clock for DAW operations including
/// transport controls, tempo management, and sample-accurate timing.

use std::time::Instant;

/// Tempo information
#[derive(Debug, Clone, Copy)]
pub struct Tempo {
    pub bpm: f32,
    pub time_signature: (u32, u32), // (numerator, denominator)
    pub swing: f32, // 0.0 = no swing, 1.0 = full swing
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            time_signature: (4, 4),
            swing: 0.0,
        }
    }
}

/// Transport state and controls
#[derive(Debug, Clone, Copy)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Recording,
}

impl Default for TransportState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Sample-accurate clock system
pub struct Clock {
    sample_rate: u32,
    samples_per_beat: f32,
    total_samples: u64,
    samples_since_start: u64,
    is_playing: bool,
    tempo: Tempo,
    start_time: Option<Instant>,
    playhead_samples: u64,
}

impl Clock {
    /// Create a new clock with given sample rate
    pub fn new(sample_rate: u32) -> Self {
        let tempo = Tempo::default();
        let samples_per_beat = Self::calculate_samples_per_beat(sample_rate, tempo.bpm);
        
        Self {
            sample_rate,
            samples_per_beat,
            total_samples: 0,
            samples_since_start: 0,
            is_playing: false,
            tempo,
            start_time: None,
            playhead_samples: 0,
        }
    }
    
    /// Calculate samples per beat based on BPM and sample rate
    fn calculate_samples_per_beat(sample_rate: u32, bpm: f32) -> f32 {
        (sample_rate as f32 * 60.0) / bpm
    }
    
    /// Start playback
    pub fn play(&mut self) {
        if !self.is_playing {
            self.is_playing = true;
            self.start_time = Some(Instant::now());
            println!("▶️ Transport playing at {} BPM", self.tempo.bpm);
        }
    }
    
    /// Stop playback
    pub fn stop(&mut self) {
        if self.is_playing {
            self.is_playing = false;
            self.total_samples += self.samples_since_start;
            self.samples_since_start = 0;
            println!("⏹️ Transport stopped");
        }
    }
    
    /// Pause playback
    pub fn pause(&mut self) {
        if self.is_playing {
            self.is_playing = false;
            self.total_samples += self.samples_since_start;
            println!("⏸️ Transport paused");
        }
    }
    
    /// Start recording
    pub fn record(&mut self) {
        // Recording is just playing with a different state
        self.play();
        println!("⏺️ Transport recording");
    }
    
    /// Update clock (called every buffer processing)
    pub fn update(&mut self) {
        if self.is_playing {
            self.samples_since_start += 1;
        }
    }
    
    /// Get current time position in seconds
    pub fn current_time(&self) -> f64 {
        (self.total_samples + self.samples_since_start) as f64 / self.sample_rate as f64
    }
    
    /// Get current time position in beats
    pub fn current_beats(&self) -> f64 {
        (self.total_samples + self.samples_since_start) as f64 / self.samples_per_beat as f64
    }
    
    /// Get current time position in bars:beats:ticks format
    pub fn current_bbt(&self) -> (u32, u32, u32) {
        let total_beats = self.current_beats();
        let bars = (total_beats / self.tempo.time_signature.0 as f64).floor() as u32;
        let remaining_beats = (total_beats % self.tempo.time_signature.0 as f64).floor() as u32;
        let ticks = ((total_beats % 1.0) * 960.0).floor() as u32; // 960 ticks per quarter note
        
        (bars, remaining_beats, ticks)
    }
    
    /// Set BPM
    pub fn set_bpm(&mut self, bpm: f32) {
        let old_bpm = self.tempo.bpm;
        self.tempo.bpm = bpm.max(20.0).min(300.0);
        self.samples_per_beat = Self::calculate_samples_per_beat(self.sample_rate, self.tempo.bpm);
        
        if (old_bpm - self.tempo.bpm).abs() > 0.1 {
            println!("🎼 BPM changed: {} -> {}", old_bpm, self.tempo.bpm);
        }
    }
    
    /// Set time signature
    pub fn set_time_signature(&mut self, numerator: u32, denominator: u32) {
        self.tempo.time_signature = (numerator, denominator);
        println!("🎵 Time signature changed to {}/{}", numerator, denominator);
    }
    
    /// Set swing amount
    pub fn set_swing(&mut self, swing: f32) {
        self.tempo.swing = swing.max(0.0).min(1.0);
    }
    
    /// Get samples per beat at current tempo
    pub fn samples_per_beat(&self) -> f32 {
        self.samples_per_beat
    }
    
    /// Get samples per bar at current tempo
    pub fn samples_per_bar(&self) -> f32 {
        self.samples_per_beat * self.tempo.time_signature.0 as f32
    }
    
    /// Get current BPM
    pub fn bpm(&self) -> f32 {
        self.tempo.bpm
    }
    
    /// Get current time signature
    pub fn time_signature(&self) -> (u32, u32) {
        self.tempo.time_signature
    }
    
    /// Get swing amount
    pub fn swing(&self) -> f32 {
        self.tempo.swing
    }
    
    /// Check if transport is playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }
    
    /// Get playhead position in samples
    pub fn playhead_samples(&self) -> u64 {
        self.total_samples + self.samples_since_start
    }
    
    /// Seek to specific sample position
    pub fn seek_to_samples(&mut self, samples: u64) {
        self.total_samples = samples;
        self.samples_since_start = 0;
        self.playhead_samples = samples;
        println!("⏩ Seeked to sample {}", samples);
    }
    
    /// Seek to specific time in seconds
    pub fn seek_to_time(&mut self, seconds: f64) {
        let samples = (seconds * self.sample_rate as f64) as u64;
        self.seek_to_samples(samples);
    }
    
    /// Seek to specific beat position
    pub fn seek_to_beats(&mut self, beats: f64) {
        let samples = (beats * self.samples_per_beat as f64) as u64;
        self.seek_to_samples(samples);
    }
    
    /// Get tempo information
    pub fn tempo_info(&self) -> Tempo {
        self.tempo
    }
    
    /// Calculate samples for a specific duration at current tempo
    pub fn samples_for_duration(&self, beats: f32) -> u32 {
        (beats * self.samples_per_beat) as u32
    }
    
    /// Calculate beats for a specific number of samples
    pub fn beats_for_samples(&self, samples: u32) -> f32 {
        samples as f32 / self.samples_per_beat
    }
    
    /// Apply swing to beat timing (for internal timing calculations)
    pub fn apply_swing(&self, beat: f32) -> f32 {
        if self.tempo.swing > 0.0 && beat % 2.0 == 1.0 {
            // Apply swing to off-beats
            let swing_amount = self.tempo.swing * 0.5; // Max 50% swing
            beat + swing_amount
        } else {
            beat
        }
    }
    
    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Reset transport to beginning
    pub fn reset(&mut self) {
        self.total_samples = 0;
        self.samples_since_start = 0;
        self.playhead_samples = 0;
        self.is_playing = false;
        self.start_time = None;
        println!("🔄 Transport reset to beginning");
    }
}

/// Main Transport controller
pub struct Transport {
    clock: Clock,
    state: TransportState,
    loop_enabled: bool,
    loop_start_samples: u64,
    loop_end_samples: u64,
    _punch_in_enabled: bool,
    _punch_out_enabled: bool,
    metronome_enabled: bool,
    external_sync_enabled: bool,
}

impl Transport {
    /// Create a new transport with given sample rate
    pub fn new(sample_rate: u32) -> Self {
        Self {
            clock: Clock::new(sample_rate),
            state: TransportState::default(),
            loop_enabled: false,
            loop_start_samples: 0,
            loop_end_samples: 0,
            _punch_in_enabled: false,
            _punch_out_enabled: false,
            metronome_enabled: false,
            external_sync_enabled: false,
        }
    }
    
    /// Play
    pub fn play(&mut self) {
        self.clock.play();
        self.state = TransportState::Playing;
    }
    
    /// Stop
    pub fn stop(&mut self) {
        self.clock.stop();
        self.state = TransportState::Stopped;
    }
    
    /// Pause
    pub fn pause(&mut self) {
        self.clock.pause();
        self.state = TransportState::Paused;
    }
    
    /// Record
    pub fn record(&mut self) {
        self.clock.record();
        self.state = TransportState::Recording;
    }
    
    /// Update transport (called every buffer)
    pub fn update(&mut self) {
        self.clock.update();
        
        // Handle looping
        if self.loop_enabled && self.clock.is_playing() {
            let current_samples = self.clock.playhead_samples();
            if current_samples >= self.loop_end_samples {
                self.clock.seek_to_samples(self.loop_start_samples);
                println!("🔄 Loop point reached, jumping back to {}", self.loop_start_samples);
            }
        }
    }
    
    /// Set BPM
    pub fn set_tempo(&mut self, bpm: f32) {
        self.clock.set_bpm(bpm);
    }
    
    /// Get current BPM
    pub fn bpm(&self) -> f32 {
        self.clock.bpm()
    }
    
    /// Enable/disable loop
    pub fn set_loop_enabled(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
        println!("🔁 Loop {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Set loop region
    pub fn set_loop_region(&mut self, start_samples: u64, end_samples: u64) {
        self.loop_start_samples = start_samples;
        self.loop_end_samples = end_samples;
    }
    
    /// Get current transport state
    pub fn transport_state(&self) -> TransportState {
        self.state
    }
    
    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.clock.is_playing()
    }
    
    /// Get current time position
    pub fn time_position(&self) -> f64 {
        self.clock.current_time()
    }
    
    /// Get current beat position
    pub fn beat_position(&self) -> f64 {
        self.clock.current_beats()
    }
    
    /// Get current bar:beat:tick position
    pub fn bbt_position(&self) -> (u32, u32, u32) {
        self.clock.current_bbt()
    }
    
    /// Seek to position
    pub fn seek_to(&mut self, samples: u64) {
        self.clock.seek_to_samples(samples);
    }
    
    /// Enable/disable metronome
    pub fn set_metronome_enabled(&mut self, enabled: bool) {
        self.metronome_enabled = enabled;
    }
    
    /// Check if metronome is enabled
    pub fn metronome_enabled(&self) -> bool {
        self.metronome_enabled
    }
    
    /// Enable/disable external sync
    pub fn set_external_sync_enabled(&mut self, enabled: bool) {
        self.external_sync_enabled = enabled;
    }
    
    /// Check if external sync is enabled
    pub fn external_sync_enabled(&self) -> bool {
        self.external_sync_enabled
    }
    
    /// Reset transport
    pub fn reset(&mut self) {
        self.clock.reset();
        self.state = TransportState::Stopped;
    }
    
    /// Get clock information
    pub fn clock(&self) -> &Clock {
        &self.clock
    }
    
    /// Get mutable clock reference (use carefully!)
    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self::new(44100)
    }
}