//! MIDI 2.0 and MPE (MIDI Polyphonic Expression) Support
//!
//! This module provides comprehensive MIDI 2.0 protocol support and MPE
//! (MIDI Polyphonic Expression) for advanced controller integration.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use serde::{Deserialize, Serialize};
use crate::node_graph::{NeuroNodeId, NeuroNodeDefinition, NeuroNodePort, NeuroDataType};

/// MIDI 2.0 Message Types
#[derive(Debug, Clone, PartialEq)]
pub enum Midi2Message {
    /// Note On (with velocity)
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u16, // 0-65535 in MIDI 2.0
        attribute: MidiAttribute,
    },
    /// Note Off
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u16,
        attribute: MidiAttribute,
    },
    /// Control Change (CC)
    ControlChange {
        channel: u8,
        controller: u8,
        value: u32, // 32-bit values in MIDI 2.0
    },
    /// Pitch Bend
    PitchBend {
        channel: u8,
        value: u32, // 32-bit pitch bend
    },
    /// Channel Pressure (Aftertouch)
    ChannelPressure {
        channel: u8,
        pressure: u32,
    },
    /// Polyphonic Key Pressure (Poly Aftertouch)
    PolyPressure {
        channel: u8,
        note: u8,
        pressure: u32,
    },
    /// Program Change
    ProgramChange {
        channel: u8,
        program: u8,
        bank: Option<u16>,
    },
    /// System Exclusive
    SystemExclusive(Vec<u8>),
    /// MIDI 2.0 Per-Note Control Change
    PerNoteControl {
        channel: u8,
        note: u8,
        controller: u8,
        value: u32,
    },
    /// MIDI 2.0 Registered Per-Note Controller
    RegisteredPerNoteControl {
        channel: u8,
        note: u8,
        controller: u8,
        value: u32,
    },
}

/// MIDI 2.0 Attribute Types
#[derive(Debug, Clone, PartialEq)]
pub enum MidiAttribute {
    None,
    ManufacturerSpecific(u8),
    ProfileSpecific(u8),
    Pitch7_9(u8), // 7.9 cent pitch bend
}

/// MPE (MIDI Polyphonic Expression) Configuration
#[derive(Debug, Clone)]
pub struct MPEConfig {
    pub master_channel: u8, // Usually channel 1 (0-indexed as 0)
    pub member_channels: Vec<u8>, // Channels 2-15 for polyphonic expression
    pub pitch_bend_range: u8, // Semitones (typically 48 for full MPE range)
    pub enable_pitch_bend: bool,
    pub enable_timbre: bool,
    pub enable_pressure: bool,
}

impl Default for MPEConfig {
    fn default() -> Self {
        Self {
            master_channel: 0, // Channel 1
            member_channels: (1..16).collect(), // Channels 2-16
            pitch_bend_range: 48, // Full MPE range
            enable_pitch_bend: true,
            enable_timbre: true,
            enable_pressure: true,
        }
    }
}

/// MPE Zone Configuration (for multi-zone setups)
#[derive(Debug, Clone)]
pub struct MPEZone {
    pub lower_master: u8,
    pub upper_master: u8,
    pub lower_members: Vec<u8>,
    pub upper_members: Vec<u8>,
    pub config: MPEConfig,
}

/// MPE Voice State
#[derive(Debug, Clone)]
pub struct MPEVoice {
    pub channel: u8,
    pub note: u8,
    pub pitch_bend: f32, // -1.0 to 1.0
    pub timbre: f32, // 0.0 to 1.0
    pub pressure: f32, // 0.0 to 1.0
    pub is_active: bool,
    pub note_on_velocity: f32,
    pub note_off_velocity: f32,
}

impl Default for MPEVoice {
    fn default() -> Self {
        Self {
            channel: 0,
            note: 60,
            pitch_bend: 0.0,
            timbre: 0.0,
            pressure: 0.0,
            is_active: false,
            note_on_velocity: 0.0,
            note_off_velocity: 0.0,
        }
    }
}

/// MIDI 2.0 Device Profile
#[derive(Debug, Clone)]
pub struct Midi2DeviceProfile {
    pub manufacturer_id: u16,
    pub device_id: u16,
    pub profile_name: String,
    pub supported_channels: Vec<u8>,
    pub max_simultaneous_notes: u16,
    pub supports_mpe: bool,
    pub mpe_config: Option<MPEConfig>,
}

/// MIDI 2.0 Processor
pub struct Midi2Processor {
    input_devices: HashMap<String, Midi2Device>,
    output_devices: HashMap<String, Midi2Device>,
    active_voices: HashMap<u8, MPEVoice>, // Keyed by channel
    mpe_zones: Vec<MPEZone>,
    global_mpe_config: MPEConfig,
    message_queue: VecDeque<Midi2Message>,
    running: Arc<AtomicBool>,
}

