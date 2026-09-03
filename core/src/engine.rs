use crossbeam_channel::{unbounded, Receiver, Sender};
use num_complex::Complex;
use crate::config::EngineConfig;
use crate::dsp::extra_nodes::ReverbNode;
use crate::dsp::graph::{DspGraph, GraphNodeKind, GraphParam, NodeId};
use crate::dsp::{AdsrEnvelopeNode, BiquadFilterNode, DspNode, OscillatorNode, SimpleDelayNode, Waveform};
use crate::instance::SuperInstancePool;
use crate::nn::ComplexNeuralNetwork;
use crate::prime::PrimeScheduler;

/// Named audio parameters addressable from M4L devices (text protocol `param`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Param {
    FilterCutoff,
    FilterResonance,
    FilterMode,
    ReverbMix,
    DelayTime,
    DelayFeedback,
    EnvAttack,
    EnvRelease,
    Waveform,
    MasterGain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    Synth,
    Sampler,
    Effect,
    Controller,
}

impl EngineMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "sampler" => EngineMode::Sampler,
            "effect" => EngineMode::Effect,
            "controller" => EngineMode::Controller,
            _ => EngineMode::Synth,
        }
    }
}

#[derive(Debug)]
pub enum EngineCommand {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    AllNotesOff,
    SetPrimeBase(u32),
    SetPrimePhase(f32),
    SetNnSensitivity(f32),
    SetInstanceDensity(u32),
    SetInstanceSpread(f32),
    SetMode(EngineMode),
    SetParam(Param, f32),
    /// Add a node to the user's DSP graph.
    GraphAddNode { id: NodeId, kind: GraphNodeKind },
    /// Remove a node and its connections.
    GraphRemoveNode { id: NodeId },
    /// Connect `from -> to`.
    GraphConnect { from: NodeId, to: NodeId },
    /// Disconnect `from -> to`.
    GraphDisconnect { from: NodeId, to: NodeId },
    /// Set a parameter on a graph node.
    GraphSetParam { id: NodeId, param: GraphParam, value: f32 },
    /// Trigger a graph node (envelope gate, sampler trigger).
    GraphTriggerNode { id: NodeId, gate: bool },
    /// Clear the entire graph.
    GraphClear,
}

pub struct CoreEngine {
    pub config: EngineConfig,
    pub pool: SuperInstancePool,
    pub prime_scheduler: PrimeScheduler,
    pub complex_nn: ComplexNeuralNetwork,
    pub osc: OscillatorNode,
    pub filter: BiquadFilterNode,
    pub env: AdsrEnvelopeNode,
    pub reverb: ReverbNode,
    pub delay: SimpleDelayNode,
    pub master_gain: f32,
    /// User's DSP graph — when nodes exist, voice output routes through it.
    pub graph: DspGraph,
    /// Base filter cutoff set by the user (M4L `param filter_cutoff`). The
    /// complex NN modulates *around* this base, scaled by `nn_sensitivity`, so
    /// manual control is not clobbered every block.
    pub filter_cutoff_base: f32,
    pub cmd_rx: Receiver<EngineCommand>,
    pub config_rx: Option<crossbeam_channel::Receiver<EngineConfig>>,
    pub sample_rate: f32,
    pub output_peak: f32,
    /// Tracks whether the master ADSR is currently gated on (any voice active).
    pub env_gated: bool,
}

impl CoreEngine {
    pub fn new(config: EngineConfig, sample_rate: f32) -> (Self, Sender<EngineCommand>) {
        let (tx, rx) = unbounded();
        let density = config.instance_density.max(1) as usize;
        let engine = Self {
            config,
            pool: SuperInstancePool::new(density),
            prime_scheduler: PrimeScheduler::new(),
            complex_nn: ComplexNeuralNetwork::new(4, 8, 4),
            osc: OscillatorNode::new(Waveform::Saw, 440.0, sample_rate),
            filter: BiquadFilterNode::new(1200.0, 0.7, sample_rate),
            env: AdsrEnvelopeNode::new(0.01, 0.2, 0.7, 0.3, sample_rate),
            reverb: ReverbNode::new(sample_rate),
delay: SimpleDelayNode::new(sample_rate as usize, (sample_rate * 0.25) as usize, 0.3),
            master_gain: 0.8,
            graph: DspGraph::new(sample_rate),
            filter_cutoff_base: 1200.0,
            cmd_rx: rx,
            config_rx: None,
            sample_rate,
            output_peak: 0.0,
            env_gated: false,
        };
        (engine, tx)
    }

    /// Attach a hot-reload config channel (produced by `ConfigWatcher::watch`).
    pub fn attach_config_channel(&mut self, rx: crossbeam_channel::Receiver<EngineConfig>) {
        self.config_rx = Some(rx);
    }

