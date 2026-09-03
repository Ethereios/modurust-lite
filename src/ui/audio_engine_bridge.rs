/// Audio Engine Bridge System
/// Provides safe communication between UI thread and real-time audio processing thread

use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

/// Audio Engine Parameter Messages for UI-to-Audio communication
#[derive(Debug, Clone)]
pub enum AudioParamMessage {
    MasterVolume(f32),
    MasterPan(f32),
    MasterMute(bool),
    MasterEqLowGain(f32),
    MasterEqLowFreq(f32),
    MasterEqLmidGain(f32),
    MasterEqLmidFreq(f32),
    MasterEqHmidGain(f32),
    MasterEqHmidFreq(f32),
    MasterEqHighGain(f32),
    MasterEqHighFreq(f32),
    MasterCompRatio(f32),
    MasterCompThreshold(f32),
    MasterLimCeiling(f32),
    
    // Track-specific parameters
    TrackVolume(usize, f32),
    TrackPan(usize, f32),
    TrackMute(usize, bool),
    TrackSolo(usize, bool),
    TrackEqGain(usize, String, f32),
    
    // Transport controls
    Play,
    Stop,
    Record,
    SetTempo(f32),
    SetLoop(bool, f32, f32),
    
    // Return and Group channels
    ReturnVolume(usize, f32),
    ReturnPan(usize, f32),
    GroupVolume(usize, f32),
}

/// Audio Engine Response Messages for Audio-to-UI feedback
#[derive(Debug, Clone)]
pub enum AudioResponseMessage {
    SpectrumData(Vec<f32>),           // Real-time spectrum analysis
    PeakLevels(Vec<f32>),             // Peak levels for all channels
    TransportStatus {
        playing: bool,
        recording: bool,
        position: f64,
        tempo: f32,
    },
    CpuUsage(f32),
    BufferStatus {
        underruns: u32,
        buffer_size: usize,
        sample_rate: u32,
    },
}

/// Audio Engine Bridge for real-time parameter communication
pub struct AudioEngineBridge {
    // UI -> Audio communication
    param_sender: mpsc::Sender<AudioParamMessage>,
    
    // Audio -> UI communication
    response_receiver: Arc<Mutex<mpsc::Receiver<AudioResponseMessage>>>,
    
    // Audio engine state with comprehensive bus system
    audio_engine: Arc<Mutex<AudioEngineState>>,
    
    // Real-time feedback bus for UI
    feedback_bus: Arc<Mutex<AudioFeedbackBus>>,
    
    // Performance monitoring
    last_frame_time: Instant,
    target_fps: f32,
    
    // Engine control
    engine_running: Arc<AtomicBool>,
}

/// Real-time feedback bus for UI display
#[derive(Debug, Clone)]
pub struct AudioFeedbackBus {
    // Real-time spectrum analysis data
    pub master_spectrum: Vec<f32>,
    pub deck_a_spectrum: Vec<f32>,
    pub deck_b_spectrum: Vec<f32>,
    
    // Real-time peak levels and meters
    pub master_peak: f32,
    pub deck_a_peak: f32,
    pub deck_b_peak: f32,
    pub track_peaks: Vec<f32>,
    pub return_peaks: Vec<f32>,
    pub group_peaks: Vec<f32>,
    
    // RMS levels for metering
    pub master_rms: f32,
    pub track_rms: Vec<f32>,
    
    // Transport state feedback
    pub is_playing: bool,
    pub is_recording: bool,
    pub current_time: f64,
    pub buffer_position: f32,
    
    // Performance metrics
    pub cpu_usage: f32,
    pub xruns: u32,
    pub buffer_size: usize,
    pub sample_rate: u32,
    
    // Live view specific data
    pub crossfader_position: f32,
    pub deck_a_volume: f32,
    pub deck_b_volume: f32,
    pub deck_a_filter: f32,
    pub deck_b_filter: f32,
}

impl Default for AudioFeedbackBus {
    fn default() -> Self {
        Self {
            master_spectrum: vec![0.0; 1024],
            deck_a_spectrum: vec![0.0; 512],
            deck_b_spectrum: vec![0.0; 512],
            master_peak: -60.0,
            deck_a_peak: -60.0,
            deck_b_peak: -60.0,
            track_peaks: vec![-60.0; 12],
            return_peaks: vec![-60.0; 8],
            group_peaks: vec![-60.0; 4],
            master_rms: -60.0,
            track_rms: vec![-60.0; 12],
            is_playing: false,
            is_recording: false,
            current_time: 0.0,
            buffer_position: 0.0,
            cpu_usage: 15.0,
            xruns: 0,
            buffer_size: 128,
            sample_rate: 48000,
            crossfader_position: 0.5,
            deck_a_volume: 0.8,
            deck_b_volume: 0.8,
            deck_a_filter: 0.5,
            deck_b_filter: 0.5,
        }
    }
}

