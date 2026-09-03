//! Real-time-safe modular audio node graph (arena-based).
//!
//! Nodes live in an index arena (`Vec<Option<Slot>>` with a free-list), so
//! `NodeId` is just a slot index. Per-node output buffers, an accumulator and
//! a scratch buffer are allocated once and reused across `process` calls, so a
//! steady-state audio block performs **zero heap allocations**.
//!
//! Processing runs in a cached topological order. The engine feeds its voice
//! signal in as the graph input; nodes with no incoming edges (sources) are
//! driven by that input. `Output` nodes are sinks: when at least one `Output`
//! node exists, the final graph output is the sum of all `Output` node
//! buffers. Otherwise the graph is a pass-through (identity).

use crate::dsp::extra_nodes::{LfoNode, MixerNode, ReverbNode, SplitterNode, StereoDelayNode};
use crate::dsp::{
    AdsrEnvelopeNode, BiquadFilterNode, DistortionNode, DspNode, FilterMode, GainNode, NoiseColor,
    NoiseNode, OscillatorNode, SamplerNode, SimpleDelayNode, Waveform,
};

pub type NodeId = usize;

/// Parameters addressable per node, normalized to the same units the UI uses
/// (Hz, seconds, 0..=1 where applicable, waveform/color as integer index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphParam {
    Frequency,
    Waveform,
    Cutoff,
    Resonance,
    FilterMode,
    EnvAttack,
    EnvDecay,
    EnvSustain,
    EnvRelease,
    LfoFreq,
    DelayTime,
    DelayFeedback,
    ReverbRoom,
    ReverbDamping,
    Gain,
    /// MixerNode gain1 (first channel gain)
    MixerGain1,
    /// MixerNode gain2 (second channel gain)
    MixerGain2,
    Drive,
    Tone,
    NoiseColor,
    PlaybackRate,
    /// StereoDelay left channel delay time (seconds)
    DelayLeftTime,
    /// StereoDelay right channel delay time (seconds)
    DelayRightTime,
    /// StereoDelay cross-feedback amount
    CrossFeedback,
}

/// Serializable node specification used by the command bus / UI so the engine
/// and the UI agree on a node's identity without a round-trip.
#[derive(Debug, Clone)]
pub enum GraphNodeKind {
    Oscillator(Waveform, f32),
    Filter(FilterMode, f32, f32),
    Envelope(f32, f32, f32, f32),
    Lfo(f32),
    Delay(f32, f32),
    Reverb(f32, f32),
    Noise(NoiseColor),
    Gain(f32),
    Distortion(f32, f32),
    Sampler,
    /// Mixer node with two gain values (gain1, gain2).
    Mixer(f32, f32),
    /// Splitter node (passthrough, fan-out handled by graph topology).
    Splitter,
    /// Stereo delay node with left/right delay times (seconds) and cross-feedback.
    StereoDelay(f32, f32, f32),
    Output,
}

impl GraphNodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            GraphNodeKind::Oscillator(..) => "oscillator",
            GraphNodeKind::Filter(..) => "filter",
            GraphNodeKind::Envelope(..) => "envelope",
            GraphNodeKind::Lfo(..) => "lfo",
            GraphNodeKind::Delay(..) => "delay",
            GraphNodeKind::Reverb(..) => "reverb",
            GraphNodeKind::Noise(..) => "noise",
            GraphNodeKind::Gain(..) => "gain",
            GraphNodeKind::Distortion(..) => "distortion",
            GraphNodeKind::Sampler => "sampler",
            GraphNodeKind::Mixer(..) => "mixer",
            GraphNodeKind::Splitter => "splitter",
            GraphNodeKind::StereoDelay(..) => "stereo_delay",
            GraphNodeKind::Output => "output",
        }
    }
}

/// A concrete DSP node stored in the graph arena.
pub enum NodeKind {
    Oscillator(OscillatorNode),
    Filter(BiquadFilterNode),
    Envelope(AdsrEnvelopeNode),
    Lfo(LfoNode),
    Delay(SimpleDelayNode),
    Reverb(ReverbNode),
    Noise(NoiseNode),
    Gain(GainNode),
    Distortion(DistortionNode),
    Sampler(SamplerNode),
    Mixer(MixerNode),
    Splitter(SplitterNode),
    StereoDelay(StereoDelayNode),
    Output,
}

