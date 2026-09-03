use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::{Arc, Mutex};
use crate::event_queue::EventQueue;
use std::time::{Duration, Instant};
use std::thread;
use std::collections::HashMap;

/// Messages sent from UI to Audio Engine
#[derive(Debug, Clone)]
pub enum AudioParamMessage {
    // Transport controls
    Play,
    Stop,
    Pause,
    Record,
    SetTempo(f32),
    SetTimeSignature(u32, u32),
    // Looping controls (enabled, start_beats, end_beats)
    SetLoop(bool, f32, f32),
    
    // Master controls
    MasterVolume(f32),
    MasterPan(f32),
    MasterMute(bool),
    
    // Track controls
    TrackVolume(usize, f32),
    TrackPan(usize, f32),
    TrackMute(usize, bool),
    TrackSolo(usize, bool),
    TrackArm(usize, bool),
    
    // Mixer controls
    SendLevel(usize, usize, f32), // (track, send, level)
    ReturnVolume(usize, f32),
    ReturnPan(usize, f32),
    GroupVolume(usize, f32),
    
    // EQ and effects
    TrackEQ(usize, String, f32), // (track, param, value)
    MasterEQLow(f32),
    MasterEQHigh(f32),
    TrackEffect(usize, usize, String), // (track, effect_slot, effect_type)
    
    // Node graph control
    CreateNode(String, String), // (node_type, ui_node_id) - Returns audio_node_id
    DeleteNode(String), // (ui_node_id)
    ConnectNodes(String, String, String, String), // (from_ui_id, to_ui_id, from_port, to_port)
    DisconnectNodes(String, String), // (from_ui_id, to_ui_id)
    SetNodeParameter(String, String, f32), // (ui_node_id, param_name, value)
    
    // Generic parameter (deprecated - use SetNodeParameter)
    SetParameter(String, f32),
    
    // System messages
    ReloadConfig,
    ResetEngine,
    // Input monitoring (enable/disable input stream to prevent feedback)
    SetInputMonitoring(bool),
}

/// Real-time audio engine state sent to UI
#[derive(Debug, Clone, Default)]
pub struct AudioEngineState {
    // Transport state
    pub playing: bool,
    pub recording: bool,
    pub bpm: f32,
    pub time_position: f64,
    pub bar_beat_tick: (u32, u32, u32),
    
    // Audio levels
    pub master_peak: f32,
    pub track_peaks: Vec<f32>,
    pub return_peaks: Vec<f32>,
    pub group_peaks: Vec<f32>,
    
    // Spectrum data for visualization
    pub spectrum_data: Vec<f32>,
    
    // Performance metrics
    pub cpu_usage: f32,
    pub buffer_xrun: u32,
    pub latency_ms: f32,
    
    // Real-time parameter values
    pub current_params: HashMap<String, f32>,
    
    // Node graph state
    pub active_nodes: Vec<String>, // UI node IDs currently processing audio
    pub node_levels: HashMap<String, f32>, // UI node ID -> output level
    pub node_to_audio_map: HashMap<String, usize>, // UI node ID -> Audio node ID
}

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub control_buffer_size: usize,  // UI->Audio messages
    pub feedback_buffer_size: usize, // Audio->UI messages
    pub feedback_rate_hz: f32,       // UI update rate
    pub max_param_queue: usize,      // Max queued parameters
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            control_buffer_size: 100,
            feedback_buffer_size: 10,
            feedback_rate_hz: 30.0, // 30 Hz UI updates
            max_param_queue: 1000,
        }
    }
}

/// Real-time audio engine bridge
pub struct AudioEngineBridge {
    // Communication channels
    param_sender: Sender<AudioParamMessage>,
    param_receiver: Receiver<AudioParamMessage>,
    feedback_sender: Sender<AudioEngineState>,
    feedback_receiver: Receiver<AudioEngineState>,
    
    // Configuration
    config: BridgeConfig,
    
    // State tracking
    last_feedback_time: Instant,
    feedback_interval: Duration,
    
    // Parameter queuing
    param_queue: Arc<Mutex<Vec<AudioParamMessage>>>,
    
    // Audio processing thread
    audio_thread_handle: Option<thread::JoinHandle<()>>,
    audio_running: Arc<Mutex<bool>>,
}

impl AudioEngineBridge {
    /// Create a new bridge with default configuration
    pub fn new() -> Self {
        Self::with_config(BridgeConfig::default())
    }
    
