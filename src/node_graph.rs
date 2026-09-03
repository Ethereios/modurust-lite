//! Node Graph System for HexoDSP DAW
//!
//! Simplified node graph implementation inspired by NUWE's architecture,
//! providing a framework for composing modular synthesis and processing pipelines.

use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique identifier for nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NeuroNodeId(pub u64);

impl NeuroNodeId {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self(timestamp)
    }
}

/// Unique identifier for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NeuroConnectionId(pub u64);

impl NeuroConnectionId {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self(timestamp)
    }
}

/// Data types that can flow through the node graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NeuroDataType {
    Float,
    Integer,
    String,
    Array,
    Audio,
    Midi,
    Control,
    Waveform,
}

/// Node connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroNodeConnection {
    pub id: NeuroConnectionId,
    pub from_node: NeuroNodeId,
    pub from_port: String,
    pub to_node: NeuroNodeId,
    pub to_port: String,
    pub data_type: NeuroDataType,
}

/// Node port definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroNodePort {
    pub name: String,
    pub data_type: NeuroDataType,
    pub required: bool,
}

/// Node definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuroNodeDefinition {
    pub id: NeuroNodeId,
    pub name: String,
    pub node_type: String,
    pub inputs: Vec<NeuroNodePort>,
    pub outputs: Vec<NeuroNodePort>,
}

/// Node graph
pub struct NeuroNodeGraph {
    nodes: HashMap<NeuroNodeId, NeuroNodeDefinition>,
    connections: HashMap<NeuroConnectionId, NeuroNodeConnection>,
    node_to_index: HashMap<NeuroNodeId, usize>,
    evaluation_order: Vec<NeuroNodeId>,
    dirty_nodes: HashSet<NeuroNodeId>,
}

impl Default for NeuroNodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl NeuroNodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: HashMap::new(),
            node_to_index: HashMap::new(),
            evaluation_order: Vec::new(),
            dirty_nodes: HashSet::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, definition: NeuroNodeDefinition) -> Result<(), String> {
        if self.nodes.contains_key(&definition.id) {
            return Err(format!("Node {} already exists", definition.id.0));
        }

        let index = self.nodes.len();
        self.node_to_index.insert(definition.id, index);
        self.nodes.insert(definition.id, definition);
        self.update_evaluation_order();
        Ok(())
    }

    /// Remove a node from the graph
    pub fn remove_node(&mut self, node_id: NeuroNodeId) -> Result<(), String> {
        if !self.nodes.contains_key(&node_id) {
            return Err(format!("Node {} not found", node_id.0));
        }

        // Remove all connections involving this node
        let connections_to_remove: Vec<_> = self.connections
            .iter()
            .filter(|(_, conn)| conn.from_node == node_id || conn.to_node == node_id)
            .map(|(id, _)| *id)
            .collect();

        for conn_id in connections_to_remove {
            self.connections.remove(&conn_id);
        }

        self.nodes.remove(&node_id);
        self.node_to_index.remove(&node_id);
        self.dirty_nodes.remove(&node_id);
        self.update_evaluation_order();
        Ok(())
    }

    /// Add a connection between nodes
    pub fn add_connection(&mut self, connection: NeuroNodeConnection) -> Result<(), String> {
        // Validate nodes exist
        if !self.nodes.contains_key(&connection.from_node) {
            return Err(format!("Source node {} not found", connection.from_node.0));
        }
        if !self.nodes.contains_key(&connection.to_node) {
            return Err(format!("Target node {} not found", connection.to_node.0));
        }

        // Validate ports exist and types match
        let from_node = &self.nodes[&connection.from_node];
        let to_node = &self.nodes[&connection.to_node];

        let from_port = from_node.outputs.iter().find(|p| p.name == connection.from_port)
            .ok_or_else(|| format!("Output port '{}' not found on node {}", connection.from_port, from_node.name))?;

        let to_port = to_node.inputs.iter().find(|p| p.name == connection.to_port)
            .ok_or_else(|| format!("Input port '{}' not found on node {}", connection.to_port, to_node.name))?;

        if from_port.data_type != to_port.data_type {
            return Err(format!("Port type mismatch: {:?} != {:?}", from_port.data_type, to_port.data_type));
        }

        // Check for cycles
        if self.would_create_cycle(connection.from_node, connection.to_node) {
            return Err("Connection would create a cycle".to_string());
        }

        let to_node = connection.to_node;
        self.connections.insert(connection.id, connection);
        self.mark_dirty(to_node);
        self.update_evaluation_order();
        Ok(())
    }

    /// Remove a connection
    pub fn remove_connection(&mut self, connection_id: NeuroConnectionId) -> Result<(), String> {
        let connection = self.connections.remove(&connection_id)
            .ok_or_else(|| format!("Connection {} not found", connection_id.0))?;

        self.mark_dirty(connection.to_node);
        self.update_evaluation_order();
        Ok(())
    }

    /// Mark a node as dirty
    pub fn mark_dirty(&mut self, node_id: NeuroNodeId) {
        self.dirty_nodes.insert(node_id);
        self.propagate_dirty_downstream(node_id);
    }

    /// Propagate dirty flag to downstream nodes
    fn propagate_dirty_downstream(&mut self, node_id: NeuroNodeId) {
        let downstream_nodes: Vec<_> = self.connections
            .values()
            .filter(|conn| conn.from_node == node_id)
            .map(|conn| conn.to_node)
            .collect();

        for downstream_node in downstream_nodes {
            if self.dirty_nodes.insert(downstream_node) {
                self.propagate_dirty_downstream(downstream_node);
            }
        }
    }

    /// Check if adding a connection would create a cycle
    fn would_create_cycle(&self, from_node: NeuroNodeId, to_node: NeuroNodeId) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![to_node];

        while let Some(current) = stack.pop() {
            if current == from_node {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Find all nodes that current connects to
            for connection in self.connections.values() {
                if connection.from_node == current {
                    stack.push(connection.to_node);
                }
            }
        }

        false
    }

    /// Update topological evaluation order
    fn update_evaluation_order(&mut self) {
        self.evaluation_order.clear();

        let mut in_degree: HashMap<NeuroNodeId, usize> = HashMap::new();
        let mut queue = VecDeque::new();

        // Calculate in-degrees
        for &node_id in self.nodes.keys() {
            let degree = self.connections.values()
                .filter(|conn| conn.to_node == node_id)
                .count();
            in_degree.insert(node_id, degree);
            if degree == 0 {
                queue.push_back(node_id);
            }
        }

        // Kahn's algorithm
        while let Some(current) = queue.pop_front() {
            self.evaluation_order.push(current);

            // Find all nodes that current connects to
            let outgoing_connections: Vec<_> = self.connections.values()
                .filter(|conn| conn.from_node == current)
                .collect();

            for connection in outgoing_connections {
                if let Some(degree) = in_degree.get_mut(&connection.to_node) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(connection.to_node);
                    }
                }
            }
        }
    }

    /// Get evaluation order
    pub fn evaluation_order(&self) -> &[NeuroNodeId] {
        &self.evaluation_order
    }

    /// Get dirty nodes
    pub fn dirty_nodes(&self) -> &HashSet<NeuroNodeId> {
        &self.dirty_nodes
    }

    /// Clear dirty flag for a node
    pub fn clear_dirty(&mut self, node_id: NeuroNodeId) {
        self.dirty_nodes.remove(&node_id);
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<NeuroNodeId, NeuroNodeDefinition> {
        &self.nodes
    }

    /// Get all connections
    pub fn connections(&self) -> &HashMap<NeuroConnectionId, NeuroNodeConnection> {
        &self.connections
    }

    /// Get connections for a specific node
    pub fn get_connections_for_node(&self, node_id: NeuroNodeId) -> Vec<&NeuroNodeConnection> {
        self.connections.values()
            .filter(|conn| conn.from_node == node_id || conn.to_node == node_id)
            .collect()
    }

    /// Serialize the graph to JSON
    pub fn to_json(&self) -> Result<String, String> {
        let graph_data = NeuroGraphData {
            nodes: self.nodes.values().cloned().collect(),
            connections: self.connections.values().cloned().collect(),
        };
        serde_json::to_string(&graph_data)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Deserialize the graph from JSON
    pub fn from_json(json: &str) -> Result<Self, String> {
        let graph_data: NeuroGraphData = serde_json::from_str(json)
            .map_err(|e| format!("Deserialization error: {}", e))?;

        let mut graph = Self::new();

        for node in graph_data.nodes {
            graph.add_node(node)?;
        }

        for connection in graph_data.connections {
            graph.add_connection(connection)?;
        }

        Ok(graph)
    }
}

/// Serializable graph data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NeuroGraphData {
    nodes: Vec<NeuroNodeDefinition>,
    connections: Vec<NeuroNodeConnection>,
}