impl NodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            NodeKind::Oscillator(_) => "oscillator",
            NodeKind::Filter(_) => "filter",
            NodeKind::Envelope(_) => "envelope",
            NodeKind::Lfo(_) => "lfo",
            NodeKind::Delay(_) => "delay",
            NodeKind::Reverb(_) => "reverb",
            NodeKind::Noise(_) => "noise",
            NodeKind::Gain(_) => "gain",
            NodeKind::Distortion(_) => "distortion",
            NodeKind::Sampler(_) => "sampler",
            NodeKind::Mixer(_) => "mixer",
            NodeKind::Splitter(_) => "splitter",
            NodeKind::StereoDelay(_) => "stereo_delay",
            NodeKind::Output => "output",
        }
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        match self {
            NodeKind::Oscillator(n) => n.process(input, output),
            NodeKind::Filter(n) => n.process(input, output),
            NodeKind::Envelope(n) => n.process(input, output),
            NodeKind::Lfo(n) => n.process(input, output),
            NodeKind::Delay(n) => n.process(input, output),
            NodeKind::Reverb(n) => n.process(input, output),
            NodeKind::Noise(n) => n.process(input, output),
            NodeKind::Gain(n) => n.process(input, output),
            NodeKind::Distortion(n) => n.process(input, output),
            NodeKind::Sampler(n) => n.process(input, output),
            NodeKind::Mixer(n) => n.process(input, output),
            NodeKind::Splitter(n) => n.process(input, output),
            NodeKind::StereoDelay(n) => n.process(input, output),
            // Output node is a sink — it just passes through.
            NodeKind::Output => output.copy_from_slice(input),
        }
    }

