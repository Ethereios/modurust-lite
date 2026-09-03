//! Integration tests: CoreEngine + UdpOscBridge + ConfigWatcher working together.

use modurust_core::comm::{encode_status_packet, EngineStatus, UdpOscBridge};
use modurust_core::config::EngineConfig;
use modurust_core::engine::{CoreEngine, EngineCommand, EngineMode, Param};
use modurust_core::prime::PrimeScheduler;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn test_config() -> EngineConfig {
    let mut c = EngineConfig::default();
    c.instance_density = 8;
    c.prime_base = 3;
    c.nn_sensitivity = 0.4;
    c.instance_spread = 0.6;
    c
}

/// Send `datagram` to `(addr, port)`, then poll `engine.process_block` until
/// `predicate` holds (up to 2s). Returns true if the predicate eventually held.
/// This avoids flaky fixed-sleep timing when the workspace test binary runs
/// many UDP tests in parallel under load.
fn send_udp_and_wait<F>(
    client: &std::net::UdpSocket,
    addr: (&str, u16),
    datagram: &[u8],
    engine: &mut CoreEngine,
    predicate: F,
) -> bool
where
    F: Fn(&CoreEngine) -> bool,
{
    let _ = client.send_to(datagram, addr);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    let mut output = vec![0.0; 64];
    while std::time::Instant::now() < deadline {
        engine.process_block(&mut output);
        if predicate(engine) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    predicate(engine)
}

#[test]
fn test_engine_note_on_produces_signal() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::NoteOn { note: 60, velocity: 0.8 });

    let mut output = vec![0.0; 256];
    engine.process_block(&mut output);

    // The oscillator chain should produce a non-silent signal.
    let peak = output.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.0, "expected non-silent output, got peak={peak}");
    assert!(engine.output_peak > 0.0);
}

#[test]
fn test_engine_voice_pool_renders_polyphony() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);

    // Allocate two voices (a chord).
    let _ = cmd_tx.send(EngineCommand::NoteOn { note: 60, velocity: 0.8 });
    let _ = cmd_tx.send(EngineCommand::NoteOn { note: 64, velocity: 0.6 });

    let mut output = vec![0.0; 512];
    engine.process_block(&mut output);

    // Both voices should be active and rendered into the output.
    assert_eq!(engine.pool.active_count(), 2, "both voices should be active");
    let peak = output.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.0, "polyphonic voice pool should produce signal, peak={peak}");

    // Release one note; the other voice should keep sounding.
    let _ = cmd_tx.send(EngineCommand::NoteOff { note: 60 });
    let mut output2 = vec![0.0; 512];
    engine.process_block(&mut output2);
    assert_eq!(engine.pool.active_count(), 1, "one voice should remain after release");

    // AllNotesOff releases everything.
    let _ = cmd_tx.send(EngineCommand::AllNotesOff);
    let mut output3 = vec![0.0; 512];
    engine.process_block(&mut output3);
    assert_eq!(engine.pool.active_count(), 0, "all voices released");
}

#[test]
fn test_engine_command_set_prime_base() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::SetPrimeBase(7));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.config.prime_base, 7);
}

#[test]
fn test_engine_command_set_prime_phase() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::SetPrimePhase(0.75));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert!((engine.config.prime_phase - 0.75).abs() < 1e-5);
}

#[test]
fn test_engine_command_set_mode() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::SetMode(EngineMode::Effect));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.config.mode, "effect");
}

#[test]
fn test_engine_command_set_instance_density() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::SetInstanceDensity(32));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.config.instance_density, 32);
    assert_eq!(engine.pool.capacity(), 32);
}

