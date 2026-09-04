use eframe::egui;
use modurust_core::config::EngineConfig;
use modurust_core::dsp::graph::GraphNodeKind;
use modurust_core::dsp::{FilterMode, Waveform, NoiseColor};
use modurust_core::engine::{EngineCommand, Param, EngineMode};
use modurust_core::io::audio::CpalEngineHost;
use std::sync::Arc;

const CONFIG_PATH: &str = "config.json";
const SAMPLE_RATE: f32 = 44100.0;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Modurust-Lite - Visual Node Editor",
        options,
        Box::new(|_cc| Box::new(StandaloneApp::new())),
    )
}

struct StandaloneApp {
    config: EngineConfig,
    host: Option<CpalEngineHost>,
    audio_error: String,
    nodes: Vec<VisualNode>,
    connections: Vec<Connection>,
    selected_node: Option<usize>,
    dragging_node: Option<usize>,
    drag_start_offset: egui::Vec2,
    graph_offset: egui::Vec2,
    peak_meter: f32,
    voice_count: usize,
    mode: String,
    filter_cutoff: f32,
    filter_resonance: f32,
    filter_mode_idx: i32,
    reverb_mix: f32,
    delay_time: f32,
    delay_feedback: f32,
    env_attack: f32,
    env_release: f32,
    waveform_idx: i32,
    master_gain: f32,
}

#[derive(Debug, Clone)]
struct VisualNode {
    id: usize,
    node_type: NodeType,
    pos: egui::Vec2,
    params: NodeParams,
}

#[derive(Debug, Clone, PartialEq)]
enum NodeType {
    Oscillator, Filter, Envelope, Lfo, Delay, Reverb, Noise, Gain,
    Distortion, Sampler, Mixer, Splitter, StereoDelay, Output, Input,
}

impl NodeType {
    fn name(&self) -> &'static str {
        match self {
            NodeType::Oscillator => "Oscillator", NodeType::Filter => "Filter",
            NodeType::Envelope => "Envelope", NodeType::Lfo => "LFO",
            NodeType::Delay => "Delay", NodeType::Reverb => "Reverb",
            NodeType::Noise => "Noise", NodeType::Gain => "Gain",
            NodeType::Distortion => "Distortion", NodeType::Sampler => "Sampler",
            NodeType::Mixer => "Mixer", NodeType::Splitter => "Splitter",
            NodeType::StereoDelay => "Stereo Delay", NodeType::Output => "Output",
            NodeType::Input => "Input",
        }
    }
    fn color(&self) -> egui::Color32 {
        match self {
            NodeType::Oscillator => egui::Color32::from_rgb(66, 133, 244),
            NodeType::Filter => egui::Color32::from_rgb(52, 168, 83),
            NodeType::Envelope => egui::Color32::from_rgb(251, 188, 5),
            NodeType::Lfo => egui::Color32::from_rgb(156, 39, 176),
            NodeType::Delay => egui::Color32::from_rgb(0, 172, 193),
            NodeType::Reverb => egui::Color32::from_rgb(94, 53, 177),
            NodeType::Noise => egui::Color32::from_rgb(244, 67, 54),
            NodeType::Gain => egui::Color32::from_rgb(96, 125, 139),
            NodeType::Distortion => egui::Color32::from_rgb(255, 87, 34),
            NodeType::Sampler => egui::Color32::from_rgb(233, 30, 99),
            NodeType::Mixer => egui::Color32::from_rgb(139, 195, 74),
            NodeType::Splitter => egui::Color32::from_rgb(63, 81, 181),
            NodeType::StereoDelay => egui::Color32::from_rgb(0, 188, 212),
            NodeType::Output => egui::Color32::from_rgb(76, 175, 80),
            NodeType::Input => egui::Color32::from_rgb(255, 152, 0),
        }
    }
    fn to_graph_kind(&self, params: &NodeParams) -> GraphNodeKind {
        match self {
            NodeType::Oscillator => GraphNodeKind::Oscillator(params.waveform, params.frequency),
            NodeType::Filter => GraphNodeKind::Filter(params.filter_mode, params.cutoff, params.resonance),
            NodeType::Envelope => GraphNodeKind::Envelope(params.attack, params.decay, params.sustain, params.release),
            NodeType::Lfo => GraphNodeKind::Lfo(params.lfo_frequency),
            NodeType::Delay => GraphNodeKind::Delay(params.delay_time, params.delay_feedback),
            NodeType::Reverb => GraphNodeKind::Reverb(params.reverb_room, params.reverb_damping),
            NodeType::Noise => GraphNodeKind::Noise(params.noise_color),
            NodeType::Gain => GraphNodeKind::Gain(params.gain),
            NodeType::Distortion => GraphNodeKind::Distortion(params.drive, params.tone),
            NodeType::Sampler => GraphNodeKind::Sampler,
            NodeType::Mixer => GraphNodeKind::Mixer(params.mixer_gain1, params.mixer_gain2),
            NodeType::Splitter => GraphNodeKind::Splitter,
            NodeType::StereoDelay => GraphNodeKind::StereoDelay(params.stereo_delay_left, params.stereo_delay_right, params.cross_feedback),
            NodeType::Output => GraphNodeKind::Output,
            NodeType::Input => GraphNodeKind::Gain(1.0),
        }
    }
}