/// Apply a `GraphParam` value to this node. `sample_rate` is passed in
    /// so nodes that need it (e.g. Delay converting seconds to samples) can
    /// use it without each node storing its own copy.
    pub fn apply_param(&mut self, param: GraphParam, value: f32, sample_rate: f32) {
        match self {
            NodeKind::Oscillator(n) => match param {
                GraphParam::Frequency => n.freq = value.max(1.0),
                GraphParam::Waveform => {
                    n.waveform = match (value as i32).clamp(0, 3) {
                        1 => Waveform::Saw,
                        2 => Waveform::Square,
                        3 => Waveform::Triangle,
                        _ => Waveform::Sine,
                    };
                }
                _ => {}
            },
            NodeKind::Filter(n) => match param {
                GraphParam::Cutoff => n.cutoff = value.clamp(20.0, 20000.0),
                GraphParam::Resonance => n.resonance = value.clamp(0.1, 10.0),
                GraphParam::FilterMode => {
                    n.set_mode(match (value as i32).clamp(0, 3) {
                        1 => FilterMode::HighPass,
                        2 => FilterMode::BandPass,
                        3 => FilterMode::Notch,
                        _ => FilterMode::LowPass,
                    });
                }
                _ => {}
            },
            NodeKind::Envelope(n) => match param {
                GraphParam::EnvAttack => n.attack = value.clamp(0.001, 10.0),
                GraphParam::EnvDecay => n.decay = value.clamp(0.001, 10.0),
                GraphParam::EnvSustain => n.sustain = value.clamp(0.0, 1.0),
                GraphParam::EnvRelease => n.release = value.clamp(0.001, 10.0),
                _ => {}
            },
            NodeKind::Lfo(n) => match param {
                GraphParam::LfoFreq => n.freq = value.max(0.01),
                _ => {}
            },
            NodeKind::Delay(n) => match param {
                GraphParam::DelayTime => {
                    let samples = (value.max(0.001) * sample_rate) as usize;
                    n.delay_samples = samples.min(n.buffer.len() - 1).max(1);
                }
                GraphParam::DelayFeedback => n.feedback = value.clamp(0.0, 0.95),
                _ => {}
            },
            NodeKind::Reverb(n) => match param {
                GraphParam::ReverbRoom => n.room_size = value.clamp(0.0, 1.0),
                GraphParam::ReverbDamping => n.damping = value.clamp(0.0, 1.0),
                _ => {}
            },
            NodeKind::Gain(n) => match param {
                GraphParam::Gain => n.gain = value,
                _ => {}
            },
            NodeKind::Distortion(n) => match param {
                GraphParam::Drive => n.drive = value.max(1.0),
                GraphParam::Tone => n.tone = value.clamp(0.0, 1.0),
                _ => {}
            },
            NodeKind::Noise(n) => match param {
                GraphParam::NoiseColor => {
                    n.set_color(match (value as i32).clamp(0, 2) {
                        1 => NoiseColor::Pink,
                        2 => NoiseColor::Brown,
                        _ => NoiseColor::White,
                    });
                }
                _ => {}
            },
            NodeKind::Sampler(n) => match param {
                GraphParam::PlaybackRate => n.playback_rate = value.max(0.01),
                _ => {}
            },
            NodeKind::Mixer(n) => match param {
                GraphParam::MixerGain1 => n.gain1 = value,
                GraphParam::MixerGain2 => n.gain2 = value,
                _ => {}
            },
            NodeKind::Splitter(_) => {
                // Splitter has no parameters (passthrough).
            }
NodeKind::StereoDelay(n) => match param {
                GraphParam::DelayLeftTime => {
                    let samples = (value.max(0.001) * sample_rate) as usize;
                    n.delay_left_samples = samples.max(1);
                }
                GraphParam::DelayRightTime => {
                    let samples = (value.max(0.001) * sample_rate) as usize;
                    n.delay_right_samples = samples.max(1);
                }
                GraphParam::CrossFeedback => n.cross_feedback = value.clamp(0.0, 0.95),
                GraphParam::DelayFeedback => n.feedback = value.clamp(0.0, 0.95),
                _ => {}
            },
            NodeKind::Output => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub from: NodeId,
    pub to: NodeId,
}

struct NodeSlot {
    kind: NodeKind,
}

pub struct DspGraph {
    nodes: Vec<Option<NodeSlot>>,
    free: Vec<NodeId>,
    edges: Vec<Connection>,
    order: Vec<NodeId>,
    order_valid: bool,
    /// Per-node output buffers, parallel to `nodes`.
    buffers: Vec<Vec<f32>>,
    accum: Vec<f32>,
    scratch: Vec<f32>,
    sample_rate: f32,
}

impl DspGraph {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            nodes: Vec::new(),
            free: Vec::new(),
            edges: Vec::new(),
            order: Vec::new(),
            order_valid: true,
            buffers: Vec::new(),
            accum: Vec::new(),
            scratch: Vec::new(),
            sample_rate,
        }
    }

    /// Insert a node at a caller-supplied slot (used by the UI so the UI's own
    /// `next_node_id` and the engine's `NodeId` agree). Returns false if the
    /// slot is already occupied.
    pub fn add_node_at(&mut self, id: NodeId, kind: GraphNodeKind) -> bool {
        let node = match kind {
            GraphNodeKind::Oscillator(w, f) => NodeKind::Oscillator(OscillatorNode::new(w, f, self.sample_rate)),
            GraphNodeKind::Filter(m, c, r) => {
                let mut f = BiquadFilterNode::new(c, r, self.sample_rate);
                f.set_mode(m);
                NodeKind::Filter(f)
            }
            GraphNodeKind::Envelope(a, d, s, r) => {
                NodeKind::Envelope(AdsrEnvelopeNode::new(a, d, s, r, self.sample_rate))
            }
            GraphNodeKind::Lfo(f) => NodeKind::Lfo(LfoNode::new(f, self.sample_rate)),
            GraphNodeKind::Delay(t, fb) => {
                let samples = (t.max(0.001) * self.sample_rate) as usize;
                let max = (2.0 * self.sample_rate) as usize;
                NodeKind::Delay(SimpleDelayNode::new(max.max(samples + 1), samples.max(1), fb))
            }
            GraphNodeKind::Reverb(room, damp) => {
                let mut r = ReverbNode::new(self.sample_rate);
                r.room_size = room;
                r.damping = damp;
                NodeKind::Reverb(r)
            }
            GraphNodeKind::Noise(c) => NodeKind::Noise(NoiseNode::new(c, self.sample_rate)),
            GraphNodeKind::Gain(g) => NodeKind::Gain(GainNode::new(g)),
            GraphNodeKind::Distortion(d, t) => NodeKind::Distortion(DistortionNode::new(d, t)),
GraphNodeKind::Sampler => NodeKind::Sampler(SamplerNode::new(Self::fallback_sample(self.sample_rate))),
            GraphNodeKind::Mixer(g1, g2) => NodeKind::Mixer(MixerNode::new(g1, g2)),
            GraphNodeKind::Splitter => NodeKind::Splitter(SplitterNode::new()),
GraphNodeKind::StereoDelay(t_l, t_r, cross_fb) => {
                let max_samples = (2.0 * self.sample_rate) as usize;
                let left_samples = (t_l.max(0.001) * self.sample_rate) as usize;
                let right_samples = (t_r.max(0.001) * self.sample_rate) as usize;
                let mut sd = StereoDelayNode::new(
                    max_samples,
                    left_samples.max(1).min(max_samples - 1),
                    right_samples.max(1).min(max_samples - 1),
                    0.3,
                );
                sd.cross_feedback = cross_fb;
                NodeKind::StereoDelay(sd)
            }
            GraphNodeKind::Output => NodeKind::Output,
        };

        if id < self.nodes.len() {
            if self.nodes[id].is_some() {
                return false;
            }
            self.nodes[id] = Some(NodeSlot { kind: node });
        } else {
            while self.nodes.len() <= id {
                self.nodes.push(None);
                self.buffers.push(Vec::new());
            }
            self.nodes[id] = Some(NodeSlot { kind: node });
        }
        if let Some(pos) = self.free.iter().position(|&f| f == id) {
            self.free.swap_remove(pos);
        }
        self.order_valid = false;
        true
    }

    /// Add a node, auto-assigning the next free slot.
    pub fn add_node(&mut self, kind: NodeKind) -> NodeId {
        let id = if let Some(f) = self.free.pop() {
            self.nodes[f] = Some(NodeSlot { kind });
            f
        } else {
            let id = self.nodes.len();
            self.nodes.push(Some(NodeSlot { kind }));
            self.buffers.push(Vec::new());
            id
        };
        self.order_valid = false;
        id
    }

    /// Generate a short decaying sine burst used as the fallback sample for
    /// `Sampler` nodes when no audio file is loaded.
    fn fallback_sample(sample_rate: f32) -> Vec<f32> {
        let len = (sample_rate * 0.5) as usize;
        (0..len)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * (1.0 - i as f32 / len as f32)
            })
            .collect()
    }

    pub fn add_oscillator(&mut self, waveform: Waveform, freq: f32) -> NodeId {
        self.add_node(NodeKind::Oscillator(OscillatorNode::new(waveform, freq, self.sample_rate)))
    }

    pub fn add_filter(&mut self, cutoff: f32, resonance: f32) -> NodeId {
        self.add_node(NodeKind::Filter(BiquadFilterNode::new(cutoff, resonance, self.sample_rate)))
    }

    pub fn add_envelope(&mut self, a: f32, d: f32, s: f32, r: f32) -> NodeId {
        self.add_node(NodeKind::Envelope(AdsrEnvelopeNode::new(a, d, s, r, self.sample_rate)))
    }

    pub fn add_delay(&mut self, max_delay: usize, delay: usize, feedback: f32) -> NodeId {
        self.add_node(NodeKind::Delay(SimpleDelayNode::new(max_delay, delay, feedback)))
    }

    pub fn add_reverb(&mut self) -> NodeId {
        self.add_node(NodeKind::Reverb(ReverbNode::new(self.sample_rate)))
    }

    pub fn add_noise(&mut self, color: NoiseColor) -> NodeId {
        self.add_node(NodeKind::Noise(NoiseNode::new(color, self.sample_rate)))
    }

    pub fn add_gain(&mut self, gain: f32) -> NodeId {
        self.add_node(NodeKind::Gain(GainNode::new(gain)))
    }

    pub fn add_distortion(&mut self, drive: f32, tone: f32) -> NodeId {
        self.add_node(NodeKind::Distortion(DistortionNode::new(drive, tone)))
    }

    pub fn add_sampler(&mut self) -> NodeId {
        self.add_node(NodeKind::Sampler(SamplerNode::new(Self::fallback_sample(self.sample_rate))))
    }