#[test]
fn test_engine_apply_config_reload() {
    let (mut engine, _cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let mut new_cfg = test_config();
    new_cfg.prime_base = 11;
    new_cfg.nn_sensitivity = 0.9;
    new_cfg.instance_density = 24;
    new_cfg.instance_spread = 0.25;

    engine.apply_config(new_cfg);

    assert_eq!(engine.config.prime_base, 11);
    assert_eq!(engine.config.nn_sensitivity, 0.9);
    assert_eq!(engine.config.instance_spread, 0.25);
    assert_eq!(engine.pool.capacity(), 24);
}

#[test]
fn test_status_packet_encoding() {
    let status = EngineStatus {
        mode: 1,
        output_peak: 0.5,
        prime_base: 5,
        prime_phase: 0.25,
        nn_sensitivity: 0.75,
        instance_density: 16,
        instance_spread: 0.5,
        active_voices: 3,
    };
    let packet = encode_status_packet(&status);
    assert_eq!(packet.len(), 31);
    assert_eq!(packet[0], 0x4D);
    assert_eq!(packet[1], 0x01);
    assert_eq!(packet[2], 1);
    let peak = f32::from_le_bytes([packet[3], packet[4], packet[5], packet[6]]);
    assert!((peak - 0.5).abs() < 1e-5);
    let base = u32::from_le_bytes([packet[7], packet[8], packet[9], packet[10]]);
    assert_eq!(base, 5);
    let voices = u32::from_le_bytes([packet[27], packet[28], packet[29], packet[30]]);
    assert_eq!(voices, 3);
}

#[test]
fn test_udp_bridge_binary_protocol_end_to_end() {
    // Bind the bridge on a high ephemeral port to avoid conflicts.
    let udp_port: u16 = 19100;
    let osc_port: u16 = 19101;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));

    // Start the UDP listener that forwards packets to the engine.
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));

    // Small sleep so the listener thread binds.
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send a NoteOn packet over UDP and wait for the engine to become audible.
    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let note_heard = send_udp_and_wait(
        &client,
        ("127.0.0.1", udp_port),
        &[0u8, 60, 100],
        &mut engine,
        |e| e.pool.active_count() > 0,
    );
    assert!(note_heard, "engine should become audible after UDP NoteOn");

    // Send SetPrimeBase(7) over UDP and wait until it applies.
    let mut set_base = [2u8, 0, 0, 0, 0];
    set_base[1..].copy_from_slice(&7u32.to_le_bytes());
    let base_applied = send_udp_and_wait(
        &client,
        ("127.0.0.1", udp_port),
        &set_base,
        &mut engine,
        |e| e.config.prime_base == 7,
    );
    assert!(base_applied, "engine prime_base should become 7 after UDP SetPrimeBase");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_config_watch_once() {
    // Write a temporary config, watch_once should load it.
    let path = std::env::temp_dir().join("modurust_test_config.json");
    let cfg = test_config();
    std::fs::write(&path, cfg.to_json()).unwrap();
    let loaded = modurust_core::config::ConfigWatcher::watch_once(&path);
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().prime_base, 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_prime_scheduler_gate_with_config() {
    let mut scheduler = PrimeScheduler::new();
    scheduler.set_base(2);
    assert!(scheduler.gate_signal());
    scheduler.tick();
    assert!(!scheduler.gate_signal());
}

#[test]
fn test_engine_set_param_filter_cutoff() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    // Disable NN morphing so the cutoff maps exactly to the requested base.
    engine.config.nn_sensitivity = 0.0;
    // 0.5 normalized -> mid cutoff.
    let _ = cmd_tx.send(EngineCommand::SetParam(Param::FilterCutoff, 0.5));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert!(engine.filter.cutoff > 20.0 && engine.filter.cutoff < 20000.0);
    assert!((engine.filter.cutoff - (20.0 + 0.5 * 19980.0)).abs() < 1.0);
}

