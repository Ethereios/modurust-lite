# Modurust-Lite Architecture

## Overview

Modurust-Lite is a **headless modular DSP audio engine** in Rust. It is exposed through four frontends:
1. A standalone egui GUI (`apps/standalone`)
2. Max for Live UDP bridges (`apps/m4l/*`)
3. A CLAP plugin (`clap/`) — primary plugin format for modern DAWs
4. A VST2 plugin (`vst/`) — universal DAW compatibility

The core engine (`modurust_core`) is the only required component; the frontends are optional and interchangeable.

## Project Structure

```
modurust-vst3/
├── core/                 modurust_core (headless engine)
│   ├── src/config.rs     EngineConfig + ConfigWatcher (notify hot-reload)
│   ├── src/engine.rs     CoreEngine (command bus, config reload, DSP graph render, peak tracking)
│   ├── src/lib.rs        crate root; re-exports EngineConfig, CoreEngine, EngineCommand
│   ├── src/dsp/
│   │   ├── mod.rs        Oscillator, BiquadFilter (LP/HP/BP/Notch), ADSR, Sampler,
│   │   │                 SimpleDelay, Noise (white/pink/brown), Gain, Distortion
│   │   ├── extra_nodes.rs  LFO, Reverb, Mixer, Splitter, Router (N×M), StereoDelay
│   │   └── graph.rs      DspGraph (arena, Kahn topo sort, cycle fallback)
│   ├── src/instance/
│   │   └── mod.rs        SuperInstancePool (voice stealing, density → capacity, spread → detune)
│   ├── src/prime/
│   │   └── mod.rs        PrimeScheduler (prime gates, pattern buffers, prime_base)
│   ├── src/nn/
│   │   └── mod.rs        ComplexNeuralNetwork (NnMode, modReLU/cardioid, feature-gated ONNX)
│   ├── src/io/
│   │   ├── audio.rs      AudioDeviceManager (cpal 0.15) + CpalEngineHost
│   │   └── midi.rs       MidiManager (midir + cpal-shim, selectable) + parser
│   └── src/comm/
│       └── mod.rs        UdpOscBridge (binary UDP primary, OSC fallback, status heartbeat)
├── apps/
│   ├── standalone/       egui GUI (config hot-reload, Start Audio, note buttons, XY morph pad)
│   └── m4l/              instrument / analyzer / modulator UDP bridges
├── clap/                 modurust-clap CLAP plugin (exposes modurust_core as a CLAP instrument/effect)
└── vst/                  modurust-vst VST2 plugin (exposes modurust_core as a VST instrument)
```

## Data Flow

```
config.json ──ConfigWatcher──▶ CoreEngine (hot reload, non-blocking)
                                   ▲
M4L / MIDI / GUI ──EngineCommand──┤
                                   │
UDP 9000 (binary + text) / 9001 (OSC) ──UdpOscBridge──▶ cmd bus
                                   │
CoreEngine.process_block ──▶ SuperInstancePool voice render
                                   │
           ──▶ Biquad Filter → master ADSR → Delay → Reverb (prime ducking, spread, master gain)
                                   │
                     cpal output stream (AudioDeviceManager)
```

## Voice-Pool Rendering

- `CoreEngine` renders a **real polyphonic `SuperInstancePool`** as the audible source: per-voice detuned saw/sine blend scaled by velocity, with a natural exponential auto-release so voices die out even without an explicit `NoteOff`.
- The master ADSR gates on the **first** active voice and off on the **last**; the complex NN modulates the filter cutoff **around a user-set `filter_cutoff_base`** (scaled by `nn_sensitivity`, so manual/M4L cutoff control is not clobbered); a prime gate applies subtle ducking between prime steps.
- `EngineCommand::SetParam(Param, value)` + the text protocol `param <name> <value>` provide live control of filter cutoff/resonance/mode, reverb mix, delay time/feedback, env attack/release, waveform, and master gain.

## Frontend Implementations

### Standalone GUI (`apps/standalone`)
- Loads/saves `config.json` with live watcher
- Starts cpal audio via `CpalEngineHost`
- Note buttons + All Notes Off
- XY morph pad (prime_phase / nn_sensitivity)
- Live config → engine push

### M4L Bridges (`apps/m4l/*`)
- Instrument: plays chord progression with arpeggiation
- Analyzer: sweeps NN sensitivity, parses status
- Modulator: sweeps prime base/density/spread, parses status
- All use std::net::UdpSocket, no external deps

### CLAP Plugin (`clap/`)
- Exposes modurust_core as a CLAP instrument/audio-effect
- 16 parameters mapped to engine controls (same as VST2)
- MIDI input (NoteOn/NoteOff)
- Audio output via CoreEngine::process_block
- UDP control listener on port 9000 for M4L text commands
- UDP status broadcaster on port 9002 for M4L monitoring
- Build: `cargo build -p modurust-clap --release`

### VST2 Plugin (`vst/`)
- Exposes modurust_core as a VST2 instrument
- 16 parameters mapped to engine controls
- MIDI input (NoteOn/NoteOff)
- Audio output via CoreEngine::process_block
- UDP control listener on port 9000 for M4L text commands
- UDP status broadcaster on port 9002 for M4L monitoring
- Build: `cargo build -p modurust-vst --release`

## Engine Command API

`EngineCommand` enum provides:
- `NoteOn { note, velocity }`, `NoteOff { note }`, `AllNotesOff`
- `SetPrimeBase(u32)`, `SetPrimePhase(f32)`
- `SetNnSensitivity(f32)`, `SetInstanceDensity(u32)`, `SetInstanceSpread(f32)`
- `SetMode(EngineMode)`
- `SetParam(Param, f32)` — for filter_cutoff, filter_resonance, filter_mode, reverb_mix, delay_time, delay_feedback, env_attack, env_release, waveform, master_gain
- `Graph*` commands for live modular patching

## Status Feed

- **Text feed** on `status_port` (9002): labeled datagrams for Max for Live
- **Binary feed** on `status_port+1..+3` (9003–9005): 31-byte packets for Rust bridges

## Tests

47 tests (20 core unit + 20 core integration + 7 VST unit) covering voice-pool polyphony, UDP end-to-end, OSC decode, status packet encoding, config hot-reload, prime gate/pattern, engine command handling, SetParam path, protocol parity, and graph text-protocol coverage.
