# Frontend Status

## Overview

Modurust-Lite is a **headless modular DSP audio engine**. Frontends are optional GUI/interface layers that expose the core engine's functionality.

## Frontend Summary

| Frontend | Status | Purpose |
|----------|--------|---------|
| **Standalone GUI** | ✅ Working | Desktop use with egui |
| **M4L Bridges** | ✅ Working | Control via Max for Live in Ableton |
| **CLAP Plugin** | ✅ Working | Primary plugin format for modern DAWs |
| **VST2 Plugin** | ✅ Working | Instrument in any DAW |

## Overview

Modurust-Lite is a **headless modular DSP audio engine**. Frontends are optional GUI/interface layers that expose the core engine's functionality.

Modurust-Lite focuses on being a modular DSP audio engine rather than a full DAW, so it does not include timeline/arrangement views, clip launching/sequencing, audio recording, or automation lanes.

## Standalone GUI (`apps/standalone`)

### Status: Working (visual node editor + full control surface)

- [x] Config hot-reload (config.json) via `ConfigWatcher` (notify)
- [x] Start/Stop audio (▶ Start Audio button, cpal `CpalEngineHost`)
- [x] **Visual node graph editor** — drag/drop, cable connections, click-to-select, palette of all 15 DSP node types
- [x] **Node palette** with sources/filters/effects/utility groups + quick templates (Synth Chain, Delay Effect)
- [x] **Per-node parameter editor** — Oscillator freq/waveform, Filter cutoff/resonance/mode, Envelope ADSR, Delay time/feedback, Reverb room/damping, Distortion drive/tone, Gain
- [x] **XY morph pad** (prime_phase × nn_sensitivity) with live visual feedback
- [x] **Global parameter strip** — all 16 M4L-addressable params (filter cutoff/resonance/mode, reverb mix, delay time/feedback, env attack/release, waveform, master gain)
- [x] **Mode selector** (Synth / Sampler / Effect / Controller) — live pushes to engine
- [x] **Note buttons** (C1–C6 chromatically) + All Notes Off
- [x] **Save / Load config** buttons writing `config.json` directly
- [x] **Status display** — mode, voice count, peak meter (color-coded green/yellow/red)
- [x] **UDP bridge** — UdpOscBridge wired up at startup (binary UDP + OSC + status broadcast)

### Build
```bash
cargo run -p modurust_standalone
```

### Dependencies
- `eframe` 0.26 (egui)
- `cpal` 0.15 (audio)
- `notify` (config watching, via core)
- `crossbeam-channel` (via core)

## Max for Live Bridges (`apps/m4l/*`)

### Status: Working

Three M4L devices provide Ableton Live integration:

#### ModurustLite_Instrument.amxd
- [x] Note on/off triggering
- [x] Prime base control
- [x] Prime phase control
- [x] Waveform selection
- [x] Filter cutoff/resonance
- [x] Reverb/delay mix
- [x] ADSR envelope
- [x] Master gain

#### ModurustLite_Analyzer.amxd
- [x] NN sensitivity sweep
- [x] Status packet parsing
- [x] Mode display
- [x] Peak meter
- [x] Voice count

#### ModurustLite_Modulator.amxd
- [x] Prime base sweep
- [x] Prime density control
- [x] Instance spread control
- [x] Status parsing

### Communication
- Send: UDP text commands to port 9000
- Receive: Status on port 9002
- Protocol: Text format (`param cutoff 0.5`, `note 60 100`)

### Dependencies
- None (uses `std::net::UdpSocket` only)

## CLAP Plugin (`clap/`)

### Status: Working

- [x] CLAP instrument/audio-effect plugin
- [x] 16 parameters exposed
- [x] MIDI input (NoteOn/NoteOff)
- [x] Audio output
- [x] UDP control listener (port 9000)
- [x] UDP status broadcaster (port 9002)

### Parameters (16)

| Index | Parameter | Range |
|-------|-----------|-------|
| 0 | Filter Cutoff | 20-20000 Hz |
| 1 | Filter Resonance | 0-1 |
| 2 | Filter Mode | LP/HP/BP/Notch |
| 3 | Reverb Mix | 0-1 |
| 4 | Delay Time | 0-1 |
| 5 | Delay Feedback | 0-1 |
| 6 | Env Attack | 0-1 |
| 7 | Env Release | 0-1 |
| 8 | Waveform | Sine/Saw/Square/Tri |
| 9 | Master Gain | 0-1 |
| 10 | Prime Base | 1-100 |
| 11 | Prime Phase | 0-1 |
| 12 | NN Sensitivity | 0-1 |
| 13 | Instance Density | 1-16 |
| 14 | Instance Spread | 0-1 |
| 15 | Engine Mode | Basic/Prime/NN/Hybrid |

### Build
```bash
cargo build -p modurust-clap --release
```

Output: `target/release/modurustclap.dll`

### Dependencies
- `clap-sys` 0.3 (CLAP API)
- `crossbeam-channel`
- `modurust_core`

## VST2 Plugin (`vst/`)

### Status: Working

- [x] VST2 instrument plugin
- [x] 16 parameters exposed
- [x] MIDI input (NoteOn/NoteOff)
- [x] Audio output
- [x] UDP control listener (port 9000)
- [x] UDP status broadcaster (port 9002)

### Parameters (16)
| Index | Parameter | Range |
|-------|-----------|-------|
| 0 | Filter Cutoff | 20-20000 Hz |
| 1 | Filter Resonance | 0-1 |
| 2 | Filter Mode | LP/HP/BP/Notch |
| 3 | Reverb Mix | 0-1 |
| 4 | Delay Time | 0-1 |
| 5 | Delay Feedback | 0-1 |
| 6 | Env Attack | 0-1 |
| 7 | Env Release | 0-1 |
| 8 | Waveform | Sine/Saw/Square/Tri |
| 9 | Master Gain | 0-1 |
| 10 | Prime Base | 1-100 |
| 11 | Prime Phase | 0-1 |
| 12 | NN Sensitivity | 0-1 |
| 13 | Instance Density | 1-16 |
| 14 | Instance Spread | 0-1 |
| 15 | Engine Mode | Basic/Prime/NN/Hybrid |

### Build
```bash
cargo build -p modurust-vst --release
```

Output: `target/release/modurustvst.dll`

### Dependencies
- `vst` 0.4 (VST2 API)
- `crossbeam-channel`
- `modurust_core`

## Future Frontend Features to Implement

### Not Yet Implemented (Planned):
- [ ] VST3 support (modern DAW compatibility)
- [ ] Plugin editor GUI (parameter editing interface)
- [ ] Preset storage/recall (save/load plugin states)
- [ ] MIDI Learn (map parameters to MIDI controllers)
- [ ] OSC control (alternative to UDP binary protocol)

### Architecture

```
                    ┌─────────────────┐
                    │  Modurust Core  │
                    │  (DSP Engine)   │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
   ┌────▼─────┐       ┌─────▼──────┐      ┌─────▼─────┐
   │ Standalone│       │ M4L Bridges│      │ VST Plugin │
   │   egui   │       │   UDP      │      │  UDP       │
   │  GUI     │       │   Port 9000│      │  Port 9000 │
   └──────────┘       └────────────┘      └────────────┘
```

All frontends communicate with the same core engine via:
- EngineCommand messages
- UDP text protocol (port 9000)
- UDP status feed (port 9002)