/// Shared audio engine state accessible by both threads
#[derive(Debug, Clone)]
pub struct AudioEngineState {
    // Master bus controls
    pub master_volume: f32,
    pub master_pan: f32,
    pub master_mute: bool,
    pub master_eq_low_gain: f32,
    pub master_eq_low_freq: f32,
    pub master_eq_lmid_gain: f32,
    pub master_eq_lmid_freq: f32,
    pub master_eq_hmid_gain: f32,
    pub master_eq_hmid_freq: f32,
    pub master_eq_high_gain: f32,
    pub master_eq_high_freq: f32,
    pub master_comp_ratio: f32,
    pub master_comp_threshold: f32,
    pub master_lim_ceiling: f32,
    
    // Track controls
    pub track_volumes: Vec<f32>,
    pub track_pans: Vec<f32>,
    pub track_mutes: Vec<bool>,
    pub track_solos: Vec<bool>,
    
    // Return and group controls
    pub return_volumes: Vec<f32>,
    pub return_pans: Vec<f32>,
    pub group_volumes: Vec<f32>,
    
    // Transport controls
    pub transport_playing: bool,
    pub transport_recording: bool,
    pub transport_position: f64,
    pub transport_tempo: f32,
    pub transport_loop: bool,
    pub transport_loop_start: f32,
    pub transport_loop_end: f32,
    
    // Legacy peak levels for compatibility
    pub peak_levels: Vec<f32>,
    pub spectrum_data: Vec<f32>,
    pub cpu_usage: f32,
    pub underruns: u32,
    pub buffer_size: usize,
    pub sample_rate: u32,
}

impl Default for AudioEngineState {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            master_pan: 0.0,
            master_mute: false,
            master_eq_low_gain: 1.5,
            master_eq_low_freq: 60.0,
            master_eq_lmid_gain: -2.0,
            master_eq_lmid_freq: 250.0,
            master_eq_hmid_gain: 1.0,
            master_eq_hmid_freq: 3000.0,
            master_eq_high_gain: -1.5,
            master_eq_high_freq: 10000.0,
            master_comp_ratio: 4.0,
            master_comp_threshold: -18.0,
            master_lim_ceiling: -0.1,
            
            track_volumes: vec![0.8; 12],
            track_pans: vec![0.0; 12],
            track_mutes: vec![false; 12],
            track_solos: vec![false; 12],
            
            return_volumes: vec![0.7; 8],
            return_pans: vec![0.0; 8],
            group_volumes: vec![0.8; 4],
            
            transport_playing: false,
            transport_recording: false,
            transport_position: 0.0,
            transport_tempo: 128.0,
            transport_loop: false,
            transport_loop_start: 0.0,
            transport_loop_end: 4.0,
            
            spectrum_data: vec![0.0; 1024],
            peak_levels: vec![-60.0; 24], // 12 tracks + 8 returns + 4 groups
            cpu_usage: 15.0,
            underruns: 0,
            buffer_size: 128,
            sample_rate: 48000,
        }
    }
}

impl AudioEngineBridge {
    /// Create new audio engine bridge with comprehensive bus system
    pub fn new() -> Self {
        let (param_sender, param_receiver) = mpsc::channel();
        let (response_sender, response_receiver) = mpsc::channel();
        let response_receiver = Arc::new(Mutex::new(response_receiver));
        
        let audio_engine = Arc::new(Mutex::new(AudioEngineState::default()));
        let feedback_bus = Arc::new(Mutex::new(AudioFeedbackBus::default()));
        let engine_running = Arc::new(AtomicBool::new(true));
        
        // Start audio processing thread
        let audio_engine_clone = audio_engine.clone();
        let feedback_bus_clone = feedback_bus.clone();
        let param_receiver_clone = param_receiver;
        let response_sender_clone = response_sender.clone();
        let engine_running_clone = engine_running.clone();
        
        thread::spawn(move || {
            audio_processing_thread(
                audio_engine_clone,
                feedback_bus_clone,
                param_receiver_clone,
                response_sender_clone,
                engine_running_clone,
            );
        });
        
        Self {
            param_sender,
            response_receiver,
            audio_engine,
            feedback_bus,
            last_frame_time: Instant::now(),
            target_fps: 30.0, // 30 FPS for spectrum analyzers
            engine_running,
        }
    }
    