#[test]
fn test_engine_set_param_master_gain_zeros() {
    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let _ = cmd_tx.send(EngineCommand::NoteOn { note: 60, velocity: 0.8 });
    // First render produces sound.
    let mut output = vec![0.0; 256];
    engine.process_block(&mut output);
    let peak_before = output.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak_before > 0.0);

    // Master gain 0 -> silent.
    let _ = cmd_tx.send(EngineCommand::SetParam(Param::MasterGain, 0.0));
    let mut output2 = vec![0.0; 256];
    engine.process_block(&mut output2);
    let peak_after = output2.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert_eq!(peak_after, 0.0);
}

#[test]
fn test_udp_bridge_binary_protocol_set_phase() {
    let udp_port: u16 = 19104;
    let osc_port: u16 = 19105;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    // Binary kind 7 = SetPrimePhase (f32 LE).
    let mut set_phase = [7u8, 0, 0, 0, 0];
    set_phase[1..].copy_from_slice(&0.75f32.to_le_bytes());
    let applied = send_udp_and_wait(
        &client,
        ("127.0.0.1", udp_port),
        &set_phase,
        &mut engine,
        |e| (e.config.prime_phase - 0.75).abs() < 1e-5,
    );
    assert!(applied, "engine prime_phase should become 0.75 after UDP binary set-phase");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_udp_bridge_text_protocol_phase() {
    let udp_port: u16 = 19106;
    let osc_port: u16 = 19107;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    // Text command: "phase 0.25"
    let applied = send_udp_and_wait(
        &client,
        ("127.0.0.1", udp_port),
        b"phase 0.25",
        &mut engine,
        |e| (e.config.prime_phase - 0.25).abs() < 1e-5,
    );
    assert!(applied, "engine prime_phase should become 0.25 after UDP text command");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_udp_bridge_osc_phase() {
    let udp_port: u16 = 19108;
    let osc_port: u16 = 19109;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_osc_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    // OSC message: /prime_phase 0.9 (float)
    let osc_msg = rosc::OscMessage {
        addr: "/prime_phase".to_string(),
        args: vec![rosc::OscType::Float(0.9)],
    };
    let packet = rosc::encoder::encode(&rosc::OscPacket::Message(osc_msg)).unwrap();
    let applied = send_udp_and_wait(
        &client,
        ("127.0.0.1", osc_port),
        &packet,
        &mut engine,
        |e| (e.config.prime_phase - 0.9).abs() < 1e-5,
    );
    assert!(applied, "engine prime_phase should become 0.9 after OSC message");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_udp_bridge_text_protocol_param() {
    let udp_port: u16 = 19102;
    let osc_port: u16 = 19103;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    // Disable NN morphing so `param filter_cutoff` maps exactly to the base.
    engine.config.nn_sensitivity = 0.0;
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    // Text command: "param filter_cutoff 0.5"
    let _ = client.send_to(b"param filter_cutoff 0.5", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert!((engine.filter.cutoff - (20.0 + 0.5 * 19980.0)).abs() < 2.0);

    // Text command: "noteon 60 100"
    let _ = client.send_to(b"noteon 60 100", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output2 = vec![0.0; 256];
    engine.process_block(&mut output2);
    let peak = output2.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.0);

    // Text command: "alloff"
    let _ = client.send_to(b"alloff", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output3 = vec![0.0; 64];
    engine.process_block(&mut output3);
    assert_eq!(engine.pool.active_count(), 0);

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Phase L1: Graph text protocol commands over UDP
// ---------------------------------------------------------------------------

#[test]
fn test_udp_bridge_text_graph_add_and_connect() {
    let udp_port: u16 = 19110;
    let osc_port: u16 = 19111;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

    // Add an oscillator at node id 1, a filter at id 2, an output at id 3.
    let _ = client.send_to(b"graph_osc 1 0 440", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_filter 2 0 1000 0.7", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_output 3", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.graph.live_count(), 3, "three graph nodes should be live");
    assert_eq!(engine.graph.node_name(1), Some("oscillator"));
    assert_eq!(engine.graph.node_name(2), Some("filter"));
    assert_eq!(engine.graph.node_name(3), Some("output"));

    // Connect osc -> filter -> output.
    let _ = client.send_to(b"graph_connect 1 2", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_connect 2 3", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output2 = vec![0.0; 128];
    engine.process_block(&mut output2);
    assert_eq!(engine.graph.edge_count(), 2, "two graph edges should exist");
    // Oscillator routed through graph should produce signal.
    let peak = output2.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.0, "graph with osc->filter->output should produce signal");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_udp_bridge_text_graph_param_trigger_remove_clear() {
    let udp_port: u16 = 19112;
    let osc_port: u16 = 19113;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

    // Add an envelope (id 1) and a sampler (id 2) and an output (id 3).
    let _ = client.send_to(b"graph_env 1 0.01 0.1 0.5 0.2", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_sampler 2", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_output 3", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.graph.live_count(), 3);

    // Set a graph param on the envelope: env_sustain 0.9.
    let _ = client.send_to(b"graph_param 1 env_sustain 0.9", ("127.0.0.1", udp_port));
    // Trigger the sampler (gate 1) and the envelope gate on (gate 1).
    let _ = client.send_to(b"graph_trigger 2 1", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_trigger 1 1", ("127.0.0.1", udp_port));
    // Connect sampler -> envelope -> output.
    let _ = client.send_to(b"graph_connect 2 1", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_connect 1 3", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));

    let mut output2 = vec![0.0; 256];
    engine.process_block(&mut output2);
    // Sampler triggered through envelope should produce sound.
    let peak = output2.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak > 0.0, "sampler->env->output should produce signal, peak={peak}");

    // Remove node id 2 (sampler).
    let _ = client.send_to(b"graph_remove 2", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output3 = vec![0.0; 64];
    engine.process_block(&mut output3);
    assert_eq!(engine.graph.live_count(), 2, "after remove, two nodes remain");
    assert_eq!(engine.graph.edge_count(), 1, "after remove, only env->output edge remains");

    // Clear the entire graph.
    let _ = client.send_to(b"graph_clear", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output4 = vec![0.0; 64];
    engine.process_block(&mut output4);
    assert_eq!(engine.graph.live_count(), 0, "graph_clear should empty the graph");

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_udp_bridge_text_graph_extra_nodes() {
    let udp_port: u16 = 19114;
    let osc_port: u16 = 19115;

    let (mut engine, cmd_tx) = CoreEngine::new(test_config(), 44100.0);
    let bridge = UdpOscBridge::new(udp_port, osc_port);
    let running = Arc::new(AtomicBool::new(true));
    bridge.start_udp_listener(cmd_tx.clone(), Arc::clone(&running));
    std::thread::sleep(std::time::Duration::from_millis(100));

    let client = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();

    // Add a mixer, splitter, and stereo delay — the Phase I extra nodes.
    let _ = client.send_to(b"graph_mixer 1 0.8 0.6", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_splitter 2", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_sdelay 3 0.1 0.15 0.2", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output = vec![0.0; 64];
    engine.process_block(&mut output);
    assert_eq!(engine.graph.live_count(), 3);
    assert_eq!(engine.graph.node_name(1), Some("mixer"));
    assert_eq!(engine.graph.node_name(2), Some("splitter"));
    assert_eq!(engine.graph.node_name(3), Some("stereo_delay"));

    // Set mixer gains via graph_param.
    let _ = client.send_to(b"graph_param 1 mixer_gain1 0.5", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_param 1 mixer_gain2 0.25", ("127.0.0.1", udp_port));
    // Set stereo delay params.
    let _ = client.send_to(b"graph_param 3 delay_left_time 0.2", ("127.0.0.1", udp_port));
    let _ = client.send_to(b"graph_param 3 cross_feedback 0.1", ("127.0.0.1", udp_port));
    std::thread::sleep(std::time::Duration::from_millis(200));
    let mut output2 = vec![0.0; 128];
    engine.process_block(&mut output2);

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

