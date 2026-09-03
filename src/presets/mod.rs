/// Modular Preset System for HexoDSP DAW
/// 
/// This module provides a comprehensive preset system that enables the revolutionary
/// Three-View workflow by offering categorized, drag-and-drop modular content.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Categories for organizing presets in the Browser Panel
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PresetCategory {
    Generators,
    Filters,
    Effects,
    Cookbook,    // Complex multi-node patches
    Utilities,
    Custom,
}

impl PresetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            PresetCategory::Generators => "Generators",
            PresetCategory::Filters => "Filters", 
            PresetCategory::Effects => "Effects",
            PresetCategory::Cookbook => "Cookbook",
            PresetCategory::Utilities => "Utilities",
            PresetCategory::Custom => "Custom",
        }
    }
}

/// Node parameter configuration for presets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeParameter {
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

/// Individual node configuration within a preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_type: String,
    pub position: (f32, f32),
    pub parameters: Vec<NodeParameter>,
}

/// Connection between nodes in a preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub from_node: usize,
    pub from_output: String,
    pub to_node: usize,
    pub to_input: String,
}

/// Complete preset definition supporting single nodes or complex patches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePreset {
    pub name: String,
    pub description: String,
    pub category: PresetCategory,
    pub author: String,
    pub version: String,
    pub nodes: Vec<NodeConfig>,
    pub connections: Vec<NodeConnection>,
    pub tags: Vec<String>,
}

/// Preset manager for the Browser Panel integration
#[derive(Clone)]
pub struct PresetManager {
    presets: HashMap<PresetCategory, Vec<NodePreset>>,
}

impl PresetManager {
    pub fn new() -> Self {
        let mut manager = Self {
            presets: HashMap::new(),
        };
        manager.load_builtin_presets();
        manager
    }