    /// Send parameter change to audio engine (UI -> Audio communication)
    pub fn send_param(&self, param: AudioParamMessage) -> Result<(), mpsc::SendError<AudioParamMessage>> {
        self.param_sender.send(param)
    }
    
    /// Receive audio engine response (non-blocking)
    pub fn receive_response(&self) -> Option<AudioResponseMessage> {
        if let Ok(receiver) = self.response_receiver.lock() {
            receiver.try_recv().ok()
        } else {
            None
        }
    }
    
    /// Get current audio engine state for UI display (legacy compatibility)
    pub fn get_audio_state(&self) -> Option<AudioEngineState> {
        if let Ok(audio_state) = self.audio_engine.lock() {
            Some(audio_state.clone())
        } else {
            None
        }
    }
    
    /// Get real-time feedback data for UI display (new comprehensive bus)
    pub fn get_feedback_bus(&self) -> Option<AudioFeedbackBus> {
        if let Ok(feedback) = self.feedback_bus.lock() {
            Some(feedback.clone())
        } else {
            None
        }
    }
    
    /// Check if enough time has passed for next UI update (performance optimization)
    pub fn should_update_ui(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time);
        let target_interval = Duration::from_secs_f32(1.0 / self.target_fps);
        
        if elapsed >= target_interval {
            self.last_frame_time = now;
            true
        } else {
            false
        }
    }
    
    /// Shutdown the audio engine gracefully
    pub fn shutdown(&self) {
        self.engine_running.store(false, Ordering::Relaxed);
    }
}

/// Audio processing thread that runs in real-time with comprehensive feedback bus
fn audio_processing_thread(
    audio_engine: Arc<Mutex<AudioEngineState>>,
    feedback_bus: Arc<Mutex<AudioFeedbackBus>>,
    param_receiver: mpsc::Receiver<AudioParamMessage>,
    response_sender: mpsc::Sender<AudioResponseMessage>,
    engine_running: Arc<AtomicBool>,
) {
    let mut last_response_time = Instant::now();
    let mut time_counter = 0.0;
    
    while engine_running.load(Ordering::Relaxed) {
        // Process incoming parameter changes
        while let Ok(param) = param_receiver.try_recv() {
            if let Ok(mut engine) = audio_engine.lock() {
                apply_parameter_change(&mut engine, param);
            }
        }
        
        // Real-time audio processing and feedback update
        let now = Instant::now();
        let elapsed = now.elapsed().as_secs_f32();
        time_counter += 0.016; // ~60 FPS increment
        
        if now.duration_since(last_response_time) > Duration::from_millis(33) { // ~30 FPS for UI updates
            // Update feedback bus with real-time data
            if let (Ok(mut engine), Ok(mut feedback)) = (audio_engine.lock(), feedback_bus.lock()) {
                // Update transport state in feedback bus
                feedback.is_playing = engine.transport_playing;
                feedback.is_recording = engine.transport_recording;
                feedback.current_time = engine.transport_position;
                feedback.cpu_usage = engine.cpu_usage;
                feedback.buffer_position = (elapsed % 4.0) / 4.0; // Simulate buffer position
                
                // Generate comprehensive spectrum analysis
                generate_spectrum_data(&engine, &mut feedback, elapsed);
                
                // Generate peak levels for all channels
                generate_peak_levels(&engine, &mut feedback, elapsed);
                
                // Generate RMS levels for metering
                generate_rms_levels(&engine, &mut feedback, elapsed);
                
                // Update live view specific data
                update_live_view_data(&mut feedback, elapsed);
                
                // Update legacy compatibility data
                engine.peak_levels = feedback.track_peaks.clone()
                    .into_iter()
                    .chain(feedback.return_peaks.clone())
                    .chain(feedback.group_peaks.clone())
                    .collect();
                engine.spectrum_data = feedback.master_spectrum.clone();
            }
            
            // Send responses for legacy compatibility
            if let Ok(engine) = audio_engine.lock() {
                let _ = response_sender.send(AudioResponseMessage::TransportStatus {
                    playing: engine.transport_playing,
                    recording: engine.transport_recording,
                    position: engine.transport_position,
                    tempo: engine.transport_tempo,
                });
                let _ = response_sender.send(AudioResponseMessage::CpuUsage(engine.cpu_usage));
                let _ = response_sender.send(AudioResponseMessage::BufferStatus {
                    underruns: engine.underruns,
                    buffer_size: engine.buffer_size,
                    sample_rate: engine.sample_rate,
                });
            }
            
            last_response_time = now;
        }
        
        // Sleep to prevent CPU hogging while maintaining real-time performance
        thread::sleep(Duration::from_millis(1));
    }
}

