use eframe::egui;
use modurust_core::config::EngineConfig;
use modurust_core::dsp::graph::GraphNodeKind;
use modurust_core::dsp::FilterMode;
use modurust_core::engine::{EngineCommand, EngineMode};
use modurust_core::io::audio::CpalEngineHost;
use std::sync::Arc;

const CONFIG_PATH: &str = "config.json";
const SAMPLE_RATE: f32 = 44100.0;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([820.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Modurust-Lite Standalone",
        options,
        Box::new(|_cc| Box::new(StandaloneApp::new())),
    )
}

struct StandaloneApp {
    config: EngineConfig,
    host: Option<CpalEngineHost>,
    audio_error: String,
    // Graph editor state
    graph_nodes: Vec<(usize, String, String)>,
    graph_connections: Vec<(usize, usize)>,
    selected_node: Option<usize>,
    connect_from: Option<usize>,
    pending_add_kind: String,
}

impl StandaloneApp {
    fn new() -> Self {
        // Load root config.json (hot-reloadable) with fallback to defaults.
        let config = EngineConfig::load_from_file(CONFIG_PATH);

        // Create the audio engine host (not started until the user clicks Start).
        let host = CpalEngineHost::new(config.clone(), SAMPLE_RATE);

        // Start UDP control listeners so M4L bridges can drive the engine.
        let bridge = modurust_core::comm::UdpOscBridge::new_with_status(
            config.udp_port,
            config.osc_port,
            config.status_port,
        );
        let command_tx = host.command_sender();
        bridge.start_udp_listener(command_tx.clone(), Arc::new(std::sync::atomic::AtomicBool::new(true)));
        bridge.start_osc_listener(command_tx.clone(), Arc::new(std::sync::atomic::AtomicBool::new(true)));

        // Broadcast engine status to M4L devices listening on status port.
        {
            let engine = host.engine();
            bridge.start_status_broadcast(
                move || modurust_core::comm::UdpOscBridge::status_from_engine(&engine),
                50,
                Arc::new(std::sync::atomic::AtomicBool::new(true)),
            );
        }

        Self {
            config,
            host: Some(host),
            audio_error: String::new(),
            graph_nodes: Vec::new(),
            graph_connections: Vec::new(),
            selected_node: None,
            connect_from: None,
            pending_add_kind: "oscillator".to_string(),
        }
    }

    fn push_config_to_engine(&self) {
        if let Some(host) = self.host.as_ref() {
            let tx = host.command_sender();
            let _ = tx.send(EngineCommand::SetPrimeBase(self.config.prime_base));
            let _ = tx.send(EngineCommand::SetPrimePhase(self.config.prime_phase));
            let _ = tx.send(EngineCommand::SetNnSensitivity(self.config.nn_sensitivity));
            let _ = tx.send(EngineCommand::SetInstanceDensity(self.config.instance_density));
            let _ = tx.send(EngineCommand::SetInstanceSpread(self.config.instance_spread));
            let mode = match self.config.mode.as_str() {
                "sampler" => EngineMode::Sampler,
                "effect" => EngineMode::Effect,
                "controller" => EngineMode::Controller,
                _ => EngineMode::Synth,
            };
            let _ = tx.send(EngineCommand::SetMode(mode));
        }
    }

    fn start_audio(&mut self) {
        if let Some(host) = self.host.as_mut() {
            if host.start().is_ok() {
                self.audio_error.clear();
            } else {
                self.audio_error = "Audio start failed".to_string();
            }
        }
    }

    fn send_note(&self, note: u8, on: bool) {
        if let Some(host) = self.host.as_ref() {
            let _ = host.command_sender().send(if on {
                EngineCommand::NoteOn { note, velocity: 0.9 }
            } else {
                EngineCommand::NoteOff { note }
            });
        }
    }

