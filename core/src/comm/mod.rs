use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use rosc::{OscPacket, OscMessage};
use crate::engine::EngineCommand;

/// Shared engine status snapshot broadcast to M4L clients over UDP.
#[derive(Debug, Clone, Copy)]
pub struct EngineStatus {
    pub mode: u8,          // 0=Synth, 1=Sampler, 2=Effect, 3=Controller
    pub output_peak: f32,  // current output peak level (0..=1)
    pub prime_base: u32,
    pub prime_phase: f32,
    pub nn_sensitivity: f32,
    pub instance_density: u32,
    pub instance_spread: f32,
    pub active_voices: usize,
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self {
            mode: 0,
            output_peak: 0.0,
            prime_base: 2,
            prime_phase: 0.0,
            nn_sensitivity: 0.5,
            instance_density: 16,
            instance_spread: 0.5,
            active_voices: 0,
        }
    }
}

/// Human-readable mode name for a numeric status mode (`mode_name(0)` == "synth").
pub fn mode_name(mode: u8) -> &'static str {
    match mode {
        1 => "sampler",
        2 => "effect",
        3 => "controller",
        _ => "synth",
    }
}

/// Binary status packet layout (sent on the UDP status port):
///   [0]  = magic 0x4D ('M')
///   [1]  = version 0x01
///   [2]  = mode (u8)
///   [3..7]  = output_peak (f32 LE)
///   [7..11] = prime_base (u32 LE)
///   [11..15]= prime_phase (f32 LE)
///   [15..19]= nn_sensitivity (f32 LE)
///   [19..23]= instance_density (u32 LE)
///   [23..27]= instance_spread (f32 LE)
///   [27..31]= active_voices (u32 LE)
pub fn encode_status_packet(status: &EngineStatus) -> [u8; 31] {
    let mut p = [0u8; 31];
    p[0] = 0x4D; // 'M'
    p[1] = 0x01; // version
    p[2] = status.mode;
    p[3..7].copy_from_slice(&status.output_peak.to_le_bytes());
    p[7..11].copy_from_slice(&status.prime_base.to_le_bytes());
    p[11..15].copy_from_slice(&status.prime_phase.to_le_bytes());
    p[15..19].copy_from_slice(&status.nn_sensitivity.to_le_bytes());
    p[19..23].copy_from_slice(&status.instance_density.to_le_bytes());
    p[23..27].copy_from_slice(&status.instance_spread.to_le_bytes());
    p[27..31].copy_from_slice(&(status.active_voices as u32).to_le_bytes());
    p
}

pub struct UdpOscBridge {
    pub udp_port: u16,
    pub osc_port: u16,
    pub status_port: u16,
}

impl UdpOscBridge {
    pub fn new(udp_port: u16, osc_port: u16) -> Self {
        Self { udp_port, osc_port, status_port: 9002 }
    }

    pub fn new_with_status(udp_port: u16, osc_port: u16, status_port: u16) -> Self {
        Self { udp_port, osc_port, status_port }
    }

