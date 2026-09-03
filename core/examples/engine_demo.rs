//! Modurust-Lite Core Engine Demo
//!
//! Demonstrates the headless engine: config loading, command bus, prime
//! scheduling, complex-NN morphing, and the DSP chain rendered off the audio
//! thread.
//!
//! Run: `cargo run -p modurust_core --example engine_demo`

use modurust_core::comm::{encode_status_packet, EngineStatus};
use modurust_core::config::EngineConfig;
use modurust_core::engine::{CoreEngine, EngineCommand, EngineMode};

fn main() {
    println!("🎛️  Modurust-Lite Core Engine Demo");

    // Load the root config.json (hot-reloadable) with defaults fallback.
    let config = EngineConfig::load_from_file("config.json");
    println!("Loaded config: mode={} prime_base={} nn_sens={}",
        config.mode, config.prime_base, config.nn_sensitivity);

    let (mut engine, cmd_tx) = CoreEngine::new(config, 44100.0);

    // Send some commands over the engine command bus.
    let _ = cmd_tx.send(EngineCommand::NoteOn { note: 60, velocity: 0.9 });
    let _ = cmd_tx.send(EngineCommand::SetPrimeBase(5));
    let _ = cmd_tx.send(EngineCommand::SetNnSensitivity(0.6));
    let _ = cmd_tx.send(EngineCommand::SetMode(EngineMode::Synth));

    // Render 200 blocks (offline, ~1.16 seconds at 44100 Hz / 256 frames).
    let mut total_peak = 0.0f32;
    for i in 0..200 {
        let mut block = vec![0.0; 256];
        engine.process_block(&mut block);
        let block_peak = block.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        total_peak = total_peak.max(block_peak);
        if i % 50 == 0 {
            println!(
                "block {i:>3}: peak={block_peak:.4} prime_step={} voices={}",
                engine.prime_scheduler.step,
                engine.pool.active_count()
            );
        }
    }

    // Build a status packet for a potential M4L client.
    let status = EngineStatus {
        mode: 0,
        output_peak: total_peak,
        prime_base: engine.config.prime_base,
        prime_phase: engine.config.prime_phase,
        nn_sensitivity: engine.config.nn_sensitivity,
        instance_density: engine.config.instance_density,
        instance_spread: engine.config.instance_spread,
        active_voices: engine.pool.active_count(),
    };
    let packet = encode_status_packet(&status);
    println!("Status packet ({} bytes): {:02X?}", packet.len(), &packet[..8]);

    let _ = cmd_tx.send(EngineCommand::NoteOff { note: 60 });
    let mut tail = vec![0.0; 256];
    engine.process_block(&mut tail);

    println!("✅ Engine demo complete. Max peak: {total_peak:.4}");
}