    fn add_graph_node(&mut self) {
        let id = self.graph_nodes.len();
        let (kind, name, params) = match self.pending_add_kind.as_str() {
            "oscillator" => {
                (
                    GraphNodeKind::Oscillator(
                        modurust_core::dsp::Waveform::Sine,
                        440.0,
                    ),
                    "oscillator".to_string(),
                    "440 Hz".to_string(),
                )
            }
            "filter" => {
                (
                    GraphNodeKind::Filter(FilterMode::LowPass, 1000.0, 0.7),
                    "filter".to_string(),
                    "LP 1k Q=0.7".to_string(),
                )
            }
            _ => return,
        };

        self.graph_nodes.push((id, name, params));
        let _ = self
            .host
            .as_ref()
            .unwrap()
            .command_sender()
            .send(EngineCommand::GraphAddNode {
                id,
                kind,
            });
    }

    fn remove_graph_node(&mut self, id: usize) {
        self.graph_nodes.retain(|(i, _, _)| *i != id);
        self.graph_connections.retain(|(f, t)| *f != id && *t != id);
        if self.selected_node == Some(id) {
            self.selected_node = None;
        }
    }

    fn connect_graph(&mut self, from: usize, to: usize) {
        self.graph_connections.push((from, to));
        let _ = self
            .host
            .as_ref()
            .unwrap()
            .command_sender()
            .send(EngineCommand::GraphConnect { from, to });
    }

    fn disconnect_graph(&mut self, from: usize, to: usize) {
        self.graph_connections.retain(|(f, t)| !(f == &from && t == &to));
        let _ = self
            .host
            .as_ref()
            .unwrap()
            .command_sender()
            .send(EngineCommand::GraphDisconnect { from, to });
    }

    fn graph_editor_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label("DSP Graph Editor");
            ui.separator();

            // Add node
            ui.horizontal(|ui| {
                ui.label("Add:");
                egui::ComboBox::from_id_source("node_kind")
                    .selected_text(&self.pending_add_kind)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.pending_add_kind, "oscillator".to_string(), "Oscillator");
                        ui.selectable_value(&mut self.pending_add_kind, "filter".to_string(), "Filter");
                    });
                if ui.button("➕ Add").clicked() {
                    self.add_graph_node();
                }
            });

            // Nodes list
            ui.separator();
            ui.label("Nodes:");
            let mut to_remove: Option<usize> = None;
            let mut clicked_node: Option<usize> = None;
            for (id, name, params) in &self.graph_nodes {
                ui.horizontal(|ui| {
                    let label = format!("#{}: {} ({})", id, name, params);
                    if ui.selectable_label(self.selected_node == Some(*id), &label).clicked() {
                        clicked_node = Some(*id);
                    }
                    if ui.button("❌").clicked() {
                        to_remove = Some(*id);
                    }
                });
            }
            if let Some(id) = to_remove {
                self.remove_graph_node(id);
            }
            if let Some(id) = clicked_node {
                // If we're in connect mode, complete the connection
                if let Some(from) = self.connect_from {
                    self.connect_graph(from, id);
                    self.connect_from = None;
                } else {
                    self.selected_node = Some(id);
                }
            }

            // Connections
            ui.separator();
            ui.label("Connections:");
            let mut to_disconnect: Option<(usize, usize)> = None;
            for (from, to) in &self.graph_connections {
                ui.horizontal(|ui| {
                    ui.label(format!("#{} → #{}", from, to));
                    if ui.button("❌").clicked() {
                        to_disconnect = Some((*from, *to));
                    }
                });
            }
            if let Some((from, to)) = to_disconnect {
                self.disconnect_graph(from, to);
            }

            // Connect mode
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(from) = self.connect_from {
                    ui.label(format!("Connecting from #{} — click a node to connect to", from));
                    if ui.button("Cancel").clicked() {
                        self.connect_from = None;
                    }
                } else {
                    if ui.button("🔗 Connect").clicked() {
                        // Enter connect mode: wait for next node selection
                        if let Some(selected) = self.selected_node {
                            self.connect_from = Some(selected);
                        }
                    }
                }
                if ui.button("🗑 Clear Graph").clicked() {
                    self.graph_nodes.clear();
                    self.graph_connections.clear();
                    self.selected_node = None;
                    self.connect_from = None;
                    if let Some(host) = self.host.as_ref() {
                        let _ = host.command_sender().send(EngineCommand::GraphClear);
                    }
                }
            });
        });
    }
}

