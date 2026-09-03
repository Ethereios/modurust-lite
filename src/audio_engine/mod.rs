/// HexoDSP Audio Engine - Real-time audio processing core
/// 
/// This module provides the foundation for low-latency, real-time audio processing
/// with proper thread safety and modern DAW capabilities.

pub mod cpal_io;
pub mod dsp_core;
pub mod node_graph;
pub mod transport;
pub mod bridge;
pub mod node_instance_manager;

use crate::event_queue::EventQueue;

use cpal_io::AudioIO;
use node_graph::NodeGraph;
use transport::Transport;
use bridge::{AudioEngineBridge, AudioParamMessage, AudioEngineState};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};

/// Main audio engine structure that coordinates all audio processing
pub struct HexoDSPEngine {
    /// Real-time audio I/O system
    pub audio_io: AudioIO,
    
    /// DSP processing core
    pub dsp_core: dsp_core::DSPCore,
    
    /// Node-based audio processing graph
    pub node_graph: NodeGraph,
    
    /// Transport and timing system
    pub transport: Transport,
    
    /// Bridge for UI communication (shared with UI)
    pub bridge: Arc<Mutex<AudioEngineBridge>>,
    
    /// Audio processing state
    pub processing_state: AudioEngineState,

    /// Map UI node IDs to audio graph node IDs
    ui_to_audio_node: HashMap<String, usize>,

    /// Track base volumes (pre mute/solo), keyed by track index
    track_base_volumes: HashMap<usize, f32>,
    /// Track mute states
    track_mutes: HashMap<usize, bool>,
    /// Set of soloed tracks
    solo_tracks: HashSet<usize>,
    /// Master mute state (gates AudioIO output)
    master_muted: bool,
}

impl HexoDSPEngine {
    /// Create a new audio engine with default settings
    pub fn new(event_queue: Arc<EventQueue>) -> Result<Self, Box<dyn std::error::Error>> {
        let audio_io = AudioIO::new(event_queue.clone())?;
        let dsp_core = dsp_core::DSPCore::new(audio_io.sample_rate(), audio_io.buffer_size());
        let node_graph = NodeGraph::new();
        let transport = Transport::new(audio_io.sample_rate());
        let bridge = Arc::new(Mutex::new(AudioEngineBridge::new()));
        
        Ok(Self {
            audio_io,
            dsp_core,
            node_graph,
            transport,
            bridge,
            processing_state: AudioEngineState::default(),
            ui_to_audio_node: HashMap::new(),
            track_base_volumes: HashMap::new(),
            track_mutes: HashMap::new(),
            solo_tracks: HashSet::new(),
            master_muted: false,
        })
    }
    
    /// Start the audio engine with real-time processing
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎵 Starting HexoDSP Audio Engine...");
        println!("Sample Rate: {} Hz", self.audio_io.sample_rate());
        println!("Buffer Size: {} samples", self.audio_io.buffer_size());
        
        // Initialize audio processing pipeline
        self.setup_default_graph()?;
        
        // Start the audio I/O system: output-only for clean audible tone feedback
        // (input monitoring toggled via AudioIO when needed)
        self.audio_io.start_output_only()?;
        