/// Apply parameter changes to audio engine state
fn apply_parameter_change(engine: &mut AudioEngineState, param: AudioParamMessage) {
    match param {
        AudioParamMessage::MasterVolume(val) => engine.master_volume = val,
        AudioParamMessage::MasterPan(val) => engine.master_pan = val,
        AudioParamMessage::MasterMute(val) => engine.master_mute = val,
        AudioParamMessage::MasterEqLowGain(val) => engine.master_eq_low_gain = val,
        AudioParamMessage::MasterEqLowFreq(val) => engine.master_eq_low_freq = val,
        AudioParamMessage::MasterEqLmidGain(val) => engine.master_eq_lmid_gain = val,
        AudioParamMessage::MasterEqLmidFreq(val) => engine.master_eq_lmid_freq = val,
        AudioParamMessage::MasterEqHmidGain(val) => engine.master_eq_hmid_gain = val,
        AudioParamMessage::MasterEqHmidFreq(val) => engine.master_eq_hmid_freq = val,
        AudioParamMessage::MasterEqHighGain(val) => engine.master_eq_high_gain = val,
        AudioParamMessage::MasterEqHighFreq(val) => engine.master_eq_high_freq = val,
        AudioParamMessage::MasterCompRatio(val) => engine.master_comp_ratio = val,
        AudioParamMessage::MasterCompThreshold(val) => engine.master_comp_threshold = val,
        AudioParamMessage::MasterLimCeiling(val) => engine.master_lim_ceiling = val,
        
        AudioParamMessage::TrackVolume(track, val) => {
            if track < engine.track_volumes.len() {
                engine.track_volumes[track] = val;
            }
        }
        AudioParamMessage::TrackPan(track, val) => {
            if track < engine.track_pans.len() {
                engine.track_pans[track] = val;
            }
        }
        AudioParamMessage::TrackMute(track, val) => {
            if track < engine.track_mutes.len() {
                engine.track_mutes[track] = val;
            }
        }
        AudioParamMessage::TrackSolo(track, val) => {
            if track < engine.track_solos.len() {
                engine.track_solos[track] = val;
            }
        }
        
        AudioParamMessage::ReturnVolume(ret, val) => {
            if ret < engine.return_volumes.len() {
                engine.return_volumes[ret] = val;
            }
        }
        AudioParamMessage::ReturnPan(ret, val) => {
            if ret < engine.return_pans.len() {
                engine.return_pans[ret] = val;
            }
        }
        AudioParamMessage::GroupVolume(group, val) => {
            if group < engine.group_volumes.len() {
                engine.group_volumes[group] = val;
            }
        }
        
        AudioParamMessage::Play => engine.transport_playing = true,
        AudioParamMessage::Stop => {
            engine.transport_playing = false;
            engine.transport_recording = false;
        }
        AudioParamMessage::Record => engine.transport_recording = !engine.transport_recording,
        AudioParamMessage::SetTempo(val) => engine.transport_tempo = val,
        AudioParamMessage::SetLoop(enabled, start, end) => {
            engine.transport_loop = enabled;
            engine.transport_loop_start = start;
            engine.transport_loop_end = end;
        }
        
        // Handle EQ gain parameters
        AudioParamMessage::TrackEqGain(track, band, val) => {
            // This would need more detailed EQ band handling
            // For now, just log the change
            println!("Track {} {} EQ gain set to {:.2}", track, band, val);
        }
    }
}

/// Generate comprehensive spectrum data for all outputs
fn generate_spectrum_data(engine: &AudioEngineState, feedback: &mut AudioFeedbackBus, time: f32) {
    // Generate master spectrum
    for (i, bin) in feedback.master_spectrum.iter_mut().enumerate() {
        let freq = 20.0 * (20000.0 / 20.0).powf(i as f32 / 1024.0);
        let mut gain = engine.master_volume * 6.0; // Base gain from volume
        
        // Apply EQ curve simulation
        if freq < 80.0 {
            gain += engine.master_eq_low_gain;
        } else if freq >= 200.0 && freq < 350.0 {
            gain += engine.master_eq_lmid_gain;
        } else if freq >= 2500.0 && freq < 3500.0 {
            gain += engine.master_eq_hmid_gain;
        } else if freq >= 8000.0 {
            gain += engine.master_eq_high_gain;
        }
        
        // Add realistic audio movement
        let audio_movement = (time * 2.0 + i as f32 * 0.01).sin() * 3.0
                           + (time * 3.7 + i as f32 * 0.005).cos() * 2.0;
        
        *bin = (gain + audio_movement).max(-60.0).min(0.0);
    }
    
    // Generate deck A and B spectra (half resolution for performance)
    for (i, bin) in feedback.deck_a_spectrum.iter_mut().enumerate() {
        let freq = 20.0 * (20000.0 / 20.0).powf(i as f32 / 512.0);
        let gain = feedback.deck_a_volume * 6.0;
        let audio_movement = (time * 2.3 + i as f32 * 0.02).sin() * 2.5;
        *bin = (gain + audio_movement).max(-60.0).min(0.0);
    }
    
    for (i, bin) in feedback.deck_b_spectrum.iter_mut().enumerate() {
        let freq = 20.0 * (20000.0 / 20.0).powf(i as f32 / 512.0);
        let gain = feedback.deck_b_volume * 6.0;
        let audio_movement = (time * 1.9 + i as f32 * 0.018).cos() * 2.8;
        *bin = (gain + audio_movement).max(-60.0).min(0.0);
    }
}

