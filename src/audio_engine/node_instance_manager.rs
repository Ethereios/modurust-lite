//! Node Instance Manager
//! 
//! Manages the bidirectional mapping between UI nodes and audio engine nodes.
//! Handles node creation, deletion, connection, and parameter synchronization.

use std::collections::HashMap;
use super::node_graph::NodeGraph;
use super::bridge::{AudioEngineBridge, AudioParamMessage};

/// Maps UI node IDs to audio engine node IDs
pub struct NodeInstanceManager {
    /// UI node ID -> Audio node ID
    ui_to_audio: HashMap<String, usize>,
    
    /// Audio node ID -> UI node ID
    audio_to_ui: HashMap<usize, String>,
    
    /// Reference to the audio node graph
    node_graph: NodeGraph,
    
    /// Communication bridge
    _bridge: AudioEngineBridge,
    
    /// Counter for generating unique audio node IDs
    _next_audio_id: usize,
}

impl NodeInstanceManager {
    /// Create a new node instance manager
    pub fn new(node_graph: NodeGraph, bridge: AudioEngineBridge) -> Self {
        Self {
            ui_to_audio: HashMap::new(),
            audio_to_ui: HashMap::new(),
            node_graph,
            _bridge: bridge,
            _next_audio_id: 0,
        }
    }
    
    /// Create a new audio node from UI request
    pub fn create_node(&mut self, ui_node_id: String, node_type: &str) -> Result<usize, String> {
        // Check if UI node already exists
        if self.ui_to_audio.contains_key(&ui_node_id) {
            return Err(format!("UI node '{}' already exists", ui_node_id));
        }
        
        // Create audio node based on type
        let audio_node_id = match node_type {
            "generator.sine" | "generator.saw" | "generator.square" | "generator.triangle" => {
                self.node_graph.add_oscillator()
            }
            "filter.lpf" | "filter.hpf" | "filter.bpf" | "filter.notch" | "filter.svf" => {
                self.node_graph.add_filter()
            }
            "effect.reverb" => {
                self.node_graph.add_reverb()
            }
            "effect.delay" => {
                self.node_graph.add_delay()
            }
            "utility.mixer" => {
                self.node_graph.add_mixer(4) // Default 4 inputs
            }
            "utility.output" => {
                self.node_graph.add_output()
            }
            "utility.input" => {
                self.node_graph.add_input()
            }
            _ => {
                // Unknown type - create a generic oscillator as fallback
                println!("⚠️ Unknown node type '{}', creating oscillator", node_type);
                self.node_graph.add_oscillator()
            }
        };
        
        // Store bidirectional mapping
        self.ui_to_audio.insert(ui_node_id.clone(), audio_node_id);
        self.audio_to_ui.insert(audio_node_id, ui_node_id.clone());
        
        println!("✅ Created audio node {} for UI node '{}'", audio_node_id, ui_node_id);
        
        Ok(audio_node_id)
    }
    
    /// Delete a node
    pub fn delete_node(&mut self, ui_node_id: &str) -> Result<(), String> {
        // Get audio node ID
        let audio_node_id = self.ui_to_audio.get(ui_node_id)
            .ok_or_else(|| format!("UI node '{}' not found", ui_node_id))?;
        
        // Remove from node graph
        self.node_graph.remove_node(*audio_node_id);
        
        // Remove from mappings
        self.audio_to_ui.remove(audio_node_id);
        self.ui_to_audio.remove(ui_node_id);
        
        println!("🗑️ Deleted node '{}'", ui_node_id);
        
        Ok(())
    }
    
