# Modurust-Lite Architecture

## Overview

Modurust-Lite is a **headless modular DSP audio engine** in Rust, built on a 15-node DSP library (oscillators, biquad filters, ADSR, sampler, noise, distortion, reverb, delay, mixer, splitter, router, LFO). It exposes the engine through **four distinct frontends**, each serving a different user and operating differently from the others:

| Frontend | Purpose | Who Uses It |
|----------|---------|-------------|
| **Standalone GUI** (`apps/standalone`) | Desktop DAW-style editor — node graph, drum machine, FX rack, presets | Professional musicians, audio engineers, DSP hobbyists on desktop |
| **VST2 Plugin** (`vst/`) | Plugin instrument in any DAW (Cubase, Logic, FL Studio, etc.) | Any DAW user needing a modular synth in their session |
| **CLAP Plugin** (`clap/`) | Modern DAW plugin (DAW 12+). Primary distribution target. | Same users as VST2, plus modern-host preference |
| **M4L Devices** (`apps/m4l/*`) | External Max for Live devices — control surface + visual feedback for plugins | Ableton Live users wanting to integrate Modurust-Lite externally |

The core engine (`modurust_core`) is the shared DSP backbone. Frontends are **not interchangeable** — each has a specific role, and feature gaps are filled per-app rather than merged into the core.

---

## Core Engine (`modurust_core`)

### Module Layout
```
modurust_core/src/
├── lib.rs              Re-exports EngineConfig, CoreEngine, EngineCommand
├── config.rs           EngineConfig + ConfigWatcher (notify hot-reload)
├── engine.rs           CoreEngine — command bus, DSP graph render, peak tracking
├── dsp/
│   ├── mod.rs          OscillatorNode, BiquadFilterNode, AdsrEnvelopeNode,
│   │                   SimpleDelayNode, NoiseNode, GainNode, DistortionNode
│   ├── extra_nodes.rs  LfoNode, ReverbNode, MixerNode, SplitterNode,
│   │                   RouterNode, StereoDelayNode
│   └── graph.rs        DspGraph — arena, Kahn topo sort, cycle fallback
├── instance/
│   └── mod.rs          SuperInstancePool — voice stealing, density→capacity, spread→detune
├── prime/
│   └── mod.rs          PrimeScheduler — prime gates, pattern buffers, base value
├── nn/
│   └── mod.rs          ComplexNeuralNetwork — NnMode, modReLU/cardioid activations
├── io/
│   ├── audio.rs        AudioDeviceManager (cpal 0.15) + CpalEngineHost
│   └── midi.rs         MidiManager (midir + cpal-shim) + parser
└── comm/
    └── mod.rs          UdpOscBridge — binary UDP primary, OSC fallback, status heartbeat
```

### Data Flow
```
config.json → ConfigWatcher → CoreEngine (hot reload, non-blocking)
                                          ▲
User input / M4L MIDI / GUI ← EngineCommand │
                                     │       │
UDP 9000 (binary+text) / 9001 (OSC) → UdpOscBridge → cmd bus
     │                                  │
CoreEngine.process_block → SuperInstancePool voice render
    │                                   │
    ├── Biquad Filter → master ADSR → Delay → Reverb
    │        (prime ducking × spread modulation × master gain)
    │
    └── cpal output stream
```

### Voice Pool Rendering
Real polyphonic rendering with detuned saw/sine blend per voice, auto-release so voices die out without explicit NoteOff. Master ADSR gates on first active voice, off on last inactive. Complex NN modulates cutoff *around* `filter_cutoff_base`, scaled by `nn_sensitivity`. Prime gate applies subtle ducking between prime steps.

### Parameters (16 M4L-addressable)
| Param | Type | Range | Description |
|-------|------|-------|-------------|
| filter_cutoff | f32 | 20–20000 Hz | Filter cutoff frequency |
| filter_resonance | f32 | 0–1 | Filter Q |
| filter_mode | u32 | 0–3 | LP/HP/BP/Notch |
| reverb_mix | f32 | 0–1 | Wet/dry mix |
| delay_time | f32 | 0–1 | Normalized delay (s) |
| delay_feedback | f32 | 0–1 | Feedback amount |
| env_attack | f32 | 0–1 | ADSR attack (normalized) |
| env_release | f32 | 0–1 | ADSR release (normalized) |
| waveform | u32 | 0–3 | Sine/Saw/Square/Triangle |
| master_gain | f32 | 0–1 | Master output volume |
| prime_base | u32 | 1+ | Prime sequence base number |
| prime_phase | f32 | 0–1 | Prime sequence phase |
| nn_sensitivity | f32 | 0–1 | NN morph influence |
| instance_density | u32 | 1–16 | Voice pool capacity |
| instance_spread | f32 | 0–1 | Voice detune spread |

---

## Frontend Implementations

### Standalone GUI (`apps/standalone`)

**Purpose**: Full DAW-style editing environment. Drag-and-drop node graph, drum machine section, FX rack, XY morph pad, pattern sequencer, preset save/load, spectral sample browser. This is the ONLY app where the user directly edits the DSP graph at runtime.

