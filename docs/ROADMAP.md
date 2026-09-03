# Modurust-Lite Roadmap

## What This Project IS NOT

**This is NOT a DAW.** There is:
- NO timeline
- NO clip matrix
- NO arrangement view
- NO transport controls
- NO automation lanes
- NO audio recording
- NO MIDI piano roll

This is a **modular DSP audio engine** - a synthesizer/effect processor that can run standalone or inside a real DAW (like Ableton Live).

## What This Project IS

**Modurust-Lite** is a headless modular DSP audio engine with three interchangeable frontends:
- Standalone egui GUI (`apps/standalone`) - for desktop use
- Max for Live UDP bridges (`apps/m4l/*`) - for Ableton Live control
- VST2 plugin (`vst/`) - for use as an instrument in any DAW

The engine processes audio through a graph of DSP nodes (oscillators, filters, effects, neural networks).

## Current State: 0.2.0 (2026-01-08)

### Completed

- Hot-reloadable root `config.json` (ConfigWatcher via `notify`).
- Full modular DSP node library: Oscillators, multi-mode Biquad filters, ADSR, Sampler, Noise, Gain, Distortion, LFO, Reverb, Mixer, Splitter, Router, StereoDelay.
- Lightweight node graph (`core/src/dsp/graph.rs`) with topological processing.
- SuperInstancePool voice allocation with voice stealing, density/spread control.
- PrimeScheduler prime-sequence pattern generation.
- Complex-valued neural network with phase-preserving activations.
- Real cpal audio backend + midir/cpal-shim MIDI backend.
- UDP-first binary control protocol (kinds 0–7) + OSC fallback.
- Text protocol on the UDP data port for M4L `udpsend`.
- Graph text protocol for live modular patching from M4L.
- Standalone egui app with config hot-reload and XY morph pad.
- Three interactive M4L bridges (instrument/analyzer/modulator).
- VST2 plugin exposing modurust_core as a VST instrument with UDP control.
- 47 tests passing (20 unit + 20 integration + 7 VST).

## Phase 1 — VST Plugin Stabilization (2-4 weeks)

### Goals
- Make the VST2 plugin production-ready
- Full bidirectional M4L ↔ VST communication
- Preset management

### Next Steps
1. Test VST plugin in Ableton Live
2. Add VST parameter smoothing
3. Add preset serialization (save/load)
4. Add proper MIDI Learn

## Phase 2 — M4L Enhancement (2-4 weeks)

### Goals
- Full control surface for all 16 parameters
- Real-time visual feedback from engine status
- Modular graph patching from M4L

### Next Steps
1. M4L visual controls for filter, reverb, delay
2. XY pad for prime/NN morph
3. Graph commands: add/remove/connect nodes
4. Status display (mode, peak, voices)

## Phase 3 — DSP Expansion (1-3 months)

### Goals
- More oscillator types
- More filter models
- More effects

### Planned
- State-variable filter (4-mode)
- Ladder filter (Moog-style)
- Wavetable oscillator
- FM synthesis
- Convolution reverb
- Granular synthesis

## Phase 4 — Hardware Deployment (3-6 months)

### Goals
- Run on embedded devices
- Minimal footprint
- MIDI/CV control

### Requirements
- Strip desktop dependencies
- Add I2S audio codec support
- Add hardware MIDI/CV interfaces

## Phase 5 — Advanced DSP (3-6 months)

### Planned
- Spectral processing (FFT-based filters)
- Physical modeling (Karplus-Strong strings)
- More neural network modes
- ONNX model loading

## Tracking & Ownership
- Issues: Create tasks per acceptance criteria
- Owners: TBD; use labels `ableton`, `m4l`, `vst`, `hardware`, `dsp`

## Notes
- Keep Bevy `LogPlugin` disabled due to `env_logger` init.
- The VST plugin uses `vst` crate 0.4 (VST2 API) for Ableton compatibility.
- M4L devices use only `std::net::UdpSocket` — no external dependencies.
- The standalone GUI uses `eframe` 0.26 — minimal dependencies.