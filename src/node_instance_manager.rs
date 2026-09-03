/// NodeInstanceManager - Bridge between UI nodes and audio processing nodes
/// 
/// This module implements the critical translation layer between visual nodes in the UI
/// and actual DSP processing nodes in the audio engine. It manages the lifecycle of audio nodes,
/// their connections, and parameter synchronization.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}};
use crate::audio_engine::bridge::{AudioParamMessage, AudioEngineState, AudioEngineBridge};
// Removed unused NodeGraph import

/// Unique identifier for node instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeInstanceId(pub u64);

/// Node parameter with atomic storage for real-time access
#[derive(Debug)]
#[allow(dead_code)]
pub struct AtomicParameter {
    /// Current parameter value (atomically accessible from audio thread)
    value: Arc<Mutex<f32>>,
    /// Parameter name
    name: String,
    /// Minimum value
    min: f32,
    /// Maximum value
    max: f32,
    /// Default value
    default: f32,
    /// Whether parameter changes should be time-stamped for sample-accurate automation
    automatable: bool,
}

/// Time-stamped parameter change for sample-accurate automation
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TimedParameterChange {
    /// Target parameter ID
    param_id: String,
    /// New parameter value
    value: f32,
    /// Sample offset within the current buffer when change should occur
    sample_offset: usize,
    /// Absolute sample position in the timeline
    timeline_sample: u64,
}

/// Node instance representation in the manager
#[derive(Debug)]
#[allow(dead_code)]
pub struct ManagedNode {
    /// Unique instance ID
    id: NodeInstanceId,
    /// Node type identifier
    node_type: String,
    /// User-friendly display name
    display_name: String,
    /// Whether node is currently active in the audio graph
    active: Arc<AtomicBool>,
    /// Position in UI (x, y)
    position: (f32, f32),
    /// Map of parameter names to atomic parameters
    parameters: HashMap<String, AtomicParameter>,
    /// Connections to other nodes (output_port -> (target_node, input_port))
    connections: Vec<(String, NodeInstanceId, String)>,
    /// Audio node index in the DSP graph (if active)
    dsp_node_index: Option<usize>,
}

/// Manager for node instances, bridging UI and audio engine
pub struct NodeInstanceManager {
    /// Map of node IDs to managed nodes
    nodes: HashMap<NodeInstanceId, ManagedNode>,
    /// Next available node ID
    next_node_id: AtomicU64,
    /// Bridge to communicate with audio engine
    bridge: Arc<Mutex<AudioEngineBridge>>,
    /// Time-stamped parameter changes queue
    parameter_queue: Arc<Mutex<Vec<TimedParameterChange>>>,
    /// Current timeline position in samples
    current_sample: Arc<AtomicU64>,
}