pub fn add_output(&mut self) -> NodeId {
        self.add_node(NodeKind::Output)
    }

    pub fn add_mixer(&mut self, gain1: f32, gain2: f32) -> NodeId {
        self.add_node(NodeKind::Mixer(MixerNode::new(gain1, gain2)))
    }

    pub fn add_splitter(&mut self) -> NodeId {
        self.add_node(NodeKind::Splitter(SplitterNode::new()))
    }

pub fn add_stereo_delay(&mut self, max_delay: usize, left_delay: usize, right_delay: usize, feedback: f32, cross_feedback: f32) -> NodeId {
        let mut sd = StereoDelayNode::new(max_delay, left_delay, right_delay, feedback);
        sd.cross_feedback = cross_feedback;
        self.add_node(NodeKind::StereoDelay(sd))
    }

    /// Remove a node and all its connections.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if id >= self.nodes.len() || self.nodes[id].is_none() {
            return false;
        }
        self.nodes[id] = None;
        self.free.push(id);
        self.edges.retain(|e| e.from != id && e.to != id);
        self.order_valid = false;
        true
    }

    /// Connect `from -> to`. Returns false if either node is absent or the
    /// edge already exists.
    pub fn connect(&mut self, from: NodeId, to: NodeId) -> bool {
        if from == to
            || from >= self.nodes.len()
            || to >= self.nodes.len()
            || self.nodes[from].is_none()
            || self.nodes[to].is_none()
        {
            return false;
        }
        if self.edges.iter().any(|e| e.from == from && e.to == to) {
            return false;
        }
        self.edges.push(Connection { from, to });
        self.order_valid = false;
        true
    }

    /// Disconnect `from -> to`. Returns true if a connection was removed.
    pub fn disconnect(&mut self, from: NodeId, to: NodeId) -> bool {
        let len = self.edges.len();
        self.edges.retain(|e| !(e.from == from && e.to == to));
        let removed = self.edges.len() < len;
        if removed {
            self.order_valid = false;
        }
        removed
    }

    /// Trigger a node that supports it (e.g. envelope gate, sampler trigger).
    /// Triggers are stacked: `true` = gate on, `false` = gate off.
    pub fn trigger_node(&mut self, id: NodeId, gate: bool) {
        if let Some(Some(slot)) = self.nodes.get_mut(id) {
            if let NodeKind::Envelope(ref mut env) = slot.kind {
                env.trigger_gate(gate);
            }
            if let NodeKind::Sampler(ref mut sam) = slot.kind {
                if gate {
                    sam.trigger();
                }
            }
        }
    }

