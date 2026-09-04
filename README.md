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
│   ├── standalone/       egui GUI (visual node graph editor, XY morph pad, 16-param controls, note buttons, config save/load)
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
- **CLAP plugin** — `modurust-clap` exposes modurust_core as a CLAP instrument/audio-effect (primary plugin format). Build with `cargo build -p modurust-clap --release`.
- **VST2 plugin** — `modurust-vst` exposes modurust_core as a VST2 instrument with 16 parameters (filter, envelope, DSP, prime, NN, master) and MIDI input. Build with `cargo build -p modurust-vst --release`.

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

## Bug Fixes (v0.2.0)

- Fixed process_events iterator bug (events.events() vs events.events.iter())
- Fixed type errors in VST plugin (i32 for unique_id/parameters, mut audio outputs)
- Removed unused filter_cutoff_norm and filter_resonance_norm functions
- Fixed UDP command parser for M4L text protocol
- Fixed status packet encoding

## Feature Overview

| Category | Features |
|----------|----------|
| **Oscillators** | Sine, saw, square, triangle; wavetable support planned |
| **Filters** | Biquad: LP, HP, BP, Notch; state-variable planned |
| **Envelopes** | ADSR with per-voice auto-release |
| **Effects** | SimpleDelay, Reverb, Distortion, Gain |
| **Modulation** | LFO, PrimeScheduler pattern generation |
| **Network** | ComplexNeuralNetwork with modReLU/cardioid |
| **Graph** | Topological sort, N×M node connections |
| **Polyphony** | SuperInstancePool with voice stealing |
| **Control** | MIDI note-on/off, param textset, UDP commands |
| **Output** | cpal audio device, CLAP/VST plugin output |

## Ableton Live Integration

### CLAP Plugin (Recommended)
```bash
cargo build -p modurust-clap --release
```
Copy `target/release/modurustclap.dll` to CLAP folder, rescan in Ableton Live 12+, insert as Instrument/Audio Effect.

### VST2 Plugin
```bash
cargo build -p modurust-vst --release
```
Copy `target/release/modurustvst.dll` to VST folder, rescan in Ableton, insert as Instrument/Audio Effect.

### M4L Bridges
Load `ModurustLite_Instrument.amxd`, `ModurustLite_Analyzer.amxd`, `ModurustLite_Modulator.amxd` from `apps/m4l/`. Configure UDP ports in `config.json` (9000/9001/9002).

### Available Controls
**Via VST/CLAP automation (16 params):** Filter Cutoff, Resonance, Mode, Reverb Mix, Delay Time/Feedback, Env Attack/Release, Waveform, Master Gain, Prime Base, Prime Phase, NN Sensitivity, Instance Density/Spread, Engine Mode.

**Via M4L UDP:** NoteOn/NoteOff, Prime/Base/Phase, NN Sensitivity, Instance Density/Spread, Mode, Graph text protocol commands.

### Graph Text Protocol (live modular patching from M4L)
```
graph_osc 1 sine 440
graph_filter 2 lowpass 1000 0.7
graph_connect 1 2
graph_output 2
graph_param 1 frequency 880
graph_param 2 cutoff 500
graph_clear
```

## Future Roadmap (Priority Ordered)

### Priority 1: Plugin Stabilization
- CLAP/VST parameter smoothing
- Preset serialization (save/load)
- MIDI Learn support

### Priority 2: M4L Enhancement
- Visual controls for all 16 parameters
- XY morph pad for prime/NN
- Graph patch commands from M4L
- Status display (mode, peak, voices)

### Priority 3: DSP Expansion
- State-variable filter (4-mode)
- Ladder filter (Moog-style)
- Wavetable oscillator
- FM synthesis
- Convolution reverb
- Granular synthesis
- ONNX model loading

### Priority 4: Hardware Deployment
- Remove desktop dependencies (minimal build)
- I2S audio codec integration
- MIDI UART, GPIO controls
- Target: Raspberry Pi, Teensy, ESP32

## Documentation

- `README.md` — architecture, protocol, frontends, node system, Ableton integration
- `TODO.md` — priority task list, completed phases
- `docs/architecture.md` — detailed architecture and data flow
- `docs/frontend-status.md` — standalone GUI, M4L, VST2, CLAP status
- `docs/node-system-features.md` — DSP node library and planned features