    /// Broadcast engine status to `127.0.0.1` at a fixed rate.
    ///
    /// - **Text port** (`status_port`, default 9002): each parameter is sent as
    ///   its own short labeled datagram (`mode synth`, `peak 0.5`, `prime_base
    ///   5`, ...). Max for Live's `udpreceive` + `route` can match those labels
    ///   directly, which the raw binary blob cannot do cleanly.
    /// - **Binary ports** (`status_port+1 .. status_port+3`, i.e. 9003-9005):
    ///   the compact 31-byte status packet, for the Rust M4L bridge demos.
    ///
    /// The Max device and Rust bridges must not share a port: Windows UDP
    /// permits only one bind per port, so the text and binary feeds are kept
    /// on disjoint ports.
    pub fn start_status_broadcast<S>(
        &self,
        status_provider: S,
        interval_ms: u64,
        running: Arc<AtomicBool>,
    ) where
        S: Fn() -> EngineStatus + Send + 'static,
    {
        let text_ports: Vec<u16> = (0..1).map(|i| self.status_port + i).collect();
        let binary_ports: Vec<u16> = (1..4).map(|i| self.status_port + i).collect();
        std::thread::spawn(move || {
            let Ok(socket) = UdpSocket::bind("127.0.0.1:0") else { return };
            while running.load(Ordering::Relaxed) {
                let status = status_provider();
                let packet = encode_status_packet(&status);
                // One datagram per labeled value so Max `route` can match each
                // label and forward just the single value that follows it.
                let text_packets: [Vec<u8>; 8] = [
                    format!("mode {}", mode_name(status.mode)).into_bytes(),
                    format!("peak {:.5}", status.output_peak).into_bytes(),
                    format!("prime_base {}", status.prime_base).into_bytes(),
                    format!("phase {:.5}", status.prime_phase).into_bytes(),
                    format!("sens {:.5}", status.nn_sensitivity).into_bytes(),
                    format!("density {}", status.instance_density).into_bytes(),
                    format!("spread {:.5}", status.instance_spread).into_bytes(),
                    format!("voices {}", status.active_voices).into_bytes(),
                ];
                for port in &binary_ports {
                    let _ = socket.send_to(&packet, ("127.0.0.1", *port));
                }
                for port in &text_ports {
                    for p in &text_packets {
                        let _ = socket.send_to(p, ("127.0.0.1", *port));
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            }
        });
    }

    /// Convenience status provider that reads from a shared `Arc<Mutex<CoreEngine>>`.
    pub fn status_from_engine(engine: &Arc<Mutex<crate::engine::CoreEngine>>) -> EngineStatus {
        if let Ok(eng) = engine.lock() {
            EngineStatus {
                mode: match eng.config.mode.as_str() {
                    "sampler" => 1,
                    "effect" => 2,
                    "controller" => 3,
                    _ => 0,
                },
                output_peak: eng.output_peak,
                prime_base: eng.config.prime_base,
                prime_phase: eng.config.prime_phase,
                nn_sensitivity: eng.config.nn_sensitivity,
                instance_density: eng.config.instance_density,
                instance_spread: eng.config.instance_spread,
                active_voices: eng.pool.active_count(),
            }
        } else {
            EngineStatus::default()
        }
    }

    pub fn start_udp_listener(&self, cmd_tx: Sender<EngineCommand>, running: Arc<AtomicBool>) {
        let port = self.udp_port;
        std::thread::spawn(move || {
            if let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) {
                let mut buf = [0u8; 1024];
                while running.load(Ordering::Relaxed) {
                    if let Ok((amt, _src)) = socket.recv_from(&mut buf) {
                        let data = &buf[..amt];
                        // ASCII text protocol: "noteon 60 100", "noteoff 60",
                        // "prime 5", "sens 0.85", "density 32", "spread 0.5",
                        // "mode synth|sampler|effect|controller", "alloff".
                        // This is the primary protocol for Max for Live (udpsend).
                        if Self::try_parse_text_command(data, &cmd_tx) {
                            continue;
                        }
                        // Simple binary protocol: [0: NoteOn, 1: NoteOff, 2: SetPrime, 3: SetSens,
                        //                          4: SetDensity, 5: SetSpread, 6: SetMode, 7: SetPhase]
                        if data.len() >= 3 && data[0] == 0 {
                            let note = data[1];
                            let vel = data[2] as f32 / 127.0;
                            let _ = cmd_tx.send(EngineCommand::NoteOn { note, velocity: vel });
                        } else if data.len() >= 2 && data[0] == 1 {
                            let note = data[1];
                            let _ = cmd_tx.send(EngineCommand::NoteOff { note });
                        } else if data.len() >= 5 && data[0] == 2 {
                            let base = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                            let _ = cmd_tx.send(EngineCommand::SetPrimeBase(base));
                        } else if data.len() >= 5 && data[0] == 3 {
                            let sens = f32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                            let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(sens));
                        } else if data.len() >= 5 && data[0] == 4 {
                            let density = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                            let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(density));
                        } else if data.len() >= 5 && data[0] == 5 {
                            let spread = f32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                            let _ = cmd_tx.send(EngineCommand::SetInstanceSpread(spread));
                        } else if data.len() >= 2 && data[0] == 6 {
                            let mode = data[1];
                            let mode = match mode {
                                1 => crate::engine::EngineMode::Sampler,
                                2 => crate::engine::EngineMode::Effect,
                                3 => crate::engine::EngineMode::Controller,
                                _ => crate::engine::EngineMode::Synth,
                            };
                            let _ = cmd_tx.send(EngineCommand::SetMode(mode));
                        } else if data.len() >= 5 && data[0] == 7 {
                            let phase = f32::from_le_bytes([data[1], data[2], data[3], data[4]]);
                            let _ = cmd_tx.send(EngineCommand::SetPrimePhase(phase));
                        }
                    }
                }
            }
        });
    }

    pub fn start_osc_listener(&self, cmd_tx: Sender<EngineCommand>, running: Arc<AtomicBool>) {
        let port = self.osc_port;
        std::thread::spawn(move || {
            if let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) {
                let mut buf = [0u8; 1024];
                while running.load(Ordering::Relaxed) {
                    if let Ok((amt, _src)) = socket.recv_from(&mut buf) {
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..amt]) {
                            Self::handle_osc_packet(packet, &cmd_tx);
                        }
                    }
                }
            }
        });
    }

    fn handle_osc_packet(packet: OscPacket, cmd_tx: &Sender<EngineCommand>) {
        match packet {
            OscPacket::Message(msg) => Self::handle_osc_msg(msg, cmd_tx),
            OscPacket::Bundle(bundle) => {
                for p in bundle.content {
                    Self::handle_osc_packet(p, cmd_tx);
                }
            }
        }
    }