    /// Apply a reloaded config live to the engine without stopping the audio thread.
    pub fn apply_config(&mut self, config: EngineConfig) {
        // Update instance pool size if density changed.
        let new_density = config.instance_density.max(1) as usize;
        if new_density != self.pool.capacity() {
            self.pool.resize(new_density);
        }
        self.pool.set_spread(config.instance_spread);
        self.config = config;
    }

    fn poll_config_reload(&mut self) {
        // Clone the receiver so we can mutate `self` inside the loop.
        let Some(rx) = self.config_rx.clone() else { return };
        while let Ok(cfg) = rx.try_recv() {
            self.apply_config(cfg);
        }
    }

    /// Apply a named parameter to the engine's active DSP chain.
    ///
    /// All values are normalized 0..=1 (except waveform which is an index),
    /// matching what an M4L `live.dial`/`live.slider` produces.
    pub fn set_param(&mut self, param: Param, value: f32) {
        match param {
            Param::FilterCutoff => {
                // 20 Hz .. 20 kHz
                self.filter_cutoff_base = 20.0 + value.clamp(0.0, 1.0) * 19980.0;
                self.filter.cutoff = self.filter_cutoff_base;
            }
            Param::FilterResonance => {
                self.filter.resonance = 0.1 + value.clamp(0.0, 1.0) * 8.9;
            }
            Param::FilterMode => {
                let mode = match (value as i32).clamp(0, 3) {
                    1 => crate::dsp::FilterMode::HighPass,
                    2 => crate::dsp::FilterMode::BandPass,
                    3 => crate::dsp::FilterMode::Notch,
                    _ => crate::dsp::FilterMode::LowPass,
                };
                self.filter.set_mode(mode);
            }
            Param::ReverbMix => {
                self.reverb.room_size = value.clamp(0.0, 1.0);
            }
            Param::DelayTime => {
                // 0 .. 1.0 s delay
                let samples = (value.clamp(0.0, 1.0) * self.sample_rate).max(1.0) as usize;
                self.delay.delay_samples = samples.min(self.delay.buffer_len());
            }
            Param::DelayFeedback => {
                self.delay.feedback = value.clamp(0.0, 0.95);
            }
            Param::EnvAttack => {
                self.env.attack = value.clamp(0.0, 1.0);
            }
            Param::EnvRelease => {
                self.env.release = value.clamp(0.0, 1.0);
            }
            Param::Waveform => {
                let wave = match (value as i32).clamp(0, 3) {
                    1 => Waveform::Saw,
                    2 => Waveform::Square,
                    3 => Waveform::Triangle,
                    _ => Waveform::Sine,
                };
                self.osc.waveform = wave;
            }
            Param::MasterGain => {
                self.master_gain = value.clamp(0.0, 1.0);
            }
        }
    }

