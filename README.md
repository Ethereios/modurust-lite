# Modurust-Lite

A headless, modular DSP audio engine in Rust, exposed via:
- **Standalone GUI** (`apps/standalone`) — egui control surface
- **M4L bridges** (`apps/m4l/*`) — Max for Live UDP control devices
- **VST2 plugin** (`vst/`) — VST instrument for any DAW
- **CLAP plugin** (`clap/`) — CLAP plugin for modern DAWs (Ableton Live 12+, etc.)

## Workspace layout

```
modurust-vst3/
├── Cargo.toml            workspace: core, apps/standalone, apps/m4l/*, vst, clap
├── config.json           hot-reloadable root config
├── .cargo/config.toml    [build] jobs = 1
├── core/                 modurust_core (headless engine)
│   ├── src/config.rs     EngineConfig + ConfigWatcher (notify hot-reload)
│   ├── src/engine.rs     CoreEngine (command bus, config reload, voice render)
│   ├── src/dsp/          modular DSP node library + graph
│   ├── src/instance/     SuperInstancePool (polyphonic voice pool)
│   ├── src/prime/        PrimeScheduler (prime gates/patterns)
│   ├── src/nn/           ComplexNeuralNetwork (complex_phase/latent, feature-gated ONNX)
│   ├── src/io/           cpal audio output + midir/cpal-shim MIDI input
│   ├── src/comm/         UdpOscBridge (binary UDP primary, OSC fallback, status heartbeat)
│   ├── tests/            integration tests (engine + comm)
│   └── examples/         engine_demo, nn_morph
├── apps/
│   ├── standalone/       egui GUI (config hot-reload, Start Audio, note buttons, XY morph pad)
│   └── m4l/              instrument / analyzer / modulator UDP bridges
├── vst/                  modurust-vst VST2 plugin (exposes modurust_core as a VST instrument)
├── clap/                 modurust-clap CLAP plugin (exposes modurust_core as a CLAP plugin)
└── docs/                 architecture, changes, roadmap, node features, frontend status
```

## Quick start

```bash
# Check the workspace
cargo check --workspace

# Run all tests (47)
cargo test --workspace

# Run a core example
cargo run -p modurust_core --example engine_demo
cargo run -p modurust_core --example nn_morph

# Run the standalone GUI
cargo run -p modurust_standalone

# Build the VST2 plugin
cargo build -p modurust-vst --release
# Output: target/release/modurustvst.dll

# Build the CLAP plugin
cargo build -p modurust-clap --release
# Output: target/release/modurustclap.dll
```

## Features

- **Headless `modurust_core`** — full modular DSP graph, SuperInstance-style voice control, prime-based pattern generation/scheduling, and complex-valued neural network inference.
- **Hot-reloadable `config.json`** — `mode`, `audio_backend`, `midi_backend`, `udp_port`, `osc_port`, `prime_base`, `prime_phase`, `nn_mode`, `nn_sensitivity`, `instance_density`, `instance_spread`. Reloaded live via `ConfigWatcher` without stopping audio.
- **Real polyphonic voice rendering** — `CoreEngine` renders the `SuperInstancePool`: per-voice detuned saw/sine blend scaled by velocity, natural auto-release, master ADSR gated on first/last voice, NN-morphed filter cutoff, prime-gate ducking.
- **Complex neural network** — phase-preserving activations (modReLU/cardioid), `complex_phase`/`latent` modes, feature-gated `ort` ONNX loading.
- **Control surface** — UDP binary protocol (kinds 0–7) as primary, OSC fallback (all 8 addresses), a text protocol for M4L `udpsend`, and a full graph text protocol for live modular patching (18+ commands).
- **Standalone GUI** — starts the cpal audio stream (▶ Start Audio), note buttons drive the voice pool, All Notes Off, 2D XY complex-latent morph pad, and live config → engine push.
- **M4L bridges** — self-contained instrument/analyzer/modulator UDP run-loops (std::net::UdpSocket only, no extra deps).
- **VST2 plugin** — `modurust-vst` exposes modurust_core as a VST2 instrument with 16 parameters (filter, envelope, DSP, prime, NN, master) and MIDI input. Build with `cargo build -p modurust-vst --release`.
- **CLAP plugin** — `modurust-clap` exposes modurust_core as a CLAP instrument/audio-effect. Build with `cargo build -p modurust-clap --release`.

## Protocol (UDP binary, primary)

| Kind | Command | Payload |
|------|---------|---------|
| 0 | NoteOn | `note: u8`, `velocity: u8` |
| 1 | NoteOff | `note: u8` |
| 2 | SetPrimeBase | `u32` LE |
| 3 | SetNnSensitivity | `f32` LE |
| 4 | SetInstanceDensity | `u32` LE |
| 5 | SetInstanceSpread | `f32` LE |
| 6 | SetMode | ASCII mode string |
| 7 | SetPrimePhase | `f32` LE |

OSC fallback listens on `osc_port` (default 9001) for `/note_on`, `/note_off`, `/prime_base`, `/prime_phase`, `/nn_sensitivity`, `/instance_density`, `/instance_spread`, `/mode`.

A **text protocol** is also accepted on the UDP data port (primary for M4L `udpsend`), e.g. `noteon 60 100`, `noteoff 60`, `alloff`, `prime 5`, `phase 0.5`, `sens 0.85`, `density 32`, `spread 0.5`, `mode synth`, and `param <name> <value>` for DSP parameters (`filter_cutoff`, `filter_resonance`, `filter_mode`, `reverb_mix`, `delay_time`, `delay_feedback`, `env_attack`, `env_release`, `waveform`, `master_gain` — values normalized 0..=1).

A full **graph text protocol** is also accepted on the same UDP port, enabling live modular patching from M4L.

Engine status is broadcast on two disjoint port groups so the Max device and Rust bridges can run at the same time (Windows UDP allows only one bind per port):

- **Text feed** on `status_port` (default **9002**)
- **Binary feed** on `status_port+1..status_port+3` (default **9003–9005**)

## Tests

47 tests (27 core unit + 20 core integration), including voice-pool polyphony rendering, UDP end-to-end (binary + text `param` + noteon/alloff), OSC decode, status packet encoding, config `watch_once`, prime gate/pattern, engine command handling, `SetParam` filter/master-gain path, protocol parity, and full graph text-protocol coverage.

## Documentation

- `docs/architecture.md` — actual project architecture and data flow
- `docs/changes-features-ACCURATE.md` — honest implementation status
- `docs/ROADMAP.md` — roadmap
- `docs/node-system-features.md` — DSP node library
- `docs/frontend-status.md` — standalone GUI + M4L status
- `ABLETON_INTEGRATION.md` — M4L + VST setup guide for Ableton Live