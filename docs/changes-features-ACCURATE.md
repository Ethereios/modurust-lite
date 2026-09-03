# Modurust-Lite: Changes & Features (Accurate)

## Summary

This is a **modular DSP audio engine** — NOT a DAW. There are no timeline, arrangement view, clip editing, or automation features.

## What Changed / Was Added

### Version 0.2.0

- **Core engine**: modurust_core rewritten as standalone library with modular DSP graph
- **Standalone GUI**: egui app with config hot-reload and XY morph pad
- **M4L bridges**: Three interactive Max for Live devices (instrument, analyzer, modulator) using UDP text protocol on port 9000
- **VST2 plugin**: modurust-vst — 16 parameters mapped to engine controls, UDP listener on port 9000, status broadcaster on port 9002
- **DSP nodes**: Added all core oscillator and filter nodes (Oscillator, BiquadFilter, ADSR, Sampler, Noise, SimpleDelay, Gain, Distortion, LFO, Reverb, Mixer, Splitter, Router, StereoDelay)
- **Node graph**: `core/src/dsp/graph.rs` with arena allocator and Kahn topological sort
- **Voice pool**: SuperInstancePool with voice stealing, density/spread control, auto-release
- **Prime sequencing**: PrimeScheduler with prime gates, pattern buffers
- **Neural network**: ComplexNeuralNetwork with modReLU/cardioid activations, feature-gated ONNX loading
- **MIDI support**: midir backend with cpal-shim, selectable input device
- **Audio backend**: cpal 0.15 with real audio output
- **UDP control**: Binary protocol (kinds 0–7) + OSC fallback + text protocol for M4L
- **Status feed**: Text on port 9002, binary on ports 9003–9005
- **Config hot-reload**: ConfigWatcher monitors config.json changes
- **Tests**: 47 passing (20 core unit + 20 core integration + 7 VST unit)

### Bug Fixes

- Fixed process_events iterator bug (events.events() vs events.events.iter())
- Fixed type errors in VST plugin (i32 for unique_id/parameters, mut audio outputs)
- Removed unused filter_cutoff_norm and filter_resonance_norm functions
- Fixed UDP command parser for M4L text protocol
- Fixed status packet encoding

## What Is NOT in This Project

**Removed / never planned:**
- Timeline / arrangement view
- Clip matrix / scene launching
- Audio recording
- MIDI piano roll
- Automation lanes
- Clip editing
- DAW-style track mixing
- Transport controls (play/stop/record)
- Song position / beat tracking

## Current Feature Set (DSP Focus)

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
| **Output** | cpal audio device, VST plugin output |

## How to Use

### Standalone
```bash
cargo run -p Modurust-standalone
```
- Press "Start Audio"
- Use note buttons or XY morph pad
- config.json hot-reload enabled

### VST in Ableton Live
```bash
cargo build -p modurust-vst --release
```
- Load `target/release/modurustvst.dll` as VST2 instrument
- MIDI notes trigger voices
- Send UDP text to port 9000 for parameter control
- Monitor status on port 9002

### M4L Devices
- Load M4L bridges from `apps/m4l/`
- Send parameter commands via UDP to port 9000
- Receive status feedback on port 9002

## Files Modified / Added

- `README.md` — rewritten accurate overview
- `docs/architecture.md` — correct data flow
- `docs/ROADMAP.md` — reality-based, no DAW aspirations
- `docs/changes-features-ACCURATE.md` — this file
- `docs/node-system-features.md` — accurate node library status
- `docs/frontend-status.md` — accurate GUI/M4L/VST status
- `FUTURE_GOALS.md` — rewritten with correct priorities
- `ABLETON_INTEGRATION.md` — new: M4L + VST setup guide
- `vst/src/lib.rs` — UDP listeners, 16 params, MIDI input, audio output
- `core/src/dsp/mod.rs` — mode() accessor added
- `core/src/lib.rs` — re-exports Param, EngineMode
- `.cargo/config.toml` — jobs = 1, cargo aliases