**Current features (egui)**:
- Visual node graph editor — 15-node palette, drag-create, cable connections, click-select, parameter panel per node
- Global parameter strip for all 16 parameters
- Mode selector (Synth / Sampler / Effect / Controller)
- Note buttons + All Notes Off
- XY morph pad (prime_phase / nn_sensitivity)
- Save/Load config.json
- Status display (mode, voice count, peak meter)
- UDP bridge wired at startup

**Future (iced migration)**:
- Tabbed interface: **Modular Synth** (node graph), **Drum Machine** (8-pad kit + 16-step seq), **FX Rack** (per-FX params)
- Sample browser with drag-drop sample loading
- Pattern sequencer panel (per-track velocity, probability, plocks, swing)
- Preset save/load (JSON)
- Visual stats: peak meter, voice count, SNR per channel

**Architecture note**: Currently uses egui 0.26 via `eframe`. Migration to `iced` 0.14 is planned. The iced version will use `Application::builder()` with `update`/`view`/`subscription` triplet instead of `eframe::App`. During migration, each `eframe::Panel` becomes an `iced::Column` or `iced::Container`.

### M4L Bridges (`apps/m4l/*`)

**Purpose**: External Max for Live devices that send control commands to plugins and receive status back. These are NOT independent apps — they control existing plugins by sending UDP messages.

| Device | Direction | Scope | Key Controls |
|--------|-----------|-------|-------------|
| `ModurustLite_Instrument.amxd` | Send | All params | NoteOn/NoteOff, param set |
| `ModurustLite_Analyzer.amxd` | Receive | Status only | Status request → parses response |
| `ModurustLite_Modulator.amxd` | Receive | Prime/Mode set | Prime base, density, spread, mode |
| *(planned)* `ModurustLite_Drum.amxd` | Send+Receive | Drum machine | Step triggers, pad control, sample assignment |

All use `std::net::UdpSocket` — zero external dependencies, pure text protocol matching `param <name> <value>`.

**New device scope**: The drum machine M4L device will expose: `pad_trigger <pad_id> <velocity>` (note on via CPU-trigger), `sample_set <pad_id> <name>` (assign sample file name), `mute_toggle <pad_id>`, `swing <value>`, `pattern_step <track> <step> <on/off> <velocity>`, `sequencer_play_or_stop`. Status mirror: current step, pad mute state, sample loaded.

### CLAP Plugin (`clap/`)

**Purpose**: Modern DAW plugin. This is the primary distribution target. CLAP instruments expose synth/sampler/drum modes; CLAP effects process through the core's delay/reverb chain.

**Features per DAW**:
- Standard 16-parameter control surface (same as VST2)
- MIDI input: NoteOn/NoteOff mapped to oscillator pitch
- Optional 4 modes: Synth (osc→filter→env→delay→reverb), Drum (8-sample sequencer), Sampler (load & play samples), FX (post-delay chain)
- UDP listener port 9000 for external control (M4L)
- UDP status broadcaster port 9002 for plugin monitoring
- Parameter smoothing for DAW automation (CIAP/CLAP spec requirement)
- Preset save/load via Serde JSON

**Deployment**: Build with `cargo build -p modurust-clap --release`. The plugin crate re-uses `modurust_core` but wraps it in the CLAP `Plugin` trait from `clap-sys` 0.3. No GUI — editor is provided via the standalone app or a future M4L visualizer.

### VST2 Plugin (`vst/`)

**Purpose**: Universal DAW compatibility. Same functionality as CLAP but for hosts supporting only VST2 (most older DAWs, FL Studio, LMMS). Simpler parameter mapping, slightly fewer modes.

**Key difference from CLAP**:
- No effects mode (VST2 instruments can't typically process post-feedback)
- Parameter smoothing less critical (no CIAP, but basic smoothing for automation)
- Standard 16 parameters, same physical values
- Build: `cargo build -p modurust-vst --release`

---

## Engine Command API

Full enum available at `engine.rs`:
- `NoteOn { note, velocity }`, `NoteOff { note }`, `AllNotesOff`
- `SetPrimeBase(u32)`, `SetPrimePhase(f32)`
- `SetNnSensitivity(f32)`, `SetInstanceDensity(u32)`, `SetInstanceSpread(f32)`
- `SetMode(EngineMode)` — Synth, Sampler, Effect, Controller
- `SetParam(Param, f32)` — all 15 named params
- `GraphAddNode { id, kind }`, `GraphRemoveNode { id }`, `GraphConnect { from, to }`, `GraphDisconnect { from, to }`, `GraphSetParam { id, param, value }`, `GraphTriggerNode { id, gate }`, `GraphClear`

### Status Feed
- **Text feed** on `status_port` (9002): labeled datagrams
- **Binary feed** on `status_port+1..+3` (9003–9005): 31-byte packets

---

## Test Coverage
- 47 tests: 27 core unit + 20 engine comm + 20 standalone
- Voice-pool polyphony verified
- UDP end-to-end tested
- Graph topological sort tested
- Protocol parity (binary vs text) tested
