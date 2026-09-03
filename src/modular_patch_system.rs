use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Modular patch system inspired by NoiseCraft and Pure Data
/// Provides visual programming interface for audio synthesis and processing

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModularPatch {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub nodes: HashMap<Uuid, ModularNode>,
    pub connections: Vec<Connection>,
    pub metadata: PatchMetadata,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchMetadata {
    pub author: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub category: PatchCategory,
    pub bpm: f32,
    pub key: String,
    pub scale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum PatchCategory {
    Synthesizer,
    Effect,
    Sequencer,
    Utility,
    Experimental,
    Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModularNode {
    pub id: Uuid,
    pub node_type: NodeType,
    pub position: (f32, f32),
    pub parameters: HashMap<String, NodeParameter>,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,
    pub bypass: bool,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum NodeType {
    // Oscillators (NoiseCraft inspired)
    SineOscillator,
    SawOscillator,
    SquareOscillator,
    TriangleOscillator,
    NoiseGenerator,
    WavetableOscillator,
    FMOscillator,
    AMOscillator,
    
    // Filters
    LowPassFilter,
    HighPassFilter,
    BandPassFilter,
    NotchFilter,
    CombFilter,
    StateVariableFilter,
    MoogFilter,
    
    // Effects
    Reverb,
    Delay,
    Chorus,
    Flanger,
    Phaser,
    Distortion,
    Compressor,
    Limiter,
    EQ,
    Bitcrusher,
    
    // Modulation
    LFO,
    Envelope,
    ADSR,
    Sequencer,
    RandomGenerator,
    SampleAndHold,
    
    // Utility
    Mixer,
    Splitter,
    VCA,
    VCF,
    Quantizer,
    Scaler,
    Offset,
    Inverter,
    
    // Pure Data compatibility
    PdBang,
    PdFloat,
    PdSymbol,
    PdList,
    PdMessage,
    PdMetro,
    PdCounter,
    PdSelect,
    
    // HexoDSP nodes
    HexoOsc,
    HexoFilter,
    HexoDelay,
    HexoReverb,
    HexoDistortion,
    
    // Custom user nodes
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeParameter {
    pub name: String,
    pub value: ParameterValue,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: String,
    pub automation: Option<AutomationCurve>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    String(String),
    Enum(String, Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationCurve {
    pub points: Vec<AutomationPoint>,
    pub curve_type: CurveType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationPoint {
    pub time: f32,
    pub value: f32,
    pub curve: f32, // Bezier curve control
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Linear,
    Exponential,
    Logarithmic,
    Bezier,
    Step,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    pub id: String,
    pub name: String,
    pub data_type: DataType,
    pub default_value: Option<f32>,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub id: String,
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Audio,
    Control,
    MIDI,
    Gate,
    Trigger,
    String,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub from_node: Uuid,
    pub from_output: String,
    pub to_node: Uuid,
    pub to_input: String,
    pub gain: f32,
    pub delay_samples: i32,
}

/// Modular patch manager for loading, saving, and organizing patches
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModularPatchManager {
    patches: HashMap<Uuid, ModularPatch>,
    current_patch: Option<Uuid>,
    patch_library: PatchLibrary,
    node_factory: NodeFactory,
}

#[derive(Debug, Clone)]
pub struct PatchLibrary {
    pub categories: HashMap<PatchCategory, Vec<Uuid>>,
    pub favorites: Vec<Uuid>,
    pub recent: Vec<Uuid>,
    pub user_patches: Vec<Uuid>,
    pub factory_patches: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct NodeFactory {
    node_templates: HashMap<NodeType, NodeTemplate>,
}

#[derive(Debug, Clone)]
pub struct NodeTemplate {
    pub node_type: NodeType,
    pub default_parameters: HashMap<String, NodeParameter>,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,
    pub description: String,
    pub category: String,
}

impl ModularPatchManager {
    pub fn new() -> Self {
        Self {
            patches: HashMap::new(),
            current_patch: None,
            patch_library: PatchLibrary::new(),
            node_factory: NodeFactory::new(),
        }
    }

    pub fn create_patch(&mut self, name: String) -> Uuid {
        let patch_id = Uuid::new_v4();
        let patch = ModularPatch {
            id: patch_id,
            name,
            description: String::new(),
            nodes: HashMap::new(),
            connections: Vec::new(),
            metadata: PatchMetadata {
                author: "User".to_string(),
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
                tags: Vec::new(),
                category: PatchCategory::Experimental,
                bpm: 120.0,
                key: "C".to_string(),
                scale: "Major".to_string(),
            },
            version: "1.0".to_string(),
        };

        self.patches.insert(patch_id, patch);
        self.current_patch = Some(patch_id);
        patch_id
    }

    pub fn add_node(&mut self, patch_id: Uuid, node_type: NodeType, position: (f32, f32)) -> Result<Uuid, String> {
        let patch = self.patches.get_mut(&patch_id).ok_or("Patch not found")?;
        let node_id = Uuid::new_v4();
        
        let template = self.node_factory.get_template(&node_type)
            .ok_or("Node template not found")?;

        let node = ModularNode {
            id: node_id,
            node_type: node_type.clone(),
            position,
            parameters: template.default_parameters.clone(),
            inputs: template.inputs.clone(),
            outputs: template.outputs.clone(),
            bypass: false,
            name: format!("{:?}", node_type),
            color: None,
        };

        patch.nodes.insert(node_id, node);
        patch.metadata.modified_at = chrono::Utc::now();
        Ok(node_id)
    }

    pub fn connect_nodes(&mut self, patch_id: Uuid, from_node: Uuid, from_output: String, 
                        to_node: Uuid, to_input: String) -> Result<Uuid, String> {
        let patch = self.patches.get_mut(&patch_id).ok_or("Patch not found")?;
        
        // Validate nodes exist
        if !patch.nodes.contains_key(&from_node) || !patch.nodes.contains_key(&to_node) {
            return Err("One or both nodes not found".to_string());
        }

        let connection_id = Uuid::new_v4();
        let connection = Connection {
            id: connection_id,
            from_node,
            from_output,
            to_node,
            to_input,
            gain: 1.0,
            delay_samples: 0,
        };

        patch.connections.push(connection);
        patch.metadata.modified_at = chrono::Utc::now();
        Ok(connection_id)
    }

    pub fn save_patch(&self, patch_id: Uuid, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let patch = self.patches.get(&patch_id).ok_or("Patch not found")?;
        let json = serde_json::to_string_pretty(patch)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_patch(&mut self, path: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let patch: ModularPatch = serde_json::from_str(&json)?;
        let patch_id = patch.id;
        self.patches.insert(patch_id, patch);
        Ok(patch_id)
    }

    pub fn get_patch(&self, patch_id: Uuid) -> Option<&ModularPatch> {
        self.patches.get(&patch_id)
    }

    pub fn get_current_patch(&self) -> Option<&ModularPatch> {
        self.current_patch.and_then(|id| self.patches.get(&id))
    }

    pub fn list_patches(&self) -> Vec<&ModularPatch> {
        self.patches.values().collect()
    }
}

impl PatchLibrary {
    pub fn new() -> Self {
        Self {
            categories: HashMap::new(),
            favorites: Vec::new(),
            recent: Vec::new(),
            user_patches: Vec::new(),
            factory_patches: Vec::new(),
        }
    }
}

impl NodeFactory {
    pub fn new() -> Self {
        let mut factory = Self {
            node_templates: HashMap::new(),
        };
        factory.initialize_templates();
        factory
    }

    fn initialize_templates(&mut self) {
        // Sine Oscillator
        self.add_template(NodeType::SineOscillator, NodeTemplate {
            node_type: NodeType::SineOscillator,
            default_parameters: {
                let mut params = HashMap::new();
                params.insert("frequency".to_string(), NodeParameter {
                    name: "Frequency".to_string(),
                    value: ParameterValue::Float(440.0),
                    min: 20.0,
                    max: 20000.0,
                    default: 440.0,
                    unit: "Hz".to_string(),
                    automation: None,
                });
                params.insert("amplitude".to_string(), NodeParameter {
                    name: "Amplitude".to_string(),
                    value: ParameterValue::Float(0.5),
                    min: 0.0,
                    max: 1.0,
                    default: 0.5,
                    unit: "".to_string(),
                    automation: None,
                });
                params
            },
            inputs: vec![
                NodeInput {
                    id: "freq_mod".to_string(),
                    name: "Freq Mod".to_string(),
                    data_type: DataType::Control,
                    default_value: Some(0.0),
                    connected: false,
                },
                NodeInput {
                    id: "amp_mod".to_string(),
                    name: "Amp Mod".to_string(),
                    data_type: DataType::Control,
                    default_value: Some(0.0),
                    connected: false,
                },
            ],
            outputs: vec![
                NodeOutput {
                    id: "audio_out".to_string(),
                    name: "Audio Out".to_string(),
                    data_type: DataType::Audio,
                },
            ],
            description: "Sine wave oscillator with frequency and amplitude modulation".to_string(),
            category: "Oscillators".to_string(),
        });

        // Low Pass Filter
        self.add_template(NodeType::LowPassFilter, NodeTemplate {
            node_type: NodeType::LowPassFilter,
            default_parameters: {
                let mut params = HashMap::new();
                params.insert("cutoff".to_string(), NodeParameter {
                    name: "Cutoff".to_string(),
                    value: ParameterValue::Float(1000.0),
                    min: 20.0,
                    max: 20000.0,
                    default: 1000.0,
                    unit: "Hz".to_string(),
                    automation: None,
                });
                params.insert("resonance".to_string(), NodeParameter {
                    name: "Resonance".to_string(),
                    value: ParameterValue::Float(0.7),
                    min: 0.1,
                    max: 10.0,
                    default: 0.7,
                    unit: "Q".to_string(),
                    automation: None,
                });
                params
            },
            inputs: vec![
                NodeInput {
                    id: "audio_in".to_string(),
                    name: "Audio In".to_string(),
                    data_type: DataType::Audio,
                    default_value: None,
                    connected: false,
                },
                NodeInput {
                    id: "cutoff_mod".to_string(),
                    name: "Cutoff Mod".to_string(),
                    data_type: DataType::Control,
                    default_value: Some(0.0),
                    connected: false,
                },
            ],
            outputs: vec![
                NodeOutput {
                    id: "audio_out".to_string(),
                    name: "Audio Out".to_string(),
                    data_type: DataType::Audio,
                },
            ],
            description: "Low pass filter with resonance control".to_string(),
            category: "Filters".to_string(),
        });

        // Add more templates for other node types...
    }

    fn add_template(&mut self, node_type: NodeType, template: NodeTemplate) {
        self.node_templates.insert(node_type, template);
    }

    pub fn get_template(&self, node_type: &NodeType) -> Option<&NodeTemplate> {
        self.node_templates.get(node_type)
    }

    pub fn get_all_templates(&self) -> &HashMap<NodeType, NodeTemplate> {
        &self.node_templates
    }
}

impl Default for ModularPatchManager {
    fn default() -> Self {
        Self::new()
    }
}