/// Parse a UTF-8 ASCII text command from a UDP datagram.
    ///
    /// Supported commands (whitespace-separated tokens):
    ///   `noteon <note> <vel>`     — note 0-127, velocity 0-127
    ///   `noteoff <note>`          — release note
    ///   `alloff`                  — release all voices
    ///   `prime <base>`            — set prime base (u32)
    ///   `phase <phase>`           — set prime phase (f32)
    ///   `sens <sensitivity>`      — set NN sensitivity (f32)
    ///   `density <density>`       — set instance density (u32)
    ///   `spread <spread>`         — set instance spread (f32)
    ///   `mode <synth|sampler|effect|controller>` — set operating mode
    ///   `param <name> <value>`    — set a DSP parameter:
    ///       filter_cutoff, filter_resonance, filter_mode,
    ///       reverb_mix, delay_time, delay_feedback,
    ///       env_attack, env_release, waveform, master_gain
    ///       (all values normalized 0..=1 except waveform/filter_mode index)
    ///   `graph_osc <id> <waveform> <freq>`      — add oscillator node (waveform: 0=Sine,1=Saw,2=Square,3=Triangle)
    ///   `graph_filter <id> <mode> <cutoff> <res>` — add filter node (mode: 0=LP,1=HP,2=BP,3=Notch)
    ///   `graph_env <id> <a> <d> <s> <r>`        — add envelope node
    ///   `graph_lfo <id> <freq>`                 — add LFO node
    ///   `graph_delay <id> <time_s> <feedback>`  — add delay node
    ///   `graph_reverb <id> <room> <damp>`       — add reverb node
    ///   `graph_noise <id> <color>`              — add noise node (color: 0=White,1=Pink,2=Brown)
    ///   `graph_gain <id> <gain>`                — add gain node
    ///   `graph_dist <id> <drive> <tone>`        — add distortion node
    ///   `graph_sampler <id>`                    — add sampler node
    ///   `graph_mixer <id> <gain1> <gain2>`      — add mixer node
    ///   `graph_splitter <id>`                   — add splitter node
    ///   `graph_sdelay <id> <t_l> <t_r> <cfb>`   — add stereo delay node
    ///   `graph_output <id>`                     — add output node
    ///   `graph_remove <id>`                     — remove a node
    ///   `graph_connect <from> <to>`             — connect nodes
    ///   `graph_disconnect <from> <to>`          — disconnect nodes
    ///   `graph_param <id> <name> <value>`       — set a graph node parameter
    ///   `graph_trigger <id> <gate>`             — trigger a node (gate: 0=off,1=on)
    ///   `graph_clear`                           — clear the entire graph
    ///
    /// Returns `true` if the datagram was recognized as a text command.
    fn try_parse_text_command(data: &[u8], cmd_tx: &Sender<EngineCommand>) -> bool {
        let Ok(text) = std::str::from_utf8(data) else { return false };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }
        let mut tokens = trimmed.split_whitespace();
        let Some(cmd) = tokens.next() else { return false };
        match cmd {
            "noteon" => {
                let note: u8 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(60);
                let vel: f32 = tokens
                    .next()
                    .and_then(|t| t.parse::<f32>().ok())
                    .map(|v| (v / 127.0).clamp(0.0, 1.0))
                    .unwrap_or(0.8);
                let _ = cmd_tx.send(EngineCommand::NoteOn { note, velocity: vel });
                true
            }
            "noteoff" => {
                let note: u8 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(60);
                let _ = cmd_tx.send(EngineCommand::NoteOff { note });
                true
            }
            "alloff" => {
                let _ = cmd_tx.send(EngineCommand::AllNotesOff);
                true
            }
            "prime" => {
                if let Some(t) = tokens.next() {
                    if let Ok(base) = t.parse::<u32>() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimeBase(base));
                    }
                }
                true
            }
            "phase" => {
                if let Some(t) = tokens.next() {
                    if let Ok(phase) = t.parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimePhase(phase));
                    }
                }
                true
            }
            "sens" => {
                if let Some(t) = tokens.next() {
                    if let Ok(sens) = t.parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(sens));
                    }
                }
                true
            }
            "density" => {
                if let Some(t) = tokens.next() {
                    if let Ok(density) = t.parse::<u32>() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(density));
                    }
                }
                true
            }
            "spread" => {
                if let Some(t) = tokens.next() {
                    if let Ok(spread) = t.parse::<f32>() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceSpread(spread));
                    }
                }
                true
            }
            "mode" => {
                if let Some(t) = tokens.next() {
                    let mode = match t {
                        "sampler" => crate::engine::EngineMode::Sampler,
                        "effect" => crate::engine::EngineMode::Effect,
                        "controller" => crate::engine::EngineMode::Controller,
                        _ => crate::engine::EngineMode::Synth,
                    };
                    let _ = cmd_tx.send(EngineCommand::SetMode(mode));
                }
                true
            }