/// Node processor trait for executing nodes
#[async_trait::async_trait]
pub trait NeuroNodeProcessor {
    async fn process(&mut self, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, String>;
}

/// Node execution engine
pub struct NeuroNodeExecutionEngine {
    processors: HashMap<NeuroNodeId, Box<dyn NeuroNodeProcessor>>,
}

impl NeuroNodeExecutionEngine {
    pub fn new() -> Self {
        Self {
            processors: HashMap::new(),
        }
    }

    pub fn register_processor(&mut self, node_id: NeuroNodeId, processor: Box<dyn NeuroNodeProcessor>) {
        self.processors.insert(node_id, processor);
    }

    pub async fn execute_graph(&mut self, graph: &mut NeuroNodeGraph) -> Result<HashMap<NeuroNodeId, HashMap<String, Value>>, String> {
        let mut results = HashMap::new();
        let mut node_outputs: HashMap<NeuroNodeId, HashMap<String, Value>> = HashMap::new();

        let eval_order = graph.evaluation_order().to_vec();
        for &node_id in &eval_order {
            if !graph.dirty_nodes().contains(&node_id) {
                continue;
            }

            let processor = self.processors.get_mut(&node_id)
                .ok_or_else(|| format!("No processor registered for node {}", node_id.0))?;

            // Collect inputs from upstream nodes
            let mut inputs = HashMap::new();
            for connection in graph.get_connections_for_node(node_id) {
                if connection.to_node == node_id {
                    // This is an input connection
                    if let Some(upstream_outputs) = node_outputs.get(&connection.from_node) {
                        if let Some(output_value) = upstream_outputs.get(&connection.from_port) {
                            inputs.insert(connection.to_port.clone(), output_value.clone());
                        }
                    }
                }
            }

            // Execute the node
            let outputs = processor.process(inputs).await?;
            node_outputs.insert(node_id, outputs.clone());
            results.insert(node_id, outputs);

            graph.clear_dirty(node_id);
        }

        Ok(results)
    }
}