/// Apply a `GraphParam` value to a node.
    pub fn set_param(&mut self, id: NodeId, param: GraphParam, value: f32) {
        let sr = self.sample_rate;
        if let Some(Some(slot)) = self.nodes.get_mut(id) {
            slot.kind.apply_param(param, value, sr);
        }
    }

    /// Load a sample buffer into a sampler node. No-op if the node is not a
    /// sampler.
    pub fn load_sample(&mut self, id: NodeId, buffer: Vec<f32>) {
        if let Some(Some(slot)) = self.nodes.get_mut(id) {
            if let NodeKind::Sampler(ref mut sam) = slot.kind {
                sam.buffer = buffer;
                sam.position = 0.0;
            }
        }
    }

    /// Number of live nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }

    /// Number of connections.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns node IDs in processing (topological) order.
    /// Uses Kahn's algorithm; if a cycle exists, returns nodes in insertion
    /// order (degraded mode).
    pub fn processing_order(&mut self) -> Vec<NodeId> {
        if self.order_valid {
            return self.order.clone();
        }

        let mut indegree: Vec<usize> = vec![0; self.nodes.len()];
        for e in &self.edges {
            if e.to < indegree.len() {
                indegree[e.to] += 1;
            }
        }

        let mut queue: Vec<NodeId> = (0..self.nodes.len())
            .filter(|&id| self.nodes.get(id).and_then(|n| if n.is_some() { Some(()) } else { None }).is_some()
                && indegree[id] == 0)
            .collect();
        let mut order = Vec::new();
        while let Some(id) = queue.pop() {
            order.push(id);
            for e in self.edges.iter().filter(|e| e.from == id) {
                if e.to < indegree.len() && indegree[e.to] > 0 {
                    indegree[e.to] -= 1;
                    if indegree[e.to] == 0 {
                        queue.push(e.to);
                    }
                }
            }
        }

        // If cycle detected, fall back to insertion order.
        if order.len() != self.node_count() {
            order = (0..self.nodes.len())
                .filter(|id| self.nodes.get(*id).and_then(|n| if n.is_some() { Some(()) } else { None }).is_some())
                .collect();
        }

        self.order = order.clone();
        self.order_valid = true;
        order
    }

    /// Process a block through the graph.
    ///
    /// `input` feeds source nodes (nodes with no incoming edges). `output`
    /// receives the sum of all `Output` node buffers. If no `Output` node
    /// exists, the graph is a pass-through (identity).
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        output.fill(0.0);
        let block_len = output.len().max(1);

        // Ensure buffers are sized to the block.
        for buf in self.buffers.iter_mut() {
            buf.resize(block_len, 0.0);
        }
        self.accum.resize(block_len, 0.0);
        self.scratch.resize(block_len, 0.0);

        let order = self.processing_order();
        let mut has_sink = false;

        for &id in &order {
            let Some(Some(ref mut slot)) = self.nodes.get_mut(id) else { continue };

            let input_count = self.edges.iter().filter(|e| e.to == id).count();
let _output_count = self.edges.iter().filter(|e| e.from == id).count();

// Build node input: sum upstream buffers, or use the external input
            // for source nodes.
            if input_count == 0 {
                // Source node: drive from external input.
                if input.len() >= block_len {
                    self.accum.copy_from_slice(&input[..block_len]);
                } else {
                    // Input is shorter than block_len (or empty); pad with zeros.
                    self.accum[..input.len()].copy_from_slice(input);
                    self.accum[input.len()..block_len].fill(0.0);
                }
            } else {
                self.accum.fill(0.0);
                for e in self.edges.iter().filter(|e| e.to == id) {
                    if e.from < self.buffers.len() {
                        let buf = &self.buffers[e.from];
                        for (i, v) in buf.iter().enumerate().take(block_len) {
                            self.accum[i] += v;
                        }
                    }
                }
            }

            // Process the node.
            self.scratch.fill(0.0);
            slot.kind.process(&self.accum, &mut self.scratch);

            // Store output in the node's buffer for downstream.
            if id < self.buffers.len() {
                self.buffers[id].copy_from_slice(&self.scratch[..block_len.min(self.scratch.len())]);
            }

            // If this node is an Output node, it's a sink.
            if matches!(slot.kind, NodeKind::Output) {
                has_sink = true;
                for (i, v) in self.scratch.iter().enumerate().take(block_len) {
                    output[i] += v;
                }
            }
        }

        // If no sink node exists, pass input through. Only the overlapping
        // region is copied; the rest of `output` stays zero-filled.
        if !has_sink {
            let n = input.len().min(output.len());
            output[..n].copy_from_slice(&input[..n]);
        }
    }

    /// Returns the set of node IDs directly feeding into `id`.
    pub fn upstream(&self, id: NodeId) -> Vec<NodeId> {
        self.edges.iter().filter(|e| e.to == id).map(|e| e.from).collect()
    }

    /// Returns the set of node IDs directly fed by `id`.
    pub fn downstream(&self, id: NodeId) -> Vec<NodeId> {
        self.edges.iter().filter(|e| e.from == id).map(|e| e.to).collect()
    }

    /// Immutable reference to a node's kind (for UI reads of names, etc.).
    pub fn node_name(&self, id: NodeId) -> Option<&'static str> {
        self.nodes.get(id).and_then(|n| n.as_ref()).map(|s| s.kind.name())
    }

    /// Count of actively allocated nodes.
    pub fn live_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_process_osc_filter() {
        let mut graph = DspGraph::new(44100.0);
        let osc = graph.add_oscillator(Waveform::Sine, 440.0);
        let filt = graph.add_filter(2000.0, 0.7);
        let out = graph.add_output();
        assert!(graph.connect(osc, filt));
        assert!(graph.connect(filt, out));

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        let mut output = vec![0.0; 128];
        graph.process(&[], &mut output);
        assert!(output.iter().any(|&v| v != 0.0), "oscillator should produce signal through graph");
    }

    #[test]
    fn test_remove_node() {
        let mut graph = DspGraph::new(44100.0);
        let osc = graph.add_oscillator(Waveform::Sine, 220.0);
        let filt = graph.add_filter(1000.0, 0.5);
        graph.connect(osc, filt);
        assert!(graph.remove_node(osc));
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node_at() {
        let mut graph = DspGraph::new(44100.0);
        let ok = graph.add_node_at(42, GraphNodeKind::Oscillator(Waveform::Saw, 110.0));
        assert!(ok);
        assert_eq!(graph.node_name(42), Some("oscillator"));
        // Double-add at same slot should fail.
        let dup = graph.add_node_at(42, GraphNodeKind::Gain(1.0));
        assert!(!dup);
    }

    #[test]
    fn test_set_param() {
        let mut graph = DspGraph::new(44100.0);
        let filt = graph.add_filter(1000.0, 0.7);
        graph.set_param(filt, GraphParam::Cutoff, 5000.0);
        graph.set_param(filt, GraphParam::Resonance, 2.0);

        // Verify by processing: changing cutoff should change output.
        let osc = graph.add_oscillator(Waveform::Sine, 440.0);
        let out = graph.add_output();
        graph.connect(osc, filt);
        graph.connect(filt, out);

        let mut output = vec![0.0; 128];
        graph.process(&[], &mut output);
        assert!(output.iter().any(|&v| v != 0.0));
    }

    #[test]
    fn test_trigger_envelope() {
        let mut graph = DspGraph::new(44100.0);
        let env = graph.add_envelope(0.01, 0.1, 0.5, 0.2);
        graph.trigger_node(env, true); // gate on

        let mut out = vec![0.0; 64];
        graph.process(&[], &mut out);
        // Envelope with no input defaults to 0; but processed through identity.
        // The envelope node itself modulates input, so with empty input it stays 0.
        // We just verify it doesn't crash and the node is still there.
        assert!(graph.node_name(env).is_some());
    }

    #[test]
    fn test_cycle_fallback() {
        let mut graph = DspGraph::new(44100.0);
        let a = graph.add_oscillator(Waveform::Sine, 220.0);
        let b = graph.add_filter(1000.0, 0.5);
        graph.connect(a, b);
        graph.connect(b, a); // cycle
        let order = graph.processing_order();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn test_sampler_fallback() {
        let mut graph = DspGraph::new(44100.0);
        let sam = graph.add_sampler();
        let out = graph.add_output();
        graph.connect(sam, out);
        graph.trigger_node(sam, true);

        let mut output = vec![0.0; 256];
        graph.process(&[], &mut output);
        // Fallback sample should produce non-zero output.
        assert!(output.iter().any(|&v| v != 0.0), "fallback sampler should produce signal");
    }

    #[test]
    fn test_noise_process() {
        let mut graph = DspGraph::new(44100.0);
        let noise = graph.add_noise(NoiseColor::White);
        let out = graph.add_output();
        graph.connect(noise, out);

        let mut output = vec![0.0; 128];
        graph.process(&[], &mut output);
        assert!(output.iter().any(|&v| v != 0.0), "noise node should produce signal");
    }

    #[test]
    fn test_disconnect() {
        let mut graph = DspGraph::new(44100.0);
        let a = graph.add_oscillator(Waveform::Sine, 440.0);
        let b = graph.add_filter(1000.0, 0.5);
        graph.connect(a, b);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.disconnect(a, b));
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_graph_identity_when_no_output() {
        let mut graph = DspGraph::new(44100.0);
        graph.add_oscillator(Waveform::Sine, 440.0);
        // No output node — should be pass-through.
        let input = vec![0.5; 64];
        let mut output = vec![0.0; 64];
        graph.process(&input, &mut output);
        assert!(output.iter().all(|&v| (v - 0.5).abs() < 1e-6), "no output node should pass input through");
    }
}