#[derive(Debug, Clone)]
struct NodeParams {
    waveform: Waveform, frequency: f32, filter_mode: FilterMode, cutoff: f32,
    resonance: f32, attack: f32, decay: f32, sustain: f32, release: f32,
    lfo_frequency: f32, delay_time: f32, delay_feedback: f32,
    reverb_room: f32, reverb_damping: f32, noise_color: NoiseColor,
    gain: f32, drive: f32, tone: f32,
    #[allow(dead_code)]
    playback_rate: f32,
    mixer_gain1: f32, mixer_gain2: f32, stereo_delay_left: f32,
    stereo_delay_right: f32, cross_feedback: f32,
}

impl Default for NodeParams {
    fn default() -> Self {
        Self {
            waveform: Waveform::Sine, frequency: 440.0, filter_mode: FilterMode::LowPass,
            cutoff: 1000.0, resonance: 0.7, attack: 0.01, decay: 0.1, sustain: 0.7,
            release: 0.3, lfo_frequency: 1.0, delay_time: 0.25, delay_feedback: 0.3,
            reverb_room: 0.85, reverb_damping: 0.5, noise_color: NoiseColor::White,
            gain: 1.0, drive: 2.0, tone: 0.5,
            playback_rate: 1.0,
            mixer_gain1: 1.0, mixer_gain2: 1.0, stereo_delay_left: 0.1,
            stereo_delay_right: 0.15, cross_feedback: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Connection { from: usize, to: usize }

impl StandaloneApp {
    fn new() -> Self {
        let config = EngineConfig::load_from_file(CONFIG_PATH);
        let host = CpalEngineHost::new(config.clone(), SAMPLE_RATE);
        let bridge = modurust_core::comm::UdpOscBridge::new_with_status(
            config.udp_port, config.osc_port, config.status_port,
        );
        let command_tx = host.command_sender();
        bridge.start_udp_listener(command_tx.clone(), Arc::new(std::sync::atomic::AtomicBool::new(true)));
        bridge.start_osc_listener(command_tx.clone(), Arc::new(std::sync::atomic::AtomicBool::new(true)));
        {
            let engine = host.engine();
            bridge.start_status_broadcast(
                move || modurust_core::comm::UdpOscBridge::status_from_engine(&engine),
                50, Arc::new(std::sync::atomic::AtomicBool::new(true)),
            );
        }
        Self {
            config: config.clone(), host: Some(host), audio_error: String::new(),
            nodes: Vec::new(), connections: Vec::new(), selected_node: None,
            dragging_node: None, drag_start_offset: egui::Vec2::ZERO,
            graph_offset: egui::Vec2::ZERO, peak_meter: 0.0, voice_count: 0,
            mode: "synth".to_string(),
            filter_cutoff: (config.prime_base as f32 / 31.0).clamp(0.0, 1.0),
            filter_resonance: 0.5, filter_mode_idx: 0, reverb_mix: 0.5,
            delay_time: 0.25, delay_feedback: 0.3, env_attack: 0.01,
            env_release: 0.3, waveform_idx: 0, master_gain: 0.8,
        }
    }
    fn add_node(&mut self, node_type: NodeType, pos: egui::Pos2) {
        let id = self.nodes.len();
        self.nodes.push(VisualNode { id, node_type: node_type.clone(), pos: egui::Vec2::new(pos.x, pos.y), params: NodeParams::default() });
        self.sync_node_to_engine(id, &node_type);
    }
    fn sync_node_to_engine(&self, id: usize, node_type: &NodeType) {
        if let Some(host) = self.host.as_ref() {
            let params = self.nodes.iter().find(|n| n.id == id).map(|n| &n.params);
            let kind = if let Some(p) = params { node_type.to_graph_kind(p) } else { node_type.to_graph_kind(&NodeParams::default()) };
            let _ = host.command_sender().send(EngineCommand::GraphAddNode { id, kind });
        }
    }
    fn remove_node(&mut self, id: usize) {
        self.nodes.retain(|n| n.id != id);
        self.connections.retain(|c| c.from != id && c.to != id);
        self.selected_node = None;
        if let Some(host) = self.host.as_ref() { let _ = host.command_sender().send(EngineCommand::GraphRemoveNode { id }); }
    }
    fn add_connection(&mut self, from: usize, to: usize) {
        if from == to || !self.nodes.iter().any(|n| n.id == from) || !self.nodes.iter().any(|n| n.id == to) || self.connections.iter().any(|c| c.from == from && c.to == to) { return; }
        self.connections.push(Connection { from, to });
        if let Some(host) = self.host.as_ref() { let _ = host.command_sender().send(EngineCommand::GraphConnect { from, to }); }
    }
    fn clear_graph(&mut self) {
        self.nodes.clear(); self.connections.clear(); self.selected_node = None;
        if let Some(host) = self.host.as_ref() { let _ = host.command_sender().send(EngineCommand::GraphClear); }
    }
    fn update_audio_stats(&mut self) {
        if let Some(host) = self.host.as_ref() { if let Ok(eng) = host.engine().lock() { self.peak_meter = eng.output_peak; self.voice_count = eng.pool.active_count(); self.mode = eng.config.mode.clone(); } }
    }
    fn push_config_to_engine(&self) {
        if let Some(host) = self.host.as_ref() { let tx = host.command_sender(); let _ = tx.send(EngineCommand::SetPrimeBase(self.config.prime_base)); let _ = tx.send(EngineCommand::SetPrimePhase(self.config.prime_phase)); let _ = tx.send(EngineCommand::SetNnSensitivity(self.config.nn_sensitivity)); let _ = tx.send(EngineCommand::SetInstanceDensity(self.config.instance_density)); let _ = tx.send(EngineCommand::SetInstanceSpread(self.config.instance_spread)); let mode = match self.config.mode.as_str() { "sampler" => EngineMode::Sampler, "effect" => EngineMode::Effect, "controller" => EngineMode::Controller, _ => EngineMode::Synth }; let _ = tx.send(EngineCommand::SetMode(mode)); }
    }
    fn push_global_params(&mut self) {
        if let Some(host) = self.host.as_ref() { let tx = host.command_sender(); let _ = tx.send(EngineCommand::SetParam(Param::FilterCutoff, self.filter_cutoff)); let _ = tx.send(EngineCommand::SetParam(Param::FilterResonance, self.filter_resonance)); let _ = tx.send(EngineCommand::SetParam(Param::FilterMode, self.filter_mode_idx as f32)); let _ = tx.send(EngineCommand::SetParam(Param::ReverbMix, self.reverb_mix)); let _ = tx.send(EngineCommand::SetParam(Param::DelayTime, self.delay_time)); let _ = tx.send(EngineCommand::SetParam(Param::DelayFeedback, self.delay_feedback)); let _ = tx.send(EngineCommand::SetParam(Param::EnvAttack, self.env_attack)); let _ = tx.send(EngineCommand::SetParam(Param::EnvRelease, self.env_release)); let _ = tx.send(EngineCommand::SetParam(Param::Waveform, self.waveform_idx as f32)); let _ = tx.send(EngineCommand::SetParam(Param::MasterGain, self.master_gain)); }
    }
    fn node_screen_rect(&self, node_pos: egui::Vec2) -> egui::Rect { egui::Rect::from_min_size(egui::Pos2::new(node_pos.x, node_pos.y) + self.graph_offset, egui::vec2(180.0, 65.0)) }
}

impl eframe::App for StandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_audio_stats();

        egui::SidePanel::left("palette").min_width(220.0).show(ctx, |ui| {
            ui.heading("Node Palette"); ui.separator();
            if ui.button("🗑 Clear All").clicked() { self.clear_graph(); }
            ui.separator(); ui.label("Sources:");
            for (nt, icon) in [(NodeType::Oscillator, "～"), (NodeType::Noise, "≋"), (NodeType::Sampler, "▶"), (NodeType::Input, "📥")] { if ui.button(format!("{} {}", icon, nt.name())).clicked() { self.add_node(nt, egui::Pos2::new(400.0, 300.0)); } }
            ui.label("Filters:");
            for (nt, icon) in [(NodeType::Filter, "⟑"), (NodeType::Lfo, "∿")] { if ui.button(format!("{} {}", icon, nt.name())).clicked() { self.add_node(nt, egui::Pos2::new(400.0, 300.0)); } }
            ui.label("Effects:");
            for (nt, icon) in [(NodeType::Envelope, "⌒"), (NodeType::Delay, "◷"), (NodeType::Reverb, "✱"), (NodeType::Distortion, "⚡"), (NodeType::StereoDelay, "⫴")] { if ui.button(format!("{} {}", icon, nt.name())).clicked() { self.add_node(nt, egui::Pos2::new(400.0, 300.0)); } }
            ui.label("Utility:");
            for (nt, icon) in [(NodeType::Gain, "⊕"), (NodeType::Mixer, "⧉"), (NodeType::Splitter, "⫴"), (NodeType::Output, "📤")] { if ui.button(format!("{} {}", icon, nt.name())).clicked() { self.add_node(nt, egui::Pos2::new(400.0, 300.0)); } }
            ui.separator(); ui.label("Quick Templates:");
            if ui.button("Synth Chain").clicked() { self.add_node(NodeType::Oscillator, egui::Pos2::new(200.0, 300.0)); self.add_node(NodeType::Filter, egui::Pos2::new(400.0, 300.0)); self.add_node(NodeType::Envelope, egui::Pos2::new(400.0, 450.0)); self.add_node(NodeType::Output, egui::Pos2::new(600.0, 300.0)); self.add_connection(0, 1); self.add_connection(1, 3); self.add_connection(2, 3); }
            if ui.button("Delay Effect").clicked() { self.add_node(NodeType::Input, egui::Pos2::new(200.0, 300.0)); self.add_node(NodeType::Delay, egui::Pos2::new(400.0, 300.0)); self.add_node(NodeType::Reverb, egui::Pos2::new(600.0, 300.0)); self.add_node(NodeType::Output, egui::Pos2::new(800.0, 300.0)); self.add_connection(0, 1); self.add_connection(1, 2); self.add_connection(2, 3); }
        });

        egui::SidePanel::right("params").min_width(280.0).show(ctx, |ui| {
            ui.heading("Node Parameters"); ui.separator();
            if let Some(node_id) = self.selected_node {
                if let Some(node) = self.nodes.iter().find(|n| n.id == node_id) { ui.label(format!("{} #{}", node.node_type.name(), node.id)); ui.separator(); }
                let mut changed = false;
                if let Some(n) = self.nodes.iter_mut().find(|n| n.id == node_id) {
                    match &n.node_type {
                        NodeType::Oscillator => { let mut freq = n.params.frequency; ui.add(egui::Slider::new(&mut freq, 20.0..=20000.0).logarithmic(true).text("Frequency")); if freq != n.params.frequency { changed = true; } n.params.frequency = freq; let mut wi = match n.params.waveform { Waveform::Sine => 0, Waveform::Saw => 1, Waveform::Square => 2, Waveform::Triangle => 3 }; ui.label("Waveform:"); egui::ComboBox::from_id_source("node_waveform").selected_text(["Sine", "Saw", "Square", "Triangle"][wi]).show_ui(ui, |ui| { for (i, name) in ["Sine", "Saw", "Square", "Triangle"].iter().enumerate() { ui.selectable_value(&mut wi, i, *name); } }); n.params.waveform = match wi { 1 => Waveform::Saw, 2 => Waveform::Square, 3 => Waveform::Triangle, _ => Waveform::Sine }; }
                        NodeType::Filter => { let mut cutoff = n.params.cutoff; ui.add(egui::Slider::new(&mut cutoff, 20.0..=20000.0).logarithmic(true).text("Cutoff")); if cutoff != n.params.cutoff { changed = true; } n.params.cutoff = cutoff; let mut resonance = n.params.resonance; ui.add(egui::Slider::new(&mut resonance, 0.1..=5.0).text("Resonance")); if resonance != n.params.resonance { changed = true; } n.params.resonance = resonance; let mut mi = match n.params.filter_mode { FilterMode::LowPass => 0, FilterMode::HighPass => 1, FilterMode::BandPass => 2, FilterMode::Notch => 3 }; ui.label("Mode:"); egui::ComboBox::from_id_source("filter_mode").selected_text(["LP", "HP", "BP", "Notch"][mi]).show_ui(ui, |ui| { for (i, name) in ["LP", "HP", "BP", "Notch"].iter().enumerate() { ui.selectable_value(&mut mi, i, *name); } }); n.params.filter_mode = match mi { 1 => FilterMode::HighPass, 2 => FilterMode::BandPass, 3 => FilterMode::Notch, _ => FilterMode::LowPass }; }
                        NodeType::Envelope => { let mut a = n.params.attack; ui.add(egui::Slider::new(&mut a, 0.001..=2.0).text("Attack")); if a != n.params.attack { changed = true; } n.params.attack = a; let mut d = n.params.decay; ui.add(egui::Slider::new(&mut d, 0.001..=2.0).text("Decay")); if d != n.params.decay { changed = true; } n.params.decay = d; let mut s = n.params.sustain; ui.add(egui::Slider::new(&mut s, 0.0..=1.0).text("Sustain")); if s != n.params.sustain { changed = true; } n.params.sustain = s; let mut r = n.params.release; ui.add(egui::Slider::new(&mut r, 0.001..=5.0).text("Release")); if r != n.params.release { changed = true; } n.params.release = r; }
                        NodeType::Delay => { let mut t = n.params.delay_time; ui.add(egui::Slider::new(&mut t, 0.001..=1.0).text("Time (s)")); if t != n.params.delay_time { changed = true; } n.params.delay_time = t; let mut f = n.params.delay_feedback; ui.add(egui::Slider::new(&mut f, 0.0..=0.95).text("Feedback")); if f != n.params.delay_feedback { changed = true; } n.params.delay_feedback = f; }
                        NodeType::Reverb => { let mut r = n.params.reverb_room; ui.add(egui::Slider::new(&mut r, 0.0..=1.0).text("Room Size")); if r != n.params.reverb_room { changed = true; } n.params.reverb_room = r; let mut d = n.params.reverb_damping; ui.add(egui::Slider::new(&mut d, 0.0..=1.0).text("Damping")); if d != n.params.reverb_damping { changed = true; } n.params.reverb_damping = d; }
                        NodeType::Gain => { let mut g = n.params.gain; ui.add(egui::Slider::new(&mut g, 0.0..=2.0).text("Gain")); if g != n.params.gain { changed = true; } n.params.gain = g; }
                        NodeType::Distortion => { let mut d = n.params.drive; ui.add(egui::Slider::new(&mut d, 1.0..=10.0).text("Drive")); if d != n.params.drive { changed = true; } n.params.drive = d; let mut t = n.params.tone; ui.add(egui::Slider::new(&mut t, 0.0..=1.0).text("Tone")); if t != n.params.tone { changed = true; } n.params.tone = t; }
                        _ => {}
                    }
                    if changed { let nt = n.node_type.clone(); self.sync_node_to_engine(node_id, &nt); }
                }
                ui.separator(); if ui.button("🗑 Delete Node").clicked() { self.remove_node(node_id); }
            } else { ui.label("Select a node"); ui.label("to edit parameters"); }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let response = ui.interact(rect, egui::Id::new("graph"), egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Middle) { self.graph_offset += response.drag_delta(); }
            if response.drag_released() && self.dragging_node.is_some() { self.dragging_node = None; }
            let click_pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            if response.clicked() {
                self.selected_node = None;
                for node in self.nodes.iter().rev() { let node_rect = self.node_screen_rect(node.pos); if node_rect.contains(click_pos) { self.selected_node = Some(node.id); break; } }
            }
            if self.selected_node.is_some() && response.dragged() && self.dragging_node.is_none() {
                if let Some(node) = self.nodes.iter().find(|n| n.id == self.selected_node.unwrap()) { let np = egui::Vec2::new(node.pos.x, node.pos.y); let ov = click_pos - np - self.graph_offset; self.drag_start_offset = egui::Vec2::new(ov.x, ov.y); self.dragging_node = self.selected_node; }
            }
            if let Some(drag_id) = self.dragging_node { if let Some(node) = self.nodes.iter_mut().find(|n| n.id == drag_id) { let np = click_pos - self.drag_start_offset - self.graph_offset; node.pos = egui::Vec2::new(np.x, np.y); } }
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 40));
            for conn in &self.connections { if let (Some(from_node), Some(to_node)) = (self.nodes.iter().find(|n| n.id == conn.from), self.nodes.iter().find(|n| n.id == conn.to)) { let fp = egui::Pos2::new(from_node.pos.x + 100.0, from_node.pos.y + 32.5) + self.graph_offset; let tp = egui::Pos2::new(to_node.pos.x, to_node.pos.y + 32.5) + self.graph_offset; painter.line_segment([fp, tp], egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(100, 200, 255))); } }
            for node in &self.nodes { let node_rect = self.node_screen_rect(node.pos); let selected = self.selected_node == Some(node.id); let color = node.node_type.color(); let stroke = if selected { egui::Stroke::new(2.5_f32, egui::Color32::WHITE) } else { egui::Stroke::new(1.5_f32, color) }; painter.rect(node_rect.expand(6.0), 8.0, color.gamma_multiply(0.3), stroke); painter.rect(node_rect, 8.0, color.gamma_multiply(0.6), egui::Stroke::NONE); let lp = egui::Pos2::new(node_rect.min.x + 8.0, node_rect.min.y + 12.0); painter.text(lp, egui::Align2::LEFT_TOP, node.node_type.name(), egui::FontId::proportional(13.0), egui::Color32::WHITE); let ip = egui::Pos2::new(node_rect.min.x + 8.0, node_rect.min.y + 30.0); painter.text(ip, egui::Align2::LEFT_TOP, format!("#{}", node.id), egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY); }
        });

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Modurust-Lite"); ui.separator();
                if ui.button("▶ Start Audio").clicked() { if let Some(host) = self.host.as_mut() { if host.start().is_err() { self.audio_error = "Failed to start audio".to_string(); } } }
                if !self.audio_error.is_empty() { ui.colored_label(egui::Color32::RED, &self.audio_error); }
                ui.separator(); ui.label(format!("Mode: {}", self.mode)); ui.separator(); ui.label(format!("Voices: {}", self.voice_count));
                let peak_color = if self.peak_meter > 0.9 { egui::Color32::RED } else if self.peak_meter > 0.7 { egui::Color32::YELLOW } else { egui::Color32::GREEN };
                ui.add(egui::ProgressBar::new(self.peak_meter.min(1.0)).fill(peak_color).text(format!("Peak: {:.0}%", self.peak_meter * 100.0)));
            });
        });

        self.push_global_params();

        egui::TopBottomPanel::bottom("bottom").max_height(260.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("💾 Save config.json").clicked() { if let Err(e) = self.config.save_to_file(CONFIG_PATH) { self.audio_error = format!("Failed to save config: {e}"); } else { self.audio_error = String::new(); } }
                if ui.button("📂 Load config").clicked() { self.config = EngineConfig::load_from_file(CONFIG_PATH); self.push_config_to_engine(); }
                ui.separator();
                ui.label("Mode:");
                egui::ComboBox::from_id_source("mode").selected_text(&self.config.mode).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.mode, "synth".to_string(), "Synth");
                    ui.selectable_value(&mut self.config.mode, "sampler".to_string(), "Sampler");
                    ui.selectable_value(&mut self.config.mode, "effect".to_string(), "Effect");
                    ui.selectable_value(&mut self.config.mode, "controller".to_string(), "Controller");
                });
                self.push_config_to_engine();
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("XY Morph Pad:");
                let (rect, response) = ui.allocate_exact_size(egui::vec2(300.0, 300.0), egui::Sense::drag());
                if response.dragged() || response.clicked() { if let Some(pos) = response.interact_pointer_pos() { let local = pos - rect.min; self.config.prime_phase = (local.x / rect.width()).clamp(0.0, 1.0); self.config.nn_sensitivity = (local.y / rect.height()).clamp(0.0, 1.0); } }
                ui.painter().rect_filled(rect, 8.0, egui::Color32::from_rgb(30, 30, 40));
                let kx = rect.min.x + self.config.prime_phase * rect.width();
                let ky = rect.min.y + self.config.nn_sensitivity * rect.height();
                ui.painter().circle_filled(egui::pos2(kx, ky), 10.0, egui::Color32::from_rgb(0, 200, 255));
                ui.painter().text(egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0), egui::Align2::LEFT_TOP, format!("phase: {:.2}, sens: {:.2}", self.config.prime_phase, self.config.nn_sensitivity), egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Filter Cutoff:"); ui.add(egui::Slider::new(&mut self.filter_cutoff, 0.0..=1.0).text(""));
                ui.label("Resonance:"); ui.add(egui::Slider::new(&mut self.filter_resonance, 0.0..=1.0).text(""));
                ui.label("Mode:"); egui::ComboBox::from_id_source("fmode").selected_text(["LP", "HP", "BP", "Notch"][self.filter_mode_idx as usize]).show_ui(ui, |ui| { for (i, name) in ["LP", "HP", "BP", "Notch"].iter().enumerate() { ui.selectable_value(&mut self.filter_mode_idx, i as i32, *name); } });
            });
            ui.horizontal(|ui| {
                ui.label("Reverb Mix:"); ui.add(egui::Slider::new(&mut self.reverb_mix, 0.0..=1.0).text(""));
                ui.label("Delay Time:"); ui.add(egui::Slider::new(&mut self.delay_time, 0.0..=1.0).text(""));
                ui.label("Delay FB:"); ui.add(egui::Slider::new(&mut self.delay_feedback, 0.0..=0.95).text(""));
            });
            ui.horizontal(|ui| {
                ui.label("Env Attack:"); ui.add(egui::Slider::new(&mut self.env_attack, 0.001..=2.0).text(""));
                ui.label("Env Release:"); ui.add(egui::Slider::new(&mut self.env_release, 0.001..=5.0).text(""));
                ui.label("Waveform:"); egui::ComboBox::from_id_source("waveform").selected_text(["Sine", "Saw", "Square", "Triangle"][self.waveform_idx as usize]).show_ui(ui, |ui| { for (i, name) in ["Sine", "Saw", "Square", "Triangle"].iter().enumerate() { ui.selectable_value(&mut self.waveform_idx, i as i32, *name); } });
                ui.label("Master Gain:"); ui.add(egui::Slider::new(&mut self.master_gain, 0.0..=1.0).text(""));
            });
        });

        egui::TopBottomPanel::bottom("piano").max_height(100.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Notes:");
                for note in [36u8, 40, 43, 48, 52, 55, 60, 64, 67, 72, 76, 79, 84] {
                    let name = match note % 12 { 0 => "C", 1 => "C#", 2 => "D", 3 => "D#", 4 => "E", 5 => "F", 6 => "F#", 7 => "G", 8 => "G#", 9 => "A", 10 => "A#", 11 => "B", _ => "?" };
                    let oct = note / 12 - 1;
                    if ui.button(format!("{}{}", name, oct)).clicked() { if let Some(host) = self.host.as_ref() { let _ = host.command_sender().send(EngineCommand::NoteOn { note, velocity: 0.8 }); } }
                }
                if ui.button("🔇 All Off").clicked() { if let Some(host) = self.host.as_ref() { let _ = host.command_sender().send(EngineCommand::AllNotesOff); } }
            });
        });
    }
}