    /// Create a new bridge with custom configuration
    pub fn with_config(config: BridgeConfig) -> Self {
        let (param_sender, param_receiver) = bounded(config.control_buffer_size);
        let (feedback_sender, feedback_receiver) = bounded(config.feedback_buffer_size);
        
        let feedback_interval = Duration::from_secs_f32(1.0 / config.feedback_rate_hz);
        
        Self {
            param_sender,
            param_receiver,
            feedback_sender,
            feedback_receiver,
            config,
            last_feedback_time: Instant::now(),
            feedback_interval,
            param_queue: Arc::new(Mutex::new(Vec::new())),
            audio_thread_handle: None,
            audio_running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Send parameter message from UI to audio engine
    pub fn send_param(&self, message: AudioParamMessage) -> Result<(), Box<dyn std::error::Error>> {
        // Check queue size
        if let Ok(queue) = self.param_queue.lock() {
            if queue.len() >= self.config.max_param_queue {
                return Err("Parameter queue full".into());
            }
        }
        
        self.param_sender.try_send(message)
            .map_err(|e| format!("Failed to send parameter: {}", e).into())
    }
    
    /// Receive parameter message (for audio thread)
    pub fn receive_param(&self) -> Option<AudioParamMessage> {
        self.param_receiver.try_recv().ok()
    }
    
    /// Send feedback state from audio engine to UI
    pub fn send_feedback(&self, state: AudioEngineState) -> Result<(), Box<dyn std::error::Error>> {
        self.feedback_sender.try_send(state)
            .map_err(|e| format!("Failed to send feedback: {}", e).into())
    }
    
    /// Receive feedback state (for UI thread)
    pub fn receive_feedback(&self) -> Option<AudioEngineState> {
        self.feedback_receiver.try_recv().ok()
    }
    
    /// Check if it\'s time to send feedback (rate limiting)
    pub fn should_send_feedback(&self) -> bool {
        self.last_feedback_time.elapsed() >= self.feedback_interval
    }
    
    /// Check if UI should update (alias for should_send_feedback for UI compatibility)
    pub fn should_update_ui(&mut self) -> bool {
        let should_update = self.last_feedback_time.elapsed() >= self.feedback_interval;
        if should_update {
            self.last_feedback_time = Instant::now();
        }
        should_update
    }
    
    /// Update feedback timestamp
    pub fn update_feedback_time(&mut self) {
        self.last_feedback_time = Instant::now();
    }
    
    /// Get current configuration
    pub fn config(&self) -> &BridgeConfig {
        &self.config
    }
    
    /// Start the audio processing thread
    pub fn start_audio_thread<F>(&mut self, mut audio_processor: F) 
    where
        F: FnMut(AudioParamMessage) + Send + 'static,
    {
        if self.audio_thread_handle.is_some() {
            println!("⚠️ Audio thread already running");
            return;
        }
        
        *self.audio_running.lock().unwrap() = true;
        
        let param_receiver = self.param_receiver.clone();
        let feedback_sender = self.feedback_sender.clone();
        let audio_running = Arc::clone(&self.audio_running);
        
        self.audio_thread_handle = Some(thread::spawn(move || {
            println!("🎵 Audio processing thread started");
            
            let mut last_update_time = Instant::now();
            let mut audio_state = AudioEngineState::default();
            let processing_interval = Duration::from_millis(1); // 1ms for real-time processing
            
            loop {
                // Check if we should stop
                if !*audio_running.lock().unwrap() {
                    break;
                }
                
                let now = Instant::now();
                
                // Process pending parameters
                while let Ok(message) = param_receiver.try_recv() {
                    audio_processor(message);
                }
                
                // Update audio state periodically
                if now.duration_since(last_update_time) >= processing_interval {
                    // Simulate audio processing updates
                    audio_state.playing = true;
                    audio_state.bpm = 128.0;
                    audio_state.time_position += 0.001; // 1ms advance
                    
                    // Update audio levels (simplified)
                    if audio_state.track_peaks.is_empty() {
                        audio_state.track_peaks = vec![-12.0; 12];
                    }
                    audio_state.master_peak = -6.0;
                    
                    // Update spectrum data
                    audio_state.spectrum_data = generate_test_spectrum();
                    
                    // Send feedback if enough time has passed
                    if now.duration_since(last_update_time) >= Duration::from_millis(33) { // 30 Hz
                        if let Err(e) = feedback_sender.send(audio_state.clone()) {
                            eprintln!("Failed to send feedback: {}", e);
                        }
                    }
                    
                    last_update_time = now;
                }
                
                // Sleep to prevent busy waiting
                thread::sleep(Duration::from_micros(100)); // 100μs
            }
            
            println!("🛑 Audio processing thread stopped");
        }));
        
        println!("✅ Audio engine bridge started");
    }
    
    /// Stop the audio processing thread
    pub fn stop_audio_thread(&mut self) {
        if let Some(handle) = self.audio_thread_handle.take() {
            *self.audio_running.lock().unwrap() = false;
            
            // Wait for thread to finish (with timeout)
            let timeout = Duration::from_secs(2);
            let start = Instant::now();
            
            loop {
                if start.elapsed() >= timeout {
                    eprintln!("⚠️ Audio thread did not stop within timeout");
                    break;
                }
                
                if handle.is_finished() {
                    let _ = handle.join();
                    break;
                }
                
                thread::sleep(Duration::from_millis(10));
            }
        }
        
        println!("🛑 Audio engine bridge stopped");
    }
    
    /// Check if audio thread is running
    pub fn is_audio_running(&self) -> bool {
        *self.audio_running.lock().unwrap()
    }
    
    /// Get audio state (blocking receive with timeout)
    pub fn get_audio_state(&self, timeout_ms: u64) -> Option<AudioEngineState> {
        let timeout = Duration::from_millis(timeout_ms);
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if let Ok(state) = self.feedback_receiver.try_recv() {
                return Some(state);
            }
            thread::sleep(Duration::from_millis(1));
        }
        
        None
    }
    
    /// Get the most recent feedback state (non-blocking)
    pub fn get_feedback_bus(&self) -> Option<AudioEngineState> {
        self.receive_feedback()
    }
    
    /// Clear all pending parameter messages
    pub fn clear_param_queue(&self) {
        self.param_receiver.try_iter().for_each(|_| {});
    }
    
    /// Get number of pending parameters
    pub fn pending_params(&self) -> usize {
        self.param_receiver.len()
    }
    
    /// Get number of pending feedback messages
    pub fn pending_feedback(&self) -> usize {
        self.feedback_receiver.len()
    }
}

impl Drop for AudioEngineBridge {
    fn drop(&mut self) {
        self.stop_audio_thread();
    }
}

/// Generate test spectrum data for visualization
fn generate_test_spectrum() -> Vec<f32> {
    let mut spectrum = Vec::with_capacity(256);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    
    for i in 0..256 {
        let _freq = i as f32 * 20.0; // 20Hz to ~5kHz range
        
        // Generate a complex spectrum with multiple harmonics
        let fundamental = (2.0 * std::f64::consts::PI * 220.0 * time).sin() as f32 * 0.3;
        let harmonic2 = (2.0 * std::f64::consts::PI * 440.0 * time).sin() as f32 * 0.2;
        let harmonic3 = (2.0 * std::f64::consts::PI * 660.0 * time).sin() as f32 * 0.1;
        let noise = ((time * 1000.0 + i as f64).sin() as f32) * 0.05;
        
        let magnitude = (fundamental + harmonic2 + harmonic3 + noise).abs() * 40.0;
        spectrum.push(magnitude.max(-60.0).min(0.0));
    }
    
    spectrum
}

/// Build the audio engine with the new bridge system
pub fn build_hexodsp_engine(event_queue: Arc<EventQueue>) -> Result<super::HexoDSPEngine, Box<dyn std::error::Error>> {
    use super::{HexoDSPEngine, NodeGraph};
    use super::node_instance_manager::NodeInstanceManager;
    use std::sync::{Arc, Mutex};
    
    let engine = HexoDSPEngine::new(event_queue.clone())?;    
    // Create separate NodeGraph and AudioEngineBridge for NodeInstanceManager
    // This is separate from engine.node_graph to avoid ownership issues
    let node_graph = NodeGraph::new();
    let bridge_for_manager = AudioEngineBridge::new();
    
    // Create NodeInstanceManager for UI ↔ Audio node mapping
    let node_manager = Arc::new(Mutex::new(NodeInstanceManager::new(node_graph, bridge_for_manager)));
    let node_manager_clone = Arc::clone(&node_manager);
    
    // Start the audio engine bridge
    engine.bridge.lock().unwrap().start_audio_thread(move |message| {
        // Route node operations through NodeInstanceManager
        let mut manager = node_manager_clone.lock().unwrap();
        
        match message {
            // Node-specific operations handled by NodeInstanceManager
            AudioParamMessage::CreateNode(_, _)
            | AudioParamMessage::DeleteNode(_)
            | AudioParamMessage::ConnectNodes(_, _, _, _)
            | AudioParamMessage::DisconnectNodes(_, _)
            | AudioParamMessage::SetNodeParameter(_, _, _) => {
                let _ = manager.process_message(message);
            }
            
            // Transport operations
            AudioParamMessage::SetTempo(bpm) => {
                println!("🎼 Audio thread: Set BPM to {}", bpm);
                // In real implementation, this would update the transport
            }
            AudioParamMessage::Play => {
                println!("▶️ Audio thread: Play");
                // Start audio processing
            }
            AudioParamMessage::Stop => {
                println!("⏹️ Audio thread: Stop");
                // Stop audio processing
            }
            AudioParamMessage::MasterVolume(volume) => {
                println!("🔊 Audio thread: Master volume {}", volume);
                // Update master volume
            }
            _ => {
                println!("🎛️ Audio thread: Parameter change {:?}", message);
            }
        }
    });
    
    Ok(engine)
}

/// Audio processing loop for integration with the engine
pub fn audio_processing_loop<F>(_processor: F) -> !
where
    F: FnMut(AudioParamMessage) + Send + 'static,
{
    let mut running = true;
    
    while running {
        // Check for control messages
        // Process audio
        // Update state
        // Send feedback
        
        // In a real implementation, this would integrate with the actual
        // audio processing thread and handle real-time audio I/O
        
        thread::sleep(Duration::from_millis(1));
        
        // Check for stop signal
        if /* stop condition */ false {
            running = false;
        }
    }
    
    std::process::exit(0);
}