"param" => {
                if let (Some(name), Some(value_tok)) = (tokens.next(), tokens.next()) {
                    if let Ok(value) = value_tok.parse::<f32>() {
                        let param = match name {
                            "filter_cutoff" => Some(crate::engine::Param::FilterCutoff),
                            "filter_resonance" => Some(crate::engine::Param::FilterResonance),
                            "filter_mode" => Some(crate::engine::Param::FilterMode),
                            "reverb_mix" => Some(crate::engine::Param::ReverbMix),
                            "delay_time" => Some(crate::engine::Param::DelayTime),
                            "delay_feedback" => Some(crate::engine::Param::DelayFeedback),
                            "env_attack" => Some(crate::engine::Param::EnvAttack),
                            "env_release" => Some(crate::engine::Param::EnvRelease),
                            "waveform" => Some(crate::engine::Param::Waveform),
                            "master_gain" => Some(crate::engine::Param::MasterGain),
                            _ => None,
                        };
                        if let Some(param) = param {
                            let _ = cmd_tx.send(EngineCommand::SetParam(param, value));
                        }
                    }
                }
                true
            }
            // --- Graph text commands ---
            "graph_osc" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let wf_idx: i32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let freq: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(440.0);
                use crate::dsp::Waveform;
                let waveform = match wf_idx.clamp(0, 3) {
                    1 => Waveform::Saw, 2 => Waveform::Square, 3 => Waveform::Triangle,
                    _ => Waveform::Sine,
                };
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Oscillator(waveform, freq),
                });
                true
            }
            "graph_filter" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let mode_idx: i32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let cutoff: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(1000.0);
                let res: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.7);
                let mode = match mode_idx.clamp(0, 3) {
                    1 => crate::dsp::FilterMode::HighPass,
                    2 => crate::dsp::FilterMode::BandPass,
                    3 => crate::dsp::FilterMode::Notch,
                    _ => crate::dsp::FilterMode::LowPass,
                };
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Filter(mode, cutoff, res),
                });
                true
            }
            "graph_env" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let a: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.01);
                let d: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.2);
                let s: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.7);
                let r: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.3);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Envelope(a, d, s, r),
                });
                true
            }
            "graph_lfo" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let freq: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Lfo(freq),
                });
                true
            }
            "graph_delay" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let time_s: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.25);
                let fb: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.3);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Delay(time_s, fb),
                });
                true
            }
            "graph_reverb" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let room: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.85);
                let damp: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.5);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Reverb(room, damp),
                });
                true
            }
            "graph_noise" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let color_idx: i32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                use crate::dsp::NoiseColor;
                let color = match color_idx.clamp(0, 2) {
                    1 => NoiseColor::Pink, 2 => NoiseColor::Brown,
                    _ => NoiseColor::White,
                };
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Noise(color),
                });
                true
            }
            "graph_gain" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let gain: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Gain(gain),
                });
                true
            }
            "graph_dist" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let drive: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(2.0);
                let tone: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.5);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Distortion(drive, tone),
                });
                true
            }
            "graph_sampler" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Sampler,
                });
                true
            }
            "graph_mixer" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let g1: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                let g2: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Mixer(g1, g2),
                });
                true
            }
            "graph_splitter" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Splitter,
                });
                true
            }
            "graph_sdelay" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let t_l: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.1);
                let t_r: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.15);
                let cfb: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::StereoDelay(t_l, t_r, cfb),
                });
                true
            }
            "graph_output" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphAddNode {
                    id,
                    kind: crate::dsp::graph::GraphNodeKind::Output,
                });
                true
            }
            "graph_remove" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphRemoveNode { id });
                true
            }
            "graph_connect" => {
                let from: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let to: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphConnect { from, to });
                true
            }
            "graph_disconnect" => {
                let from: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let to: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let _ = cmd_tx.send(EngineCommand::GraphDisconnect { from, to });
                true
            }
            "graph_param" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let name = tokens.next().unwrap_or("");
                let value: f32 = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                use crate::dsp::graph::GraphParam;
                let param = match name {
                    "frequency" => Some(GraphParam::Frequency),
                    "waveform" => Some(GraphParam::Waveform),
                    "cutoff" => Some(GraphParam::Cutoff),
                    "resonance" => Some(GraphParam::Resonance),
                    "filter_mode" => Some(GraphParam::FilterMode),
                    "env_attack" => Some(GraphParam::EnvAttack),
                    "env_decay" => Some(GraphParam::EnvDecay),
                    "env_sustain" => Some(GraphParam::EnvSustain),
                    "env_release" => Some(GraphParam::EnvRelease),
                    "lfo_freq" => Some(GraphParam::LfoFreq),
                    "delay_time" => Some(GraphParam::DelayTime),
                    "delay_feedback" => Some(GraphParam::DelayFeedback),
                    "reverb_room" => Some(GraphParam::ReverbRoom),
                    "reverb_damping" => Some(GraphParam::ReverbDamping),
                    "gain" => Some(GraphParam::Gain),
                    "mixer_gain1" => Some(GraphParam::MixerGain1),
                    "mixer_gain2" => Some(GraphParam::MixerGain2),
                    "drive" => Some(GraphParam::Drive),
                    "tone" => Some(GraphParam::Tone),
                    "noise_color" => Some(GraphParam::NoiseColor),
                    "playback_rate" => Some(GraphParam::PlaybackRate),
                    "delay_left_time" => Some(GraphParam::DelayLeftTime),
                    "delay_right_time" => Some(GraphParam::DelayRightTime),
                    "cross_feedback" => Some(GraphParam::CrossFeedback),
                    _ => None,
                };
                if let Some(param) = param {
                    let _ = cmd_tx.send(EngineCommand::GraphSetParam { id, param, value });
                }
                true
            }
            "graph_trigger" => {
                let id: usize = tokens.next().and_then(|t| t.parse().ok()).unwrap_or(0);
                let gate: bool = tokens.next().and_then(|t| t.parse::<i32>().ok()).map(|v| v != 0).unwrap_or(false);
                let _ = cmd_tx.send(EngineCommand::GraphTriggerNode { id, gate });
                true
            }
            "graph_clear" => {
                let _ = cmd_tx.send(EngineCommand::GraphClear);
                true
            }
            _ => false,
        }
    }

    fn handle_osc_msg(msg: OscMessage, cmd_tx: &Sender<EngineCommand>) {
        match msg.addr.as_str() {
            "/note_on" => {
                if msg.args.len() >= 2 {
                    if let (Some(note), Some(vel)) = (msg.args[0].clone().int(), msg.args[1].clone().float()) {
                        let _ = cmd_tx.send(EngineCommand::NoteOn { note: note as u8, velocity: vel });
                    }
                }
            }
            "/note_off" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(note) = arg.clone().int() {
                        let _ = cmd_tx.send(EngineCommand::NoteOff { note: note as u8 });
                    }
                }
            }
            "/prime_base" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(base) = arg.clone().int() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimeBase(base as u32));
                    }
                }
            }
            "/prime_phase" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(phase) = arg.clone().float() {
                        let _ = cmd_tx.send(EngineCommand::SetPrimePhase(phase));
                    }
                }
            }
            "/nn_sensitivity" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(sens) = arg.clone().float() {
                        let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(sens));
                    }
                }
            }
            "/instance_density" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(density) = arg.clone().int() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(density as u32));
                    }
                }
            }
            "/instance_spread" => {
                if let Some(arg) = msg.args.first() {
                    if let Some(spread) = arg.clone().float() {
                        let _ = cmd_tx.send(EngineCommand::SetInstanceSpread(spread));
                    }
                }
            }
            "/mode" => {
                if let Some(arg) = msg.args.first() {
                    let mode = match arg.clone().string().as_deref() {
                        Some("sampler") => crate::engine::EngineMode::Sampler,
                        Some("effect") => crate::engine::EngineMode::Effect,
                        Some("controller") => crate::engine::EngineMode::Controller,
                        _ => crate::engine::EngineMode::Synth,
                    };
                    let _ = cmd_tx.send(EngineCommand::SetMode(mode));
                }
            }
            _ => {}
        }
    }
}