impl eframe::App for StandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Modurust-Lite Standalone - Working Build");

            ui.horizontal(|ui| {
                ui.label(format!("Config: {}", CONFIG_PATH));
                if ui.button("💾 Save config.json").clicked() {
                    // Save current config to file
                    if let Err(e) = self.config.save_to_file(CONFIG_PATH) {
                        self.audio_error = format!("Failed to save config: {e}");
                    } else {
                        self.audio_error = String::new();
                    }
                }
            });
            ui.separator();

            // Mode selection
            ui.horizontal(|ui| {
                ui.label("Operating Mode:");
                egui::ComboBox::from_label("")
                    .selected_text(&self.config.mode)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.config.mode, "synth".to_string(), "Synth");
                        ui.selectable_value(&mut self.config.mode, "sampler".to_string(), "Sampler");
                        ui.selectable_value(&mut self.config.mode, "effect".to_string(), "Effect");
                        ui.selectable_value(&mut self.config.mode, "controller".to_string(), "Controller");
                    });
            });
            ui.separator();

            if ui.button("▶ Start Audio").clicked() {
                self.start_audio();
            }
            if !self.audio_error.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(255, 80, 80), &self.audio_error);
            }
            ui.add_space(10.0);

            // Test notes
            ui.group(|ui| {
                ui.label("Test Notes (drives the voice pool)");
                ui.horizontal(|ui| {
                    for note in [48u8, 52, 55, 60, 64, 67, 72] {
                        let label = format!("♪ {note}");
                        if ui.button(label).clicked() {
                            self.send_note(note, true);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("All Notes Off").clicked() {
                        if let Some(host) = self.host.as_ref() {
                            let _ = host.command_sender().send(EngineCommand::AllNotesOff);
                        }
                    }
                });
            });
            ui.add_space(10.0);

            // Prime & NN controls
            ui.group(|ui| {
                ui.label("Prime & NN Controls");
                ui.horizontal(|ui| {
                    // Prime Base +1 button
                    if ui.button("Prime Base +1").clicked() {
                        if let Some(host) = self.host.as_ref() {
                            let _ = host.command_sender().send(EngineCommand::SetPrimeBase(
                                self.config.prime_base.saturating_add(1)
                            ));
                        }
                    }
                    // Sliders (just bind to config; pushed at end of group)
                    ui.add(egui::Slider::new(&mut self.config.prime_base, 2..=31)
                        .text("Prime Base"));
                    ui.add(egui::Slider::new(&mut self.config.prime_phase, 0.0..=1.0)
                        .text("Prime Phase"));
                    ui.add(egui::Slider::new(&mut self.config.nn_sensitivity, 0.0..=1.0)
                        .text("NN Sensitivity"));
                    ui.add(egui::Slider::new(&mut self.config.instance_density, 1..=64)
                        .text("Instance Density"));
                    ui.add(egui::Slider::new(&mut self.config.instance_spread, 0.0..=1.0)
                        .text("Instance Spread"));
                });
                // Push config changes to engine every frame
                self.push_config_to_engine();
            });
            ui.add_space(10.0);

            // XY morph pad
            ui.group(|ui| {
                ui.label("NN Latent Space Morph Pad (XY)");
                let (rect, response) = ui.allocate_exact_size(egui::vec2(200.0, 200.0), egui::Sense::drag());
                if response.dragged() {
                    if let Some(_pos) = response.interact_pointer_pos() {
                        // Map XY to controls - just print for now
                    }
                }
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 30, 40));

                // Draw knob
                let knob_pos = egui::pos2(
                    rect.min.x + self.config.prime_phase * rect.width(),
                    rect.min.y + self.config.nn_sensitivity * rect.height(),
                );
                ui.painter().circle_filled(knob_pos, 8.0, egui::Color32::from_rgb(0, 200, 255));
            });
            ui.add_space(10.0);

            // Graph editor
            ui.group(|ui| {
                ui.label("DSP Graph Editor");
                self.graph_editor_ui(ui);
            });
        });
    }
}