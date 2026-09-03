use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Comprehensive MIDI control system for hardware controller integration
/// Supports CC mapping, parameter automation, and advanced MIDI routing

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiControlSystem {
    pub devices: HashMap<String, MidiDevice>,
    pub mappings: HashMap<Uuid, MidiMapping>,
    pub control_surfaces: HashMap<String, ControlSurface>,
    pub midi_router: MidiRouter,
    pub learn_mode: LearnMode,
    pub global_settings: GlobalMidiSettings,
    pub presets: HashMap<String, MidiControlPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiDevice {
    pub id: String,
    pub name: String,
    pub device_type: MidiDeviceType,
    pub input_ports: Vec<MidiPort>,
    pub output_ports: Vec<MidiPort>,
    pub connected: bool,
    pub latency_ms: f32,
    pub clock_sync: bool,
    pub device_info: DeviceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidiDeviceType {
    Controller,     // MIDI controller (keyboard, pad controller, etc.)
    Interface,      // MIDI interface
    Synthesizer,    // Hardware synthesizer
    Drum,          // Drum machine
    Sequencer,     // Hardware sequencer
    Computer,      // Computer/DAW
    Generic,       // Generic MIDI device
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiPort {
    pub id: String,
    pub name: String,
    pub port_type: PortType,
    pub channel_mask: u16, // Bitmask for enabled channels (0-15)
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortType {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub version: String,
    pub serial_number: Option<String>,
    pub capabilities: DeviceCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub supports_mpe: bool,
    pub supports_midi2: bool,
    pub max_polyphony: Option<u8>,
    pub supported_ccs: Vec<u8>,
    pub has_aftertouch: bool,
    pub has_pitchbend: bool,
    pub has_program_change: bool,
    pub custom_sysex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiMapping {
    pub id: Uuid,
    pub name: String,
    pub source: MidiSource,
    pub target: MidiTarget,
    pub mapping_type: MappingType,
    pub enabled: bool,
    pub learn_active: bool,
    pub curve: MappingCurve,
    pub range: (f32, f32), // Min/max output range
    pub invert: bool,
    pub smoothing: f32,    // Smoothing factor (0.0-1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiSource {
    pub device_id: String,
    pub port_id: String,
    pub channel: u8,
    pub message_type: MidiMessageType,
    pub data1: Option<u8>, // CC number, note number, etc.
    pub data2_range: Option<(u8, u8)>, // Value range
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MidiMessageType {
    NoteOn,
    NoteOff,
    ControlChange,
    PitchBend,
    Aftertouch,
    ChannelPressure,
    ProgramChange,
    SysEx,
    Clock,
    Start,
    Stop,
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiTarget {
    pub target_type: TargetType,
    pub parameter_path: String, // Hierarchical parameter path
    pub channel: Option<u8>,
    pub automation_lane: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    // DAW parameters
    MasterVolume,
    MasterPan,
    ChannelVolume(u8),
    ChannelPan(u8),
    ChannelMute(u8),
    ChannelSolo(u8),
    ChannelSend(u8, u8), // Channel, send number
    
    // Transport controls
    Play,
    Stop,
    Record,
    Loop,
    Tempo,
    
    // Plugin parameters
    PluginParameter { plugin_id: Uuid, parameter_index: u32 },
    
    // Mixer controls
    EQGain(u8, u8),     // Channel, band
    EQFreq(u8, u8),     // Channel, band
    EQQ(u8, u8),        // Channel, band
    CompThreshold(u8),
    CompRatio(u8),
    CompAttack(u8),
    CompRelease(u8),
    
    // Modular system
    ModularNodeParameter { node_id: Uuid, parameter: String },
    
    // Custom parameters
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingType {
    Direct,         // Direct 1:1 mapping
    Scaled,         // Scaled to range
    Exponential,    // Exponential curve
    Logarithmic,    // Logarithmic curve
    Toggle,         // Toggle on/off
    Trigger,        // Trigger action
    Relative,       // Relative change (encoders)
    Absolute,       // Absolute value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingCurve {
    pub curve_type: CurveType,
    pub points: Vec<CurvePoint>,
    pub smoothing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Linear,
    Exponential(f32),   // Exponent
    Logarithmic(f32),   // Base
    SCurve,             // S-shaped curve
    Custom,             // Custom curve points
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub input: f32,     // Input value (0.0-1.0)
    pub output: f32,    // Output value (0.0-1.0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSurface {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub template: SurfaceTemplate,
    pub controls: HashMap<String, SurfaceControl>,
    pub pages: Vec<ControlPage>,
    pub current_page: usize,
    pub feedback_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurfaceTemplate {
    // Popular control surfaces
    AbletonPush2,
    NovationLaunchpad,
    AkaiMPC,
    NativeInstrumentsMaschine,
    ArturiaKeylab,
    PresonusFaderPort,
    BehringerXTouch,
    
    // Generic templates
    GenericKeyboard,
    GenericPadController,
    GenericMixer,
    GenericTransport,
    
    // Custom template
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceControl {
    pub id: String,
    pub name: String,
    pub control_type: ControlType,
    pub position: (f32, f32), // X, Y position on surface
    pub size: (f32, f32),     // Width, height
    pub midi_mapping: Option<Uuid>,
    pub feedback_mapping: Option<MidiFeedback>,
    pub color: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlType {
    Fader,
    Knob,
    Button,
    Pad,
    Key,
    Encoder,
    Touchstrip,
    XYPad,
    Jog,
    Display,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiFeedback {
    pub message_type: MidiMessageType,
    pub channel: u8,
    pub data1: u8,
    pub feedback_type: FeedbackType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    LED,            // Simple LED on/off
    RGB,            // RGB color feedback
    Value,          // Numeric value feedback
    Text,           // Text display feedback
    Meter,          // Level meter feedback
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPage {
    pub id: String,
    pub name: String,
    pub controls: Vec<String>, // Control IDs
    pub color_scheme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiRouter {
    pub routes: Vec<MidiRoute>,
    pub filters: Vec<MidiFilter>,
    pub transformers: Vec<MidiTransformer>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiRoute {
    pub id: Uuid,
    pub name: String,
    pub source: RouteSource,
    pub destination: RouteDestination,
    pub enabled: bool,
    pub channel_map: Option<HashMap<u8, u8>>, // Input channel -> Output channel
    pub filters: Vec<Uuid>,
    pub transformers: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSource {
    pub device_id: String,
    pub port_id: String,
    pub channels: Vec<u8>, // Source channels
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDestination {
    pub device_id: String,
    pub port_id: String,
    pub channels: Vec<u8>, // Destination channels
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiFilter {
    pub id: Uuid,
    pub name: String,
    pub filter_type: FilterType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterType {
    MessageType(Vec<MidiMessageType>), // Allow only these message types
    Channel(Vec<u8>),                  // Allow only these channels
    CCRange(u8, u8),                   // Allow CC numbers in range
    NoteRange(u8, u8),                 // Allow notes in range
    VelocityRange(u8, u8),             // Allow velocities in range
    Custom(String),                    // Custom filter expression
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiTransformer {
    pub id: Uuid,
    pub name: String,
    pub transform_type: TransformType,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformType {
    ChannelMap(HashMap<u8, u8>),       // Map input channels to output channels
    Transpose(i8),                     // Transpose notes by semitones
    VelocityScale(f32),                // Scale velocity by factor
    CCRemap(HashMap<u8, u8>),          // Remap CC numbers
    Delay(f32),                        // Delay messages by milliseconds
    Quantize(f32),                     // Quantize timing to grid
    Humanize(f32),                     // Add random timing/velocity variation
    Custom(String),                    // Custom transformation script
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnMode {
    pub active: bool,
    pub target_mapping: Option<Uuid>,
    pub timeout_ms: u32,
    pub auto_assign: bool,
    pub learn_type: LearnType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearnType {
    SingleParameter,    // Learn single parameter
    MultiParameter,     // Learn multiple parameters
    ControlSurface,     // Learn entire control surface
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMidiSettings {
    pub clock_source: ClockSource,
    pub sync_mode: SyncMode,
    pub latency_compensation: bool,
    pub auto_latency_detection: bool,
    pub buffer_size: u32,
    pub sample_rate: u32,
    pub midi_thru: bool,
    pub panic_on_stop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClockSource {
    Internal,
    External(String), // Device ID
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    None,
    MidiClock,
    MTC,        // MIDI Time Code
    MMC,        // MIDI Machine Control
    Link,       // Ableton Link
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiControlPreset {
    pub name: String,
    pub description: String,
    pub mappings: Vec<Uuid>,
    pub control_surface_config: Option<String>,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
}

impl MidiControlSystem {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            mappings: HashMap::new(),
            control_surfaces: HashMap::new(),
            midi_router: MidiRouter::new(),
            learn_mode: LearnMode::default(),
            global_settings: GlobalMidiSettings::default(),
            presets: HashMap::new(),
        }
    }

    pub fn scan_devices(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // In a real implementation, this would scan for MIDI devices
        let mock_devices = vec![
            "USB MIDI Interface".to_string(),
            "Arturia KeyLab 61".to_string(),
            "Novation Launchpad Pro".to_string(),
            "Akai MPK Mini".to_string(),
            "Native Instruments Maschine".to_string(),
        ];

        for device_name in &mock_devices {
            let device = MidiDevice {
                id: device_name.clone(),
                name: device_name.clone(),
                device_type: self.detect_device_type(device_name),
                input_ports: vec![
                    MidiPort {
                        id: format!("{}_in", device_name),
                        name: format!("{} Input", device_name),
                        port_type: PortType::Input,
                        channel_mask: 0xFFFF, // All channels enabled
                        active: true,
                    }
                ],
                output_ports: vec![
                    MidiPort {
                        id: format!("{}_out", device_name),
                        name: format!("{} Output", device_name),
                        port_type: PortType::Output,
                        channel_mask: 0xFFFF,
                        active: true,
                    }
                ],
                connected: true,
                latency_ms: 5.0,
                clock_sync: false,
                device_info: DeviceInfo {
                    manufacturer: "Unknown".to_string(),
                    model: device_name.clone(),
                    version: "1.0".to_string(),
                    serial_number: None,
                    capabilities: DeviceCapabilities::default(),
                },
            };

            self.devices.insert(device_name.clone(), device);
        }

        println!("🎹 Found {} MIDI devices", mock_devices.len());
        Ok(mock_devices)
    }

    fn detect_device_type(&self, device_name: &str) -> MidiDeviceType {
        let name_lower = device_name.to_lowercase();
        
        if name_lower.contains("keyboard") || name_lower.contains("keylab") || name_lower.contains("mpk") {
            MidiDeviceType::Controller
        } else if name_lower.contains("launchpad") || name_lower.contains("pad") {
            MidiDeviceType::Controller
        } else if name_lower.contains("maschine") {
            MidiDeviceType::Drum
        } else if name_lower.contains("interface") {
            MidiDeviceType::Interface
        } else {
            MidiDeviceType::Generic
        }
    }

    pub fn create_mapping(&mut self, name: String, source: MidiSource, target: MidiTarget) -> Uuid {
        let mapping_id = Uuid::new_v4();
        let mapping = MidiMapping {
            id: mapping_id,
            name,
            source,
            target,
            mapping_type: MappingType::Direct,
            enabled: true,
            learn_active: false,
            curve: MappingCurve::linear(),
            range: (0.0, 1.0),
            invert: false,
            smoothing: 0.0,
        };

        self.mappings.insert(mapping_id, mapping);
        mapping_id
    }

    pub fn start_learn_mode(&mut self, target_mapping: Uuid) {
        self.learn_mode.active = true;
        self.learn_mode.target_mapping = Some(target_mapping);
        println!("🎛️ MIDI Learn mode activated - move a control on your MIDI device");
    }

    pub fn stop_learn_mode(&mut self) {
        self.learn_mode.active = false;
        self.learn_mode.target_mapping = None;
        println!("🎛️ MIDI Learn mode deactivated");
    }

    pub fn process_midi_message(&mut self, device_id: &str, message: MidiMessage) -> Vec<ParameterChange> {
        let mut parameter_changes = Vec::new();

        // Check if we're in learn mode
        if self.learn_mode.active {
            if let Some(target_mapping_id) = self.learn_mode.target_mapping {
                self.assign_learned_control(target_mapping_id, device_id, &message);
                self.stop_learn_mode();
            }
        }

        // Process existing mappings
        for mapping in self.mappings.values() {
            if mapping.enabled && self.message_matches_source(&message, &mapping.source, device_id) {
                if let Some(change) = self.apply_mapping(mapping, &message) {
                    parameter_changes.push(change);
                }
            }
        }

        parameter_changes
    }

    fn message_matches_source(&self, message: &MidiMessage, source: &MidiSource, device_id: &str) -> bool {
        if source.device_id != device_id {
            return false;
        }

        match (&message.message_type, &source.message_type) {
            (MidiMessageType::ControlChange, MidiMessageType::ControlChange) => {
                message.channel == source.channel && 
                source.data1.map_or(true, |cc| cc == message.data1)
            },
            (MidiMessageType::NoteOn, MidiMessageType::NoteOn) => {
                message.channel == source.channel &&
                source.data1.map_or(true, |note| note == message.data1)
            },
            (MidiMessageType::PitchBend, MidiMessageType::PitchBend) => {
                message.channel == source.channel
            },
            _ => message.message_type == source.message_type && message.channel == source.channel,
        }
    }

    fn apply_mapping(&self, mapping: &MidiMapping, message: &MidiMessage) -> Option<ParameterChange> {
        let input_value = self.normalize_midi_value(message);
        let mapped_value = self.apply_curve(&mapping.curve, input_value);
        let final_value = self.scale_to_range(mapped_value, mapping.range);

        Some(ParameterChange {
            target: mapping.target.clone(),
            value: if mapping.invert { 1.0 - final_value } else { final_value },
            smoothing: mapping.smoothing,
        })
    }

    fn normalize_midi_value(&self, message: &MidiMessage) -> f32 {
        match message.message_type {
            MidiMessageType::ControlChange => message.data2 as f32 / 127.0,
            MidiMessageType::NoteOn => message.data2 as f32 / 127.0, // Velocity
            MidiMessageType::PitchBend => {
                let pitch_value = ((message.data2 as u16) << 7) | (message.data1 as u16);
                (pitch_value as f32 - 8192.0) / 8192.0 // Normalize to -1.0 to 1.0
            },
            _ => 0.0,
        }
    }

    fn apply_curve(&self, curve: &MappingCurve, input: f32) -> f32 {
        match curve.curve_type {
            CurveType::Linear => input,
            CurveType::Exponential(exp) => input.powf(exp),
            CurveType::Logarithmic(base) => input.log(base),
            CurveType::SCurve => {
                // S-curve using sigmoid function
                let x = (input - 0.5) * 6.0; // Scale to -3 to 3
                1.0 / (1.0 + (-x).exp())
            },
            CurveType::Custom => {
                // Interpolate between custom curve points
                if curve.points.len() < 2 {
                    return input;
                }

                for i in 0..curve.points.len() - 1 {
                    let p1 = &curve.points[i];
                    let p2 = &curve.points[i + 1];
                    
                    if input >= p1.input && input <= p2.input {
                        let t = (input - p1.input) / (p2.input - p1.input);
                        return p1.output + t * (p2.output - p1.output);
                    }
                }
                
                input
            },
        }
    }

    fn scale_to_range(&self, value: f32, range: (f32, f32)) -> f32 {
        range.0 + value * (range.1 - range.0)
    }

    fn assign_learned_control(&mut self, mapping_id: Uuid, device_id: &str, message: &MidiMessage) {
        if let Some(mapping) = self.mappings.get_mut(&mapping_id) {
            mapping.source = MidiSource {
                device_id: device_id.to_string(),
                port_id: format!("{}_in", device_id),
                channel: message.channel,
                message_type: message.message_type.clone(),
                data1: Some(message.data1),
                data2_range: None,
            };
            
            println!("🎛️ Learned control: {} CC{} -> {}", 
                device_id, message.data1, mapping.name);
        }
    }

    pub fn create_control_surface(&mut self, template: SurfaceTemplate, device_id: String) -> String {
        let surface_id = format!("surface_{}", device_id);
        let surface = ControlSurface::from_template(surface_id.clone(), template, device_id);
        self.control_surfaces.insert(surface_id.clone(), surface);
        surface_id
    }

    pub fn save_preset(&self, name: String, mappings: Vec<Uuid>) -> Result<(), String> {
        let _preset = MidiControlPreset {
            name: name.clone(),
            description: "User created preset".to_string(),
            mappings,
            control_surface_config: None,
            author: "User".to_string(),
            version: "1.0".to_string(),
            tags: vec!["user".to_string()],
        };

        // In a real implementation, this would save to file
        println!("💾 Saved MIDI control preset: {}", name);
        Ok(())
    }

    pub fn load_preset(&mut self, name: &str) -> Result<(), String> {
        // In a real implementation, this would load from file
        println!("📁 Loaded MIDI control preset: {}", name);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MidiMessage {
    pub message_type: MidiMessageType,
    pub channel: u8,
    pub data1: u8,
    pub data2: u8,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ParameterChange {
    pub target: MidiTarget,
    pub value: f32,
    pub smoothing: f32,
}

impl MappingCurve {
    pub fn linear() -> Self {
        Self {
            curve_type: CurveType::Linear,
            points: vec![
                CurvePoint { input: 0.0, output: 0.0 },
                CurvePoint { input: 1.0, output: 1.0 },
            ],
            smoothing: 0.0,
        }
    }

    pub fn exponential(exponent: f32) -> Self {
        Self {
            curve_type: CurveType::Exponential(exponent),
            points: Vec::new(),
            smoothing: 0.0,
        }
    }

    pub fn s_curve() -> Self {
        Self {
            curve_type: CurveType::SCurve,
            points: Vec::new(),
            smoothing: 0.0,
        }
    }
}

impl ControlSurface {
    pub fn from_template(id: String, template: SurfaceTemplate, _device_id: String) -> Self {
        let (name, controls) = match template {
            SurfaceTemplate::AbletonPush2 => {
                ("Ableton Push 2".to_string(), Self::create_push2_controls())
            },
            SurfaceTemplate::NovationLaunchpad => {
                ("Novation Launchpad".to_string(), Self::create_launchpad_controls())
            },
            SurfaceTemplate::GenericKeyboard => {
                ("Generic Keyboard".to_string(), Self::create_keyboard_controls())
            },
            _ => {
                ("Generic Controller".to_string(), HashMap::new())
            },
        };

        Self {
            id,
            name,
            manufacturer: "Generic".to_string(),
            model: "Controller".to_string(),
            template,
            controls,
            pages: vec![
                ControlPage {
                    id: "main".to_string(),
                    name: "Main".to_string(),
                    controls: Vec::new(),
                    color_scheme: "default".to_string(),
                }
            ],
            current_page: 0,
            feedback_enabled: true,
        }
    }

    fn create_push2_controls() -> HashMap<String, SurfaceControl> {
        let mut controls = HashMap::new();
        
        // Create 8 encoders
        for i in 0..8 {
            controls.insert(
                format!("encoder_{}", i),
                SurfaceControl {
                    id: format!("encoder_{}", i),
                    name: format!("Encoder {}", i + 1),
                    control_type: ControlType::Encoder,
                    position: (i as f32 * 50.0, 0.0),
                    size: (40.0, 40.0),
                    midi_mapping: None,
                    feedback_mapping: None,
                    color: Some("#4FC3F7".to_string()),
                    label: format!("ENC{}", i + 1),
                }
            );
        }

        // Create 8 faders
        for i in 0..8 {
            controls.insert(
                format!("fader_{}", i),
                SurfaceControl {
                    id: format!("fader_{}", i),
                    name: format!("Fader {}", i + 1),
                    control_type: ControlType::Fader,
                    position: (i as f32 * 50.0, 100.0),
                    size: (30.0, 100.0),
                    midi_mapping: None,
                    feedback_mapping: None,
                    color: Some("#FF6B35".to_string()),
                    label: format!("CH{}", i + 1),
                }
            );
        }

        controls
    }

    fn create_launchpad_controls() -> HashMap<String, SurfaceControl> {
        let mut controls = HashMap::new();
        
        // Create 8x8 pad grid
        for row in 0..8 {
            for col in 0..8 {
                let pad_id = format!("pad_{}_{}", row, col);
                controls.insert(
                    pad_id.clone(),
                    SurfaceControl {
                        id: pad_id,
                        name: format!("Pad {},{}", row, col),
                        control_type: ControlType::Pad,
                        position: (col as f32 * 40.0, row as f32 * 40.0),
                        size: (35.0, 35.0),
                        midi_mapping: None,
                        feedback_mapping: Some(MidiFeedback {
                            message_type: MidiMessageType::NoteOn,
                            channel: 0,
                            data1: row * 8 + col,
                            feedback_type: FeedbackType::RGB,
                        }),
                        color: Some("#00FF00".to_string()),
                        label: String::new(),
                    }
                );
            }
        }

        controls
    }

    fn create_keyboard_controls() -> HashMap<String, SurfaceControl> {
        let mut controls = HashMap::new();
        
        // Create 88 keys (full piano keyboard)
        for i in 0..88 {
            let note = i + 21; // A0 = 21
            let is_black_key = [1, 3, 6, 8, 10].contains(&(note % 12));
            
            controls.insert(
                format!("key_{}", note),
                SurfaceControl {
                    id: format!("key_{}", note),
                    name: format!("Key {}", note),
                    control_type: ControlType::Key,
                    position: (i as f32 * 10.0, if is_black_key { 0.0 } else { 30.0 }),
                    size: (if is_black_key { 8.0 } else { 10.0 }, if is_black_key { 30.0 } else { 50.0 }),
                    midi_mapping: None,
                    feedback_mapping: None,
                    color: Some(if is_black_key { "#000000".to_string() } else { "#FFFFFF".to_string() }),
                    label: String::new(),
                }
            );
        }

        controls
    }
}

impl MidiRouter {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            filters: Vec::new(),
            transformers: Vec::new(),
            enabled: true,
        }
    }

    pub fn add_route(&mut self, name: String, source: RouteSource, destination: RouteDestination) -> Uuid {
        let route_id = Uuid::new_v4();
        let route = MidiRoute {
            id: route_id,
            name,
            source,
            destination,
            enabled: true,
            channel_map: None,
            filters: Vec::new(),
            transformers: Vec::new(),
        };

        self.routes.push(route);
        route_id
    }
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            supports_mpe: false,
            supports_midi2: false,
            max_polyphony: Some(16),
            supported_ccs: (0..128).collect(),
            has_aftertouch: true,
            has_pitchbend: true,
            has_program_change: true,
            custom_sysex: false,
        }
    }
}

impl Default for LearnMode {
    fn default() -> Self {
        Self {
            active: false,
            target_mapping: None,
            timeout_ms: 10000, // 10 seconds
            auto_assign: true,
            learn_type: LearnType::SingleParameter,
        }
    }
}

impl Default for GlobalMidiSettings {
    fn default() -> Self {
        Self {
            clock_source: ClockSource::Internal,
            sync_mode: SyncMode::MidiClock,
            latency_compensation: true,
            auto_latency_detection: true,
            buffer_size: 256,
            sample_rate: 44100,
            midi_thru: false,
            panic_on_stop: true,
        }
    }
}

impl Default for MidiControlSystem {
    fn default() -> Self {
        Self::new()
    }
}