impl NodeInstanceManager {
    /// Create a new NodeInstanceManager
    pub fn new(bridge: Arc<Mutex<AudioEngineBridge>>) -> Self {
        Self {
            nodes: HashMap::new(),
            next_node_id: AtomicU64::new(1),
            bridge,
            parameter_queue: Arc::new(Mutex::new(Vec::new())),
            current_sample: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new node instance
    pub fn create_node(&mut self, node_type: &str, display_name: &str, position: (f32, f32)) -> NodeInstanceId {
        let id = NodeInstanceId(self.next_node_id.fetch_add(1, Ordering::SeqCst));
        
        // Create the managed node
        let node = ManagedNode {
            id,
            node_type: node_type.to_string(),
            display_name: display_name.to_string(),
            active: Arc::new(AtomicBool::new(false)),
            position,
            parameters: HashMap::new(),
            connections: Vec::new(),
            dsp_node_index: None,
        };
        
        // Add to our collection
        self.nodes.insert(id, node);
        
        // Send message to audio engine to create the actual DSP node
        if let Ok(bridge) = self.bridge.lock() {
            let _ = bridge.send_param(AudioParamMessage::CreateNode(
                node_type.to_string(),
                id.0.to_string(),
            ));
        }
        
        id
    }
    
    /// Remove a node instance
    pub fn remove_node(&mut self, id: NodeInstanceId) -> bool {
        if let Some(_node) = self.nodes.remove(&id) {
            // Send message to audio engine to remove the DSP node via UI id
            if let Ok(bridge) = self.bridge.lock() {
                let _ = bridge.send_param(AudioParamMessage::DeleteNode(id.0.to_string()));
            }
            true
        } else {
            false
        }
    }
    
    /// Connect two nodes
    pub fn connect_nodes(&mut self, from_id: NodeInstanceId, output_port: &str, 
                         to_id: NodeInstanceId, input_port: &str) -> bool {
        // Verify both nodes exist
        if !self.nodes.contains_key(&from_id) || !self.nodes.contains_key(&to_id) {
            return false;
        }
        
        // Add connection to source node
        if let Some(node) = self.nodes.get_mut(&from_id) {
            node.connections.push((output_port.to_string(), to_id, input_port.to_string()));
            
            // Send message to audio engine to connect nodes by UI ids
            if let Ok(bridge) = self.bridge.lock() {
                let _ = bridge.send_param(AudioParamMessage::ConnectNodes(
                    from_id.0.to_string(),
                    to_id.0.to_string(),
                    output_port.to_string(),
                    input_port.to_string(),
                ));
            }
            
            true
        } else {
            false
        }
    }
    
    /// Set a node parameter value
    pub fn set_parameter(&mut self, id: NodeInstanceId, param_name: &str, value: f32) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            if let Some(param) = node.parameters.get(param_name) {
                // Update atomic parameter value for real-time access
                if let Ok(mut v) = param.value.lock() { *v = value; }
                
                // Send parameter update to audio engine using UI id
                if let Ok(bridge) = self.bridge.lock() {
                    let _ = bridge.send_param(AudioParamMessage::SetNodeParameter(
                        id.0.to_string(),
                        param_name.to_string(),
                        value,
                    ));
                }
                
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    /// Schedule a time-stamped parameter change for sample-accurate automation
    pub fn schedule_parameter_change(&self, id: NodeInstanceId, param_name: &str, 
                                    value: f32, sample_offset: usize) {
        if let Some(node) = self.nodes.get(&id) {
            if let Some(param) = node.parameters.get(param_name) {
                if param.automatable {
                    if let Some(index) = node.dsp_node_index {
                        let param_id = format!("{}:{}", index, param_name);
                        let timeline_sample = self.current_sample.load(Ordering::Acquire) + sample_offset as u64;
                        
                        let change = TimedParameterChange {
                            param_id,
                            value,
                            sample_offset,
                            timeline_sample,
                        };
                        
                        if let Ok(mut queue) = self.parameter_queue.lock() {
                            queue.push(change);
                        }
                    }
                }
            }
        }
    }
    
    /// Process any pending parameter changes for the current audio buffer
    pub fn process_parameter_changes(&self, buffer_size: usize) {
        let current_sample = self.current_sample.load(Ordering::Acquire);
        let next_sample = current_sample + buffer_size as u64;
        
        if let Ok(mut queue) = self.parameter_queue.lock() {
            // Collect changes that should occur in this buffer and remove them from the queue
            let mut changes: Vec<TimedParameterChange> = Vec::new();
            queue.retain(|change| {
                let in_buffer = change.timeline_sample >= current_sample && change.timeline_sample < next_sample;
                if in_buffer { changes.push(change.clone()); }
                !in_buffer
            });
            
            // Send them to the audio engine
            for change in changes {
                if let Ok(bridge) = self.bridge.lock() {
                    let _ = bridge.send_param(AudioParamMessage::SetParameter(
                        change.param_id,
                        change.value,
                    ));
                }
            }
        }
        
        // Update current sample position
        self.current_sample.store(next_sample, Ordering::Release);
    }
    
    /// Update node states from audio engine feedback
    pub fn update_from_engine_state(&mut self, state: &AudioEngineState) {
        // Update parameters from engine state
        for (param_id, value) in &state.current_params {
            if let Some((node_index, param_name)) = param_id.split_once(':') {
                if let Ok(index) = node_index.parse::<usize>() {
                    // Find the node with this DSP index
                    if let Some(node_id) = self.find_node_by_dsp_index(index) {
                        if let Some(node) = self.nodes.get_mut(&node_id) {
                            if let Some(param) = node.parameters.get_mut(param_name) {
                                // Update atomic parameter value
                                if let Ok(mut v) = param.value.lock() { *v = *value; }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Find a node by its DSP index
    fn find_node_by_dsp_index(&self, index: usize) -> Option<NodeInstanceId> {
        self.nodes.iter()
            .find(|(_, node)| node.dsp_node_index == Some(index))
            .map(|(id, _)| *id)
    }
    
    /// Get all node IDs
    pub fn get_node_ids(&self) -> Vec<NodeInstanceId> {
        self.nodes.keys().cloned().collect()
    }
    
    /// Get node information
    pub fn get_node_info(&self, id: NodeInstanceId) -> Option<(String, String, (f32, f32))> {
        self.nodes.get(&id).map(|node| {
            (node.node_type.clone(), node.display_name.clone(), node.position)
        })
    }
    
    /// Get node connections
    pub fn get_node_connections(&self, id: NodeInstanceId) -> Vec<(String, NodeInstanceId, String)> {
        self.nodes.get(&id)
            .map(|node| node.connections.clone())
            .unwrap_or_default()
    }
}

// Add atomic f32 implementation since it's not in std
#[repr(transparent)]
pub struct AtomicF32(std::sync::atomic::AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(std::sync::atomic::AtomicU32::new(value.to_bits()))
    }
    
    pub fn store(&self, value: f32, ordering: Ordering) {
        self.0.store(value.to_bits(), ordering);
    }
    
    pub fn load(&self, ordering: Ordering) -> f32 {
        f32::from_bits(self.0.load(ordering))
    }
}