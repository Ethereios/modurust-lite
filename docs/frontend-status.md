# Frontend Status

## Overview

Modurust-Lite is a **headless modular DSP audio engine**. Frontends are optional GUI/interface layers that expose the core engine's functionality.

**This is NOT a DAW** - there is no timeline, arrangement view, clip editing, or automation.

## Frontend Summary

| Frontend | Status | Purpose |
|----------|--------|---------|
| **Standalone GUI** | ✅ Working | Desktop use with egui |
| **M4L Bridges** | ✅ Working | Control via Max for Live in Ableton |
| **VST2 Plugin** | ✅ Working | Instrument in any DAW |

## Standalone GUI (`apps/standalone`)

### Status: Working

- [x] Config hot-reload (config.json)
- [x] Audio device selection
- [x] Start/Stop audio
- [x] Note buttons (C2-C5 + All Notes Off)
- [x] XY morph pad (prime_phase / nn_sensitivity)
- [x] Parameter display
- [x] Peak meter
- [x] Voice count display

### Build
```bash
cargo run -p Modurust-standalone
```

### Dependencies
- `eframe` 0.26 (egui)
- `cpal` 0.15 (audio)
- `notify` (config watching)

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

## What Frontends Do NOT Provide

### Not Implemented
- [ ] VST3 support
- [ ] VST editor GUI
- [ ] Preset storage/recall
- [ ] MIDI Learn
- [ ] OSC control
- [ ] Node graph editor UI

### Never Planned
- Timeline / arrangement view
- Clip launching / sequencing
- Audio recording
- Automation lanes
- DAW-style mixing console

## Architecture

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