        println!("✅ HexoDSP Audio Engine started successfully");
        Ok(())
    }
    
    /// Stop the audio engine
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛑 Stopping HexoDSP Audio Engine...");
        self.audio_io.stop()?;
        Ok(())
    }
    
    /// Set up the default audio processing graph
    fn setup_default_graph(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Create default nodes: Oscillator -> Filter -> Output
        let osc_id = self.node_graph.add_oscillator();
        let filter_id = self.node_graph.add_filter();
        let output_id = self.node_graph.add_output();
        
        // Connect oscillator to filter
        self.node_graph.connect(osc_id, filter_id, "audio_out", "audio_in")?;
        
        // Connect filter to output
        self.node_graph.connect(filter_id, output_id, "audio_out", "audio_in")?;
        
        println!("🎛️ Default audio graph initialized");
        Ok(())
    }
    
    /// Process audio buffer (called from real-time audio thread)
    pub fn process_audio(&mut self, input_buffer: &[f32], output_buffer: &mut [f32]) {
        // Update transport/timing
        self.transport.update();
        
        // Process UI messages without holding the bridge lock across handler
        loop {
            let message_opt = {
                let br = self.bridge.lock().unwrap();
                br.receive_param()
            };
            match message_opt {
                Some(message) => self.handle_param_message(message),
                None => break,
            }
        }
        
        // If not playing, output silence and skip heavy processing
        if !self.transport.is_playing() {
            for s in output_buffer.iter_mut() { *s = 0.0; }
        } else {
            // Process the node graph when transport is active
            self.node_graph.process(input_buffer, output_buffer);
        }
        
        // Update audio state for UI feedback
        self.update_audio_state();
        
        // Send state back to UI
        if self.bridge.lock().unwrap().should_send_feedback() {
            let _ = self.bridge.lock().unwrap().send_feedback(self.processing_state.clone());
        }
    }
    
    /// Handle parameter changes from UI
    fn handle_param_message(&mut self, message: AudioParamMessage) {
        match message {
            AudioParamMessage::SetTempo(bpm) => {
                self.transport.set_tempo(bpm);
            }
            AudioParamMessage::SetLoop(enabled, start_beats, end_beats) => {
                // Convert beat positions to samples using current tempo
                let spb = self.transport.clock().samples_per_beat();
                let start_samples = (start_beats.max(0.0) * spb) as u64;
                let end_samples = (end_beats.max(start_beats) * spb) as u64;
                self.transport.set_loop_enabled(enabled);
                self.transport.set_loop_region(start_samples, end_samples);
            }
            AudioParamMessage::Play => {
                self.transport.play();
                // Gate CPAL output by transport state
                self.audio_io.set_playback_enabled(true);
            }
            AudioParamMessage::Stop => {
                self.transport.stop();
                self.audio_io.set_playback_enabled(false);
            }
            AudioParamMessage::Record => {
                self.transport.record();
            }
            AudioParamMessage::Pause => {
                // Pause should silence output but keep transport position
                self.transport.pause();
                self.audio_io.set_playback_enabled(false);
            }
            AudioParamMessage::SetInputMonitoring(enabled) => {
                let _ = self.audio_io.set_input_monitoring(enabled);
            }
            AudioParamMessage::MasterVolume(volume) => {
                self.node_graph.set_master_volume(volume);
                let _ = self.audio_io.set_tone_amp(volume);
            }
            AudioParamMessage::MasterPan(pan) => {
                self.node_graph.set_master_pan(pan);
                let _ = self.audio_io.set_tone_pan(pan);
            }
            AudioParamMessage::MasterMute(muted) => {
                // Gate audio output while keeping transport state unaffected
                self.master_muted = muted;
                if muted {
                    self.audio_io.set_playback_enabled(false);
                } else {
                    // Restore playback according to transport state
                    self.audio_io.set_playback_enabled(self.transport.is_playing());
                }
            }
            AudioParamMessage::TrackVolume(track, volume) => {
                // Store base volume and reapply mute/solo rules
                self.track_base_volumes.insert(track, volume);
                self.apply_mute_solo();
            }
            AudioParamMessage::TrackPan(track, pan) => {
                self.node_graph.set_track_pan(track, pan);
            }
            AudioParamMessage::TrackMute(track, muted) => {
                self.track_mutes.insert(track, muted);
                self.apply_mute_solo();
            }
            AudioParamMessage::TrackSolo(track, solo) => {
                if solo {
                    self.solo_tracks.insert(track);
                } else {
                    self.solo_tracks.remove(&track);
                }
                self.apply_mute_solo();
            }
            AudioParamMessage::ReturnVolume(bus, volume) => {
                self.node_graph.set_return_volume(bus, volume);
            }
            AudioParamMessage::ReturnPan(bus, pan) => {
                self.node_graph.set_return_pan(bus, pan);
            }
            AudioParamMessage::CreateNode(node_type, ui_node_id) => {
                let added_id = match node_type.as_str() {
                    "Oscillator" | "Sine" | "Saw" => self.node_graph.add_oscillator(),
                    "Filter" => self.node_graph.add_filter(),
                    "Delay" => self.node_graph.add_delay(),
                    "Reverb" => self.node_graph.add_reverb(),
                    "Output" => self.node_graph.add_output(),
                    "Input" => self.node_graph.add_input(),
                    _ => self.node_graph.add_oscillator(),
                };
                self.ui_to_audio_node.insert(ui_node_id.clone(), added_id);
                println!("➕ Created node {:?} -> id {} (ui {})", node_type, added_id, ui_node_id);
            }
            AudioParamMessage::DeleteNode(ui_node_id) => {
                if let Some(&node_id) = self.ui_to_audio_node.get(&ui_node_id) {
                    self.node_graph.remove_node(node_id);
                    self.ui_to_audio_node.remove(&ui_node_id);
                    println!("➖ Deleted node ui {} (id {})", ui_node_id, node_id);
                } else {
                    eprintln!("DeleteNode: unknown ui node id {}", ui_node_id);
                }
            }
            AudioParamMessage::ConnectNodes(from_ui, to_ui, from_port, to_port) => {
                match (self.ui_to_audio_node.get(&from_ui), self.ui_to_audio_node.get(&to_ui)) {
                    (Some(&from), Some(&to)) => {
                        if let Err(e) = self.node_graph.connect(from, to, &from_port, &to_port) {
                            eprintln!("Failed to connect nodes: {}", e);
                        } else {
                            println!("🔗 Connected {}:{} -> {}:{} (ui {} -> ui {})", from, from_port, to, to_port, from_ui, to_ui);
                        }
                    }
                    _ => eprintln!("ConnectNodes: unknown ui ids from={} to={}", from_ui, to_ui),
                }
            }
            AudioParamMessage::DisconnectNodes(from_ui, to_ui) => {
                match (self.ui_to_audio_node.get(&from_ui), self.ui_to_audio_node.get(&to_ui)) {
                    (Some(&from), Some(&to)) => {
                        self.node_graph.disconnect(from, to);
                        println!("🔌 Disconnected {} -> {} (ui {} -> ui {})", from, to, from_ui, to_ui);
                    }
                    _ => eprintln!("DisconnectNodes: unknown ui ids from={} to={}", from_ui, to_ui),
                }
            }
            AudioParamMessage::SetNodeParameter(ui_node_id, param, value) => {
                if let Some(&node_id) = self.ui_to_audio_node.get(&ui_node_id) {
                    self.node_graph.set_node_parameter(node_id, &param, value);
                } else {
                    eprintln!("SetNodeParameter: unknown ui node id {}", ui_node_id);
                }
            }
            AudioParamMessage::SetParameter(param, value) => {
                if param.eq_ignore_ascii_case("frequency") || param.eq_ignore_ascii_case("freq") {
                    let _ = self.audio_io.set_tone_freq(value);
                }
            }
            _ => {
                println!("🔧 Received parameter: {:?}", message);
            }
        }
    }

    /// Apply mute/solo rules to track volumes, based on stored base volumes
    fn apply_mute_solo(&mut self) {
        // Determine number of known tracks from node graph
        let current = self.node_graph.get_track_volumes();
        let track_count = current.len();
        let any_solo = !self.solo_tracks.is_empty();

        for track in 0..track_count {
            let base = self
                .track_base_volumes
                .get(&track)
                .copied()
                .unwrap_or(current.get(track).copied().unwrap_or(0.8));

            // Evaluate audible state
            let muted = self.track_mutes.get(&track).copied().unwrap_or(false);
            let audible = if muted {
                false
            } else if any_solo {
                self.solo_tracks.contains(&track)
            } else {
                true
            };

            let applied = if audible { base } else { 0.0 };
            self.node_graph.set_track_volume(track, applied);
        }
    }
    
    /// Update audio processing state for UI feedback
    fn update_audio_state(&mut self) {
        // Update master peak level
        self.processing_state.master_peak = self.node_graph.get_master_peak();
        
        // Update track peak levels
        self.processing_state.track_peaks = self.node_graph.get_track_peaks();
        // Update return bus peak levels
        self.processing_state.return_peaks = self.node_graph.get_return_peaks();
        
        // Update spectrum data
        self.processing_state.spectrum_data = self.node_graph.get_spectrum_data();
        
        // Update transport state
        self.processing_state.playing = self.transport.is_playing();
        self.processing_state.bpm = self.transport.bpm();
        self.processing_state.time_position = self.transport.time_position();

        // Publish current master params for UI sync
        self.processing_state
            .current_params
            .insert("MasterVolume".to_string(), self.node_graph.get_master_volume());
        self.processing_state
            .current_params
            .insert("MasterPan".to_string(), self.node_graph.get_master_pan());

        // Publish track and return params for UI sync
        for (i, v) in self.node_graph.get_track_volumes().iter().enumerate() {
            self.processing_state
                .current_params
                .insert(format!("TrackVolume_{}", i), *v);
        }
        for (i, p) in self.node_graph.get_track_pans().iter().enumerate() {
            self.processing_state
                .current_params
                .insert(format!("TrackPan_{}", i), *p);
        }
        for (i, v) in self.node_graph.get_return_volumes().iter().enumerate() {
            self.processing_state
                .current_params
                .insert(format!("ReturnVolume_{}", i), *v);
        }
        for (i, p) in self.node_graph.get_return_pans().iter().enumerate() {
            self.processing_state
                .current_params
                .insert(format!("ReturnPan_{}", i), *p);
        }

        // Publish UI->Audio node mapping for UI feedback if needed
        self.processing_state.node_to_audio_map = self.ui_to_audio_node.clone();
    }
}