impl Midi2Processor {
    pub fn new() -> Self {
        Self {
            input_devices: HashMap::new(),
            output_devices: HashMap::new(),
            active_voices: HashMap::new(),
            mpe_zones: Vec::new(),
            global_mpe_config: MPEConfig::default(),
            message_queue: VecDeque::with_capacity(1024),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start MIDI processing
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, Ordering::SeqCst);
        println!("MIDI 2.0 processor started");
        Ok(())
    }

    /// Stop MIDI processing
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, Ordering::SeqCst);
        // Clear all active voices
        self.active_voices.clear();
        println!("MIDI 2.0 processor stopped");
        Ok(())
    }

    /// Process incoming MIDI 2.0 message
    pub fn process_message(&mut self, message: Midi2Message) -> Result<(), Box<dyn std::error::Error>> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        match &message {
            Midi2Message::NoteOn { channel, note, velocity, .. } => {
                self.handle_note_on(*channel, *note, *velocity as f32 / 65535.0)?;
            }
            Midi2Message::NoteOff { channel, note, velocity, .. } => {
                self.handle_note_off(*channel, *note, *velocity as f32 / 65535.0)?;
            }
            Midi2Message::PerNoteControl { channel, note, controller, value } => {
                self.handle_per_note_control(*channel, *note, *controller, *value as f32 / 4294967295.0)?;
            }
            Midi2Message::PitchBend { channel, value } => {
                self.handle_pitch_bend(*channel, *value)?;
            }
            Midi2Message::ChannelPressure { channel, pressure } => {
                self.handle_channel_pressure(*channel, *pressure)?;
            }
            Midi2Message::PolyPressure { channel, note, pressure } => {
                self.handle_poly_pressure(*channel, *note, *pressure)?;
            }
            _ => {} // Handle other message types as needed
        }

        // Queue message for further processing
        self.message_queue.push_back(message);

        // Keep queue size manageable
        while self.message_queue.len() > 1024 {
            self.message_queue.pop_front();
        }

        Ok(())
    }

    /// Handle Note On event
    fn handle_note_on(&mut self, channel: u8, note: u8, velocity: f32) -> Result<(), Box<dyn std::error::Error>> {
        let voice = self.active_voices.entry(channel).or_insert(MPEVoice::default());
        voice.channel = channel;
        voice.note = note;
        voice.is_active = true;
        voice.note_on_velocity = velocity;
        voice.pitch_bend = 0.0; // Reset pitch bend for new note
        voice.timbre = 0.0;
        voice.pressure = 0.0;

        println!("MPE Note On: Channel {}, Note {}, Velocity {:.3}", channel, note, velocity);
        Ok(())
    }

    /// Handle Note Off event
    fn handle_note_off(&mut self, channel: u8, note: u8, velocity: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(voice) = self.active_voices.get_mut(&channel) {
            if voice.note == note {
                voice.is_active = false;
                voice.note_off_velocity = velocity;
                println!("MPE Note Off: Channel {}, Note {}, Velocity {:.3}", channel, note, velocity);
            }
        }
        Ok(())
    }

    /// Handle Per-Note Control Change (MPE)
    fn handle_per_note_control(&mut self, channel: u8, note: u8, controller: u8, value: f32) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(voice) = self.active_voices.get_mut(&channel) {
            if voice.note == note {
                match controller {
                    74 => { // Timbre (MPE CC 74)
                        voice.timbre = value;
                        println!("MPE Timbre: Channel {}, Note {}, Value {:.3}", channel, note, value);
                    }
                    _ => {} // Handle other per-note controllers
                }
            }
        }
        Ok(())
    }

    /// Handle Pitch Bend (MPE)
    fn handle_pitch_bend(&mut self, channel: u8, value: u32) -> Result<(), Box<dyn std::error::Error>> {
        let normalized_pitch = (value as f32 / 4294967295.0) * 2.0 - 1.0; // Convert to -1.0 to 1.0

        if let Some(voice) = self.active_voices.get_mut(&channel) {
            voice.pitch_bend = normalized_pitch;
            println!("MPE Pitch Bend: Channel {}, Value {:.3}", channel, normalized_pitch);
        }
        Ok(())
    }

    /// Handle Channel Pressure (MPE)
    fn handle_channel_pressure(&mut self, channel: u8, pressure: u32) -> Result<(), Box<dyn std::error::Error>> {
        let normalized_pressure = pressure as f32 / 4294967295.0;

        if let Some(voice) = self.active_voices.get_mut(&channel) {
            voice.pressure = normalized_pressure;
            println!("MPE Channel Pressure: Channel {}, Value {:.3}", channel, normalized_pressure);
        }
        Ok(())
    }

    /// Handle Polyphonic Pressure (MPE)
    fn handle_poly_pressure(&mut self, channel: u8, note: u8, pressure: u32) -> Result<(), Box<dyn std::error::Error>> {
        let normalized_pressure = pressure as f32 / 4294967295.0;

        if let Some(voice) = self.active_voices.get_mut(&channel) {
            if voice.note == note {
                voice.pressure = normalized_pressure;
                println!("MPE Poly Pressure: Channel {}, Note {}, Value {:.3}", channel, note, normalized_pressure);
            }
        }
        Ok(())
    }

    /// Get active MPE voices
    pub fn get_active_voices(&self) -> Vec<&MPEVoice> {
        self.active_voices.values().filter(|v| v.is_active).collect()
    }

    /// Configure MPE zones
    pub fn configure_mpe_zones(&mut self, zones: Vec<MPEZone>) {
        self.mpe_zones = zones;
        println!("Configured {} MPE zones", self.mpe_zones.len());
    }

    /// Get next queued message
    pub fn next_message(&mut self) -> Option<Midi2Message> {
        self.message_queue.pop_front()
    }

    /// Check if processor is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get MPE configuration
    pub fn get_mpe_config(&self) -> &MPEConfig {
        &self.global_mpe_config
    }

    /// Set MPE configuration
    pub fn set_mpe_config(&mut self, config: MPEConfig) {
        self.global_mpe_config = config;
        println!("Updated MPE configuration");
    }
}