    /// Connect two nodes
    pub fn connect_nodes(
        &mut self,
        from_ui_id: &str,
        to_ui_id: &str,
        from_port: &str,
        to_port: &str,
    ) -> Result<(), String> {
        // Get audio node IDs
        let from_audio_id = self.ui_to_audio.get(from_ui_id)
            .ok_or_else(|| format!("Source UI node '{}' not found", from_ui_id))?;
        let to_audio_id = self.ui_to_audio.get(to_ui_id)
            .ok_or_else(|| format!("Target UI node '{}' not found", to_ui_id))?;
        
        // Create connection in audio graph
        self.node_graph.connect(*from_audio_id, *to_audio_id, from_port, to_port)?;
        
        println!("🔗 Connected {} -> {}", from_ui_id, to_ui_id);
        
        Ok(())
    }
    
    /// Disconnect two nodes
    pub fn disconnect_nodes(&mut self, from_ui_id: &str, to_ui_id: &str) -> Result<(), String> {
        // Get audio node IDs
        let from_audio_id = self.ui_to_audio.get(from_ui_id)
            .ok_or_else(|| format!("Source UI node '{}' not found", from_ui_id))?;
        let to_audio_id = self.ui_to_audio.get(to_ui_id)
            .ok_or_else(|| format!("Target UI node '{}' not found", to_ui_id))?;
        
        // Disconnect in audio graph
        self.node_graph.disconnect(*from_audio_id, *to_audio_id);
        
        println!("🔌 Disconnected {} -/- {}", from_ui_id, to_ui_id);
        
        Ok(())
    }
    
    /// Set a node parameter
    pub fn set_node_parameter(
        &mut self,
        ui_node_id: &str,
        param_name: &str,
        value: f32,
    ) -> Result<(), String> {
        // Get audio node ID
        let _audio_node_id = self.ui_to_audio.get(ui_node_id)
            .ok_or_else(|| format!("UI node '{}' not found", ui_node_id))?;
        
        // TODO: Set parameter on audio node
        // For now, just log it
        println!("🎛️ Set {}:{} = {:.3}", ui_node_id, param_name, value);
        
        Ok(())
    }
    
    /// Get audio node ID from UI node ID
    pub fn get_audio_id(&self, ui_node_id: &str) -> Option<usize> {
        self.ui_to_audio.get(ui_node_id).copied()
    }
    
    /// Get UI node ID from audio node ID
    pub fn get_ui_id(&self, audio_node_id: usize) -> Option<&String> {
        self.audio_to_ui.get(&audio_node_id)
    }
    
    /// Process messages from UI
    pub fn process_message(&mut self, message: AudioParamMessage) -> Result<(), String> {
        match message {
            AudioParamMessage::CreateNode(node_type, ui_node_id) => {
                self.create_node(ui_node_id, &node_type)?;
            }
            AudioParamMessage::DeleteNode(ui_node_id) => {
                self.delete_node(&ui_node_id)?;
            }
            AudioParamMessage::ConnectNodes(from_ui_id, to_ui_id, from_port, to_port) => {
                self.connect_nodes(&from_ui_id, &to_ui_id, &from_port, &to_port)?;
            }
            AudioParamMessage::DisconnectNodes(from_ui_id, to_ui_id) => {
                self.disconnect_nodes(&from_ui_id, &to_ui_id)?;
            }
            AudioParamMessage::SetNodeParameter(ui_node_id, param_name, value) => {
                self.set_node_parameter(&ui_node_id, &param_name, value)?;
            }
            _ => {
                // Other message types handled elsewhere
            }
        }
        
        Ok(())
    }
    
    /// Get the underlying node graph (for audio processing)
    pub fn node_graph(&self) -> &NodeGraph {
        &self.node_graph
    }
    
    /// Get mutable reference to node graph
    pub fn node_graph_mut(&mut self) -> &mut NodeGraph {
        &mut self.node_graph
    }
    
    /// Get all active UI node IDs
    pub fn active_ui_nodes(&self) -> Vec<String> {
        self.ui_to_audio.keys().cloned().collect()
    }
    
    /// Get mapping for feedback to UI
    pub fn get_ui_to_audio_map(&self) -> HashMap<String, usize> {
        self.ui_to_audio.clone()
    }
}