    /// Load built-in presets for immediate usability
    fn load_builtin_presets(&mut self) {
        // Generator presets
        let generators = vec![
            NodePreset {
                name: "Sine Oscillator".to_string(),
                description: "Basic sine wave generator".to_string(),
                category: PresetCategory::Generators,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "generator.sine".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "frequency".to_string(),
                            value: 440.0,
                            min: 20.0,
                            max: 20000.0,
                            default: 440.0,
                        },
                        NodeParameter {
                            name: "amplitude".to_string(),
                            value: 0.5,
                            min: 0.0,
                            max: 1.0,
                            default: 0.5,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["oscillator".to_string(), "basic".to_string()],
            },
            NodePreset {
                name: "Saw Oscillator".to_string(),
                description: "Sawtooth wave generator".to_string(),
                category: PresetCategory::Generators,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "generator.saw".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "frequency".to_string(),
                            value: 440.0,
                            min: 20.0,
                            max: 20000.0,
                            default: 440.0,
                        },
                        NodeParameter {
                            name: "amplitude".to_string(),
                            value: 0.5,
                            min: 0.0,
                            max: 1.0,
                            default: 0.5,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["oscillator".to_string(), "saw".to_string()],
            },
            NodePreset {
                name: "Noise Generator".to_string(),
                description: "White noise generator".to_string(),
                category: PresetCategory::Generators,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "generator.noise".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "amplitude".to_string(),
                            value: 0.3,
                            min: 0.0,
                            max: 1.0,
                            default: 0.3,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["noise".to_string(), "generator".to_string()],
            },
        ];

        // Filter presets
        let filters = vec![
            NodePreset {
                name: "Low Pass Filter".to_string(),
                description: "Classic low-pass filter".to_string(),
                category: PresetCategory::Filters,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "filter.lpf".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "cutoff".to_string(),
                            value: 1000.0,
                            min: 20.0,
                            max: 20000.0,
                            default: 1000.0,
                        },
                        NodeParameter {
                            name: "resonance".to_string(),
                            value: 0.7,
                            min: 0.1,
                            max: 10.0,
                            default: 0.7,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["filter".to_string(), "lowpass".to_string()],
            },
            NodePreset {
                name: "High Pass Filter".to_string(),
                description: "Classic high-pass filter".to_string(),
                category: PresetCategory::Filters,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "filter.hpf".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "cutoff".to_string(),
                            value: 200.0,
                            min: 20.0,
                            max: 20000.0,
                            default: 200.0,
                        },
                        NodeParameter {
                            name: "resonance".to_string(),
                            value: 0.7,
                            min: 0.1,
                            max: 10.0,
                            default: 0.7,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["filter".to_string(), "highpass".to_string()],
            },
        ];

        // Effect presets
        let effects = vec![
            NodePreset {
                name: "Delay".to_string(),
                description: "Digital delay effect".to_string(),
                category: PresetCategory::Effects,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "effect.delay".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "time".to_string(),
                            value: 0.25,
                            min: 0.001,
                            max: 2.0,
                            default: 0.25,
                        },
                        NodeParameter {
                            name: "feedback".to_string(),
                            value: 0.3,
                            min: 0.0,
                            max: 0.95,
                            default: 0.3,
                        },
                        NodeParameter {
                            name: "mix".to_string(),
                            value: 0.3,
                            min: 0.0,
                            max: 1.0,
                            default: 0.3,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["delay".to_string(), "effect".to_string()],
            },
            NodePreset {
                name: "Reverb".to_string(),
                description: "Algorithmic reverb".to_string(),
                category: PresetCategory::Effects,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "effect.reverb".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "room_size".to_string(),
                            value: 0.5,
                            min: 0.0,
                            max: 1.0,
                            default: 0.5,
                        },
                        NodeParameter {
                            name: "damping".to_string(),
                            value: 0.5,
                            min: 0.0,
                            max: 1.0,
                            default: 0.5,
                        },
                        NodeParameter {
                            name: "mix".to_string(),
                            value: 0.3,
                            min: 0.0,
                            max: 1.0,
                            default: 0.3,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["reverb".to_string(), "effect".to_string()],
            },
        ];

        // Cookbook presets (complex multi-node patches)
        let cookbook = vec![
            NodePreset {
                name: "FM Synthesizer".to_string(),
                description: "Two-operator FM synthesis patch".to_string(),
                category: PresetCategory::Cookbook,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![
                    NodeConfig {
                        node_type: "generator.sine".to_string(),
                        position: (0.0, 0.0),
                        parameters: vec![
                            NodeParameter {
                                name: "frequency".to_string(),
                                value: 440.0,
                                min: 20.0,
                                max: 20000.0,
                                default: 440.0,
                            },
                        ],
                    },
                    NodeConfig {
                        node_type: "generator.sine".to_string(),
                        position: (150.0, -50.0),
                        parameters: vec![
                            NodeParameter {
                                name: "frequency".to_string(),
                                value: 880.0,
                                min: 20.0,
                                max: 20000.0,
                                default: 880.0,
                            },
                        ],
                    },
                    NodeConfig {
                        node_type: "utility.mixer".to_string(),
                        position: (300.0, 0.0),
                        parameters: vec![],
                    },
                ],
                connections: vec![
                    NodeConnection {
                        from_node: 0,
                        from_output: "output".to_string(),
                        to_node: 1,
                        to_input: "frequency_mod".to_string(),
                    },
                    NodeConnection {
                        from_node: 1,
                        from_output: "output".to_string(),
                        to_node: 2,
                        to_input: "input_1".to_string(),
                    },
                ],
                tags: vec!["fm".to_string(), "synthesizer".to_string(), "complex".to_string()],
            },
        ];

        // Utility presets
        let utilities = vec![
            NodePreset {
                name: "Mixer 4-Channel".to_string(),
                description: "Four-channel audio mixer".to_string(),
                category: PresetCategory::Utilities,
                author: "HexoDSP".to_string(),
                version: "1.0".to_string(),
                nodes: vec![NodeConfig {
                    node_type: "utility.mixer".to_string(),
                    position: (0.0, 0.0),
                    parameters: vec![
                        NodeParameter {
                            name: "gain_1".to_string(),
                            value: 0.8,
                            min: 0.0,
                            max: 2.0,
                            default: 0.8,
                        },
                        NodeParameter {
                            name: "gain_2".to_string(),
                            value: 0.8,
                            min: 0.0,
                            max: 2.0,
                            default: 0.8,
                        },
                        NodeParameter {
                            name: "gain_3".to_string(),
                            value: 0.8,
                            min: 0.0,
                            max: 2.0,
                            default: 0.8,
                        },
                        NodeParameter {
                            name: "gain_4".to_string(),
                            value: 0.8,
                            min: 0.0,
                            max: 2.0,
                            default: 0.8,
                        },
                    ],
                }],
                connections: vec![],
                tags: vec!["mixer".to_string(), "utility".to_string()],
            },
        ];

        self.presets.insert(PresetCategory::Generators, generators);
        self.presets.insert(PresetCategory::Filters, filters);
        self.presets.insert(PresetCategory::Effects, effects);
        self.presets.insert(PresetCategory::Cookbook, cookbook);
        self.presets.insert(PresetCategory::Utilities, utilities);
    }

    /// Get all presets for a specific category
    pub fn get_presets_by_category(&self, category: &PresetCategory) -> Option<&Vec<NodePreset>> {
        self.presets.get(category)
    }

    /// Get all available categories
    pub fn get_categories(&self) -> Vec<&PresetCategory> {
        self.presets.keys().collect()
    }

    /// Search presets by name or tags
    pub fn search_presets(&self, query: &str) -> Vec<&NodePreset> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for presets in self.presets.values() {
            for preset in presets {
                if preset.name.to_lowercase().contains(&query_lower) ||
                   preset.description.to_lowercase().contains(&query_lower) ||
                   preset.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower)) {
                    results.push(preset);
                }
            }
        }

        results
    }

    /// Add a custom preset
    pub fn add_preset(&mut self, preset: NodePreset) {
        let category = preset.category.clone();
        self.presets.entry(category).or_insert_with(Vec::new).push(preset);
    }

    /// Get a preset by name
    pub fn get_preset_by_name(&self, name: &str) -> Option<&NodePreset> {
        for presets in self.presets.values() {
            for preset in presets {
                if preset.name == name {
                    return Some(preset);
                }
            }
        }
        None
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}