/// MIDI 2.0 Device
#[derive(Debug, Clone)]
pub struct Midi2Device {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub profile: Midi2DeviceProfile,
    pub is_connected: bool,
}

/// MIDI 2.0 Device Manager
pub struct Midi2DeviceManager {
    devices: HashMap<String, Midi2Device>,
    processor: Arc<Mutex<Midi2Processor>>,
}

impl Midi2DeviceManager {
    pub fn new(processor: Arc<Mutex<Midi2Processor>>) -> Self {
        Self {
            devices: HashMap::new(),
            processor,
        }
    }

    /// Enumerate available MIDI 2.0 devices
    pub fn enumerate_devices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In a real implementation, this would query the system for MIDI devices
        // For now, we'll add some placeholder devices

        let devices = vec![
            Midi2Device {
                id: "mpe_controller_1".to_string(),
                name: "MPE Touch Controller".to_string(),
                manufacturer: "Roli".to_string(),
                profile: Midi2DeviceProfile {
                    manufacturer_id: 0x1234,
                    device_id: 0x5678,
                    profile_name: "MPE Controller".to_string(),
                    supported_channels: (0..16).collect(),
                    max_simultaneous_notes: 15,
                    supports_mpe: true,
                    mpe_config: Some(MPEConfig::default()),
                },
                is_connected: true,
            },
            Midi2Device {
                id: "midi2_keyboard".to_string(),
                name: "MIDI 2.0 Keyboard".to_string(),
                manufacturer: "Generic".to_string(),
                profile: Midi2DeviceProfile {
                    manufacturer_id: 0x0000,
                    device_id: 0x0001,
                    profile_name: "MIDI 2.0 Keyboard".to_string(),
                    supported_channels: vec![0], // Single channel
                    max_simultaneous_notes: 128,
                    supports_mpe: false,
                    mpe_config: None,
                },
                is_connected: true,
            },
        ];

        for device in devices {
            self.devices.insert(device.id.clone(), device);
        }

        println!("Found {} MIDI 2.0 devices", self.devices.len());
        Ok(())
    }

    /// Connect to a MIDI device
    pub fn connect_device(&mut self, device_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.is_connected = true;
            println!("Connected to MIDI device: {}", device.name);

            // If device supports MPE, configure it
            if let Some(mpe_config) = &device.profile.mpe_config {
                let mut processor = self.processor.lock().unwrap();
                processor.set_mpe_config(mpe_config.clone());
            }
        } else {
            return Err(format!("Device {} not found", device_id).into());
        }
        Ok(())
    }

    /// Disconnect from a MIDI device
    pub fn disconnect_device(&mut self, device_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.is_connected = false;
            println!("Disconnected from MIDI device: {}", device.name);
        }
        Ok(())
    }

    /// Get list of devices
    pub fn get_devices(&self) -> Vec<&Midi2Device> {
        self.devices.values().collect()
    }

    /// Get connected devices
    pub fn get_connected_devices(&self) -> Vec<&Midi2Device> {
        self.devices.values().filter(|d| d.is_connected).collect()
    }
}

/// MIDI 2.0 to Legacy MIDI Converter
pub struct Midi2ToMidi1Converter {
    processor: Arc<Mutex<Midi2Processor>>,
}

impl Midi2ToMidi1Converter {
    pub fn new(processor: Arc<Mutex<Midi2Processor>>) -> Self {
        Self { processor }
    }