    pub fn process_block(&mut self, output: &mut [f32]) {
        // 1. Drain hot-reloaded config channel first.
        self.poll_config_reload();

        // 2. Process control commands.
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                EngineCommand::NoteOn { note, velocity } => {
                    let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
                    self.pool.allocate(freq, velocity);
                }
                EngineCommand::NoteOff { note } => {
                    let freq = 440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0);
                    self.pool.release_note(freq);
                }
                EngineCommand::AllNotesOff => {
                    self.pool.release_all();
                }
                EngineCommand::SetPrimeBase(base) => {
                    self.config.prime_base = base;
                    self.prime_scheduler.set_base(base);
                }
                EngineCommand::SetPrimePhase(phase) => {
                    self.config.prime_phase = phase.clamp(0.0, 1.0);
                }
                EngineCommand::SetNnSensitivity(sens) => {
                    self.config.nn_sensitivity = sens;
                }
                EngineCommand::SetInstanceDensity(density) => {
                    self.config.instance_density = density.max(1);
                    self.pool.resize(self.config.instance_density as usize);
                }
                EngineCommand::SetInstanceSpread(spread) => {
                    self.config.instance_spread = spread.clamp(0.0, 1.0);
                    self.pool.set_spread(spread);
                }
                EngineCommand::SetMode(mode) => {
                    self.config.mode = match mode {
                        EngineMode::Synth => "synth".to_string(),
                        EngineMode::Sampler => "sampler".to_string(),
                        EngineMode::Effect => "effect".to_string(),
                        EngineMode::Controller => "controller".to_string(),
                    };
                }
EngineCommand::SetParam(param, value) => {
                    self.set_param(param, value);
                }
                EngineCommand::GraphAddNode { id, kind } => {
                    self.graph.add_node_at(id, kind);
                }
                EngineCommand::GraphRemoveNode { id } => {
                    self.graph.remove_node(id);
                }
                EngineCommand::GraphConnect { from, to } => {
                    self.graph.connect(from, to);
                }
                EngineCommand::GraphDisconnect { from, to } => {
                    self.graph.disconnect(from, to);
                }
                EngineCommand::GraphSetParam { id, param, value } => {
                    self.graph.set_param(id, param, value);
                }
                EngineCommand::GraphTriggerNode { id, gate } => {
                    self.graph.trigger_node(id, gate);
                }
                EngineCommand::GraphClear => {
                    // Rebuild from scratch
                    self.graph = DspGraph::new(self.sample_rate);
                }
            }
        }

        self.prime_scheduler.tick();
        // Prime gate derived from the configured base prime value.
        let prime_gate = self.prime_scheduler.gate_signal();

        // 3. Process complex NN for parameter morphing. The NN modulates
        //    *around* the user-set `filter_cutoff_base`, scaled by
        //    `nn_sensitivity`, so manual/M4L control is not clobbered and
        //    `nn_sensitivity = 0` means the base cutoff is used directly.
        let z_in = vec![
            Complex::new(self.config.prime_phase, 0.0),
            Complex::new(self.config.nn_sensitivity, 0.5),
            Complex::new(0.1, 0.2),
            Complex::new(0.3, 0.4),
        ];
        let z_out = self.complex_nn.process_latent(&z_in);
        if let Some(c) = z_out.first() {
            let morph = (c.norm() - 1.0).clamp(-0.5, 0.5);
            let sensitivity = self.config.nn_sensitivity.clamp(0.0, 1.0);
            let cutoff = self.filter_cutoff_base * (1.0 + morph * sensitivity * 2.0);
            self.filter.cutoff = cutoff.clamp(20.0, 20000.0);
        }

        // 4. Master ADSR gate follows pool activity: fade in/out with the
        //    first voice that starts and the last voice that stops.
        let active = self.pool.active_count() > 0;
        if active && !self.env_gated {
            self.env.trigger_gate(true);
            self.env_gated = true;
        } else if !active && self.env_gated {
            self.env.trigger_gate(false);
            self.env_gated = false;
        }

// 5. Render the voice pool polyphonically (the actual audible source).
        let mut voice_out = vec![0.0; output.len()];
        self.render_active_voices(&mut voice_out);

        // Route through the user's DSP graph if nodes exist, otherwise fall
        // back to the built-in fixed chain (Filter -> Env -> Delay -> Reverb).
        if self.graph.live_count() > 0 {
            self.graph.process(&voice_out, output);
        } else {
            let mut filter_out = vec![0.0; output.len()];
            self.filter.process(&voice_out, &mut filter_out);
            let mut env_out = vec![0.0; output.len()];
            self.env.process(&filter_out, &mut env_out);
            let mut delay_out = vec![0.0; output.len()];
            self.delay.process(&env_out, &mut delay_out);
            self.reverb.process(&delay_out, output);
        }

        // 6. Prime gate ducking + instance spread modulation (subtle timbral variation)
        //    + master gain (controllable from M4L `param master_gain`).
        let spread = self.config.instance_spread;
        for (i, sample) in output.iter_mut().enumerate() {
            let gate_gain = if prime_gate { 1.0 } else { 0.85 };
            let spread_mod = 1.0 + spread * (i as f32 * 0.001).sin() * 0.02;
            *sample *= gate_gain * spread_mod * self.master_gain;
        }

        // 7. Track output peak for UI/monitoring.
        self.output_peak = output.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    }

    /// Render all active voice instances into `output` (mono block).
    ///
    /// Each active voice produces a detuned saw/sine blend at its pitch,
    /// scaled by velocity, with a natural exponential release so notes die out
    /// even when no explicit `NoteOff` is received. Voices that reach silence
    /// are automatically deactivated.
    pub fn render_active_voices(&mut self, output: &mut [f32]) {
        let release_rate = 1.0 / (0.25 * self.sample_rate).max(1.0);
        for out in output.iter_mut() {
            let mut sum = 0.0;
            for v in self.pool.iter_active_mut() {
                let freq = (v.pitch + v.detune).max(1.0);
                let phase_inc = freq / self.sample_rate;
                let saw = 2.0 * v.phase - 1.0;
                let sine = (v.phase * 2.0 * std::f32::consts::PI).sin();
                let wave = 0.7 * saw + 0.3 * sine;
                sum += wave * v.envelope * v.velocity;
                v.phase = (v.phase + phase_inc) % 1.0;
                v.envelope -= release_rate;
                if v.envelope <= 0.001 {
                    v.envelope = 0.0;
                    v.active = false;
                }
            }
            *out = sum;
        }
    }
}