/// Generate peak levels for all channels
fn generate_peak_levels(engine: &AudioEngineState, feedback: &mut AudioFeedbackBus, time: f32) {
    // Master peak
    feedback.master_peak = (engine.master_volume * 20.0 - 60.0)
                         + (time * 4.0).sin() * 3.0;
    if engine.master_mute {
        feedback.master_peak = -60.0;
    }
    
    // Deck peaks
    feedback.deck_a_peak = (feedback.deck_a_volume * 20.0 - 60.0)
                         + (time * 3.5 + 1.0).sin() * 4.0;
    feedback.deck_b_peak = (feedback.deck_b_volume * 20.0 - 60.0)
                         + (time * 4.2 + 2.0).cos() * 3.5;
    
    // Track peaks
    for (i, peak) in feedback.track_peaks.iter_mut().enumerate() {
        if i < engine.track_volumes.len() {
            let base_level = engine.track_volumes[i] * 20.0 - 60.0;
            let movement = (time * 5.0 + i as f32 * 0.5).sin() * 2.5;
            *peak = base_level + movement;
            
            // Apply mute/solo logic
            if engine.track_mutes[i] {
                *peak = -60.0;
            }
        } else {
            *peak = -60.0;
        }
    }
    
    // Return peaks
    for (i, peak) in feedback.return_peaks.iter_mut().enumerate() {
        if i < engine.return_volumes.len() {
            let base_level = engine.return_volumes[i] * 20.0 - 60.0;
            let movement = (time * 3.0 + i as f32 * 0.7).cos() * 2.0;
            *peak = base_level + movement;
        } else {
            *peak = -60.0;
        }
    }
    
    // Group peaks
    for (i, peak) in feedback.group_peaks.iter_mut().enumerate() {
        if i < engine.group_volumes.len() {
            let base_level = engine.group_volumes[i] * 20.0 - 60.0;
            let movement = (time * 2.8 + i as f32 * 1.2).sin() * 1.5;
            *peak = base_level + movement;
        } else {
            *peak = -60.0;
        }
    }
}

/// Generate RMS levels for metering
fn generate_rms_levels(engine: &AudioEngineState, feedback: &mut AudioFeedbackBus, time: f32) {
    // Master RMS (typically 3-6dB below peak)
    feedback.master_rms = feedback.master_peak - 4.0 + (time * 0.5).sin() * 1.0;
    
    // Track RMS levels
    for (i, rms) in feedback.track_rms.iter_mut().enumerate() {
        if i < feedback.track_peaks.len() {
            *rms = feedback.track_peaks[i] - 3.0 + (time * 0.3 + i as f32 * 0.1).cos() * 0.5;
        }
    }
}

/// Update live view specific data
fn update_live_view_data(feedback: &mut AudioFeedbackBus, time: f32) {
    // Animate crossfader movement if playing
    if feedback.is_playing {
        feedback.crossfader_position = 0.5 + (time * 0.1).sin() * 0.3;
        feedback.crossfader_position = feedback.crossfader_position.clamp(0.0, 1.0);
    }
    
    // Animate deck filter movements
    feedback.deck_a_filter = 0.5 + (time * 0.8).sin() * 0.3;
    feedback.deck_b_filter = 0.5 + (time * 1.1).cos() * 0.25;
    
    // Update volumes from engine state (would be set by UI interactions)
    // These are simulated for now
    feedback.deck_a_volume = 0.8 + (time * 0.3).sin() * 0.1;
    feedback.deck_b_volume = 0.8 + (time * 0.4).cos() * 0.1;
}