    /// Convert MIDI 2.0 message to MIDI 1.0
    pub fn convert_message(&self, message: &Midi2Message) -> Vec<u8> {
        match message {
            Midi2Message::NoteOn { channel, note, velocity, .. } => {
                // Convert 16-bit velocity to 7-bit
                let midi1_velocity = ((velocity >> 9) & 0x7F) as u8;
                vec![0x90 | channel, *note, midi1_velocity]
            }
            Midi2Message::NoteOff { channel, note, .. } => {
                vec![0x80 | channel, *note, 0x40] // Standard note off
            }
            Midi2Message::ControlChange { channel, controller, value } => {
                // Convert 32-bit value to 7-bit
                let midi1_value = ((value >> 25) & 0x7F) as u8;
                vec![0xB0 | channel, *controller, midi1_value]
            }
            Midi2Message::PitchBend { channel, value } => {
                // Convert 32-bit pitch bend to 14-bit
                let midi1_pitch = ((value >> 18) & 0x3FFF) as u16;
                let lsb = (midi1_pitch & 0x7F) as u8;
                let msb = ((midi1_pitch >> 7) & 0x7F) as u8;
                vec![0xE0 | channel, lsb, msb]
            }
            Midi2Message::ProgramChange { channel, program, .. } => {
                vec![0xC0 | channel, *program]
            }
            _ => vec![], // Unsupported message types
        }
    }

    /// Convert MPE voice data to MIDI 1.0 CC messages
    pub fn convert_mpe_voice(&self, voice: &MPEVoice) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        // Pitch bend (14-bit)
        let pitch_bend_14bit = ((voice.pitch_bend + 1.0) * 8191.0) as u16;
        let lsb = (pitch_bend_14bit & 0x7F) as u8;
        let msb = ((pitch_bend_14bit >> 7) & 0x7F) as u8;
        messages.push(vec![0xE0 | voice.channel, lsb, msb]);

        // Timbre as CC 74
        let timbre_7bit = (voice.timbre * 127.0) as u8;
        messages.push(vec![0xB0 | voice.channel, 74, timbre_7bit]);

        // Pressure as channel pressure
        let pressure_7bit = (voice.pressure * 127.0) as u8;
        messages.push(vec![0xD0 | voice.channel, pressure_7bit]);

        messages
    }
}

/// Create MIDI 2.0 node registry
pub fn create_midi2_node_registry() -> HashMap<String, Box<dyn Fn(NeuroNodeId) -> NeuroNodeDefinition + Send + Sync>> {
    let mut registry: HashMap<String, Box<dyn Fn(NeuroNodeId) -> NeuroNodeDefinition + Send + Sync>> = HashMap::new();

    // MIDI 2.0 Input Node
    let midi_input_factory = |id: NeuroNodeId| -> NeuroNodeDefinition {
        NeuroNodeDefinition {
            id,
            name: "MIDI 2.0 Input".to_string(),
            node_type: "midi2.input".to_string(),
            inputs: vec![],
            outputs: vec![
                NeuroNodePort { name: "midi_messages".to_string(), data_type: NeuroDataType::Array, required: true },
                NeuroNodePort { name: "mpe_voices".to_string(), data_type: NeuroDataType::Array, required: false },
            ],
        }
    };

    // MIDI 2.0 Output Node
    let midi_output_factory = |id: NeuroNodeId| -> NeuroNodeDefinition {
        NeuroNodeDefinition {
            id,
            name: "MIDI 2.0 Output".to_string(),
            node_type: "midi2.output".to_string(),
            inputs: vec![
                NeuroNodePort { name: "midi_messages".to_string(), data_type: NeuroDataType::Array, required: true },
            ],
            outputs: vec![],
        }
    };

    // MPE Processor Node
    let mpe_processor_factory = |id: NeuroNodeId| -> NeuroNodeDefinition {
        NeuroNodeDefinition {
            id,
            name: "MPE Processor".to_string(),
            node_type: "midi2.mpe_processor".to_string(),
            inputs: vec![
                NeuroNodePort { name: "midi_in".to_string(), data_type: NeuroDataType::Array, required: true },
                NeuroNodePort { name: "pitch_bend_range".to_string(), data_type: NeuroDataType::Float, required: false },
            ],
            outputs: vec![
                NeuroNodePort { name: "midi_out".to_string(), data_type: NeuroDataType::Array, required: true },
                NeuroNodePort { name: "voice_data".to_string(), data_type: NeuroDataType::Array, required: false },
            ],
        }
    };

    registry.insert("midi2.input".to_string(), Box::new(midi_input_factory));
    registry.insert("midi2.output".to_string(), Box::new(midi_output_factory));
    registry.insert("midi2.mpe_processor".to_string(), Box::new(mpe_processor_factory));

    registry
}