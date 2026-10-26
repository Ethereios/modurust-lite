# Modurust-Lite Frontend Status

## Current Standalone GUI (`apps/standalone`)

**Version**: egui 0.26 via `eframe` 0.26  
**Build**: `cargo run -p modurust_standalone`  
**Status**: Active — see architecture.md § Standalone GUI. Full rewrite to iced 0.14 planned (Phase 2 roadmap).

### What It Does
- Node graph editor: drag-drop palette of 15 DSP nodes, cable connections, click-select
- XY morph pad: `prime_phase` / `nn_sensitivity`
- Global parameter strip: all 16 M4L-addressable params
- Mode selector: Synth / Sampler / Effect / Controller
- Note buttons + All Notes Off
- Save config.json / Load config.json
- Status display: mode, voice count, peak meter
- UDP bridge: binary on 9000, OSC on 9001, status on 9002

### What It Wants To Be (future iced)
- Tabbed interface: **Modular Synth** (node graph), **Drum Machine** (8-pad kit + 16-step seq), **FX Rack** (per-FX strips)
- Sample browser with drag-drop loading
- 16-pattern sequencer with ConditionTrig per-track
- Preset save/load from UI
- Spectral classification of samples

**Note**: The egui path has no drum machine, no FX rack, no named patterns. These are Phase 1+2 additions.

---

## M4L Bridges (`apps/m4l/*`)

**Version**: UDP-only, no external deps  
**Build**: `.amxd` files placed in Ableton Live Set  
**Status**: Active — 3 devices exist.

| Device | Purpose | Key Controls |
|--------|---------|-------------|
| `ModurustLite_Instrument.amxd` | Plays chord progression with arpeggiation | NoteOn/NoteOff, all 16 params |
| `ModurustLite_Analyzer.amxd` | Sweeps NN sensitivity, parses status | Reads status broadcast on 9002 |
| `ModurustLite_Modulator.amxd` | Sweeps prime base/density/spread, parses status | Prime sequence control |

### What They Do
- All use `std::net::UdpSocket` — zero external dependencies
- Communicate via text datagrams (`param <name> <value>`)
- Instrument sends notes, mod sends parameters, analyzer receives status
- No GUI inside Max — all parameters are mapped to M4L dials/sliders

### What's Missing
- Drum machine M4L device (`ModurustLite_Drum.amxd`) — 8-pad kit, sample assign, step grid
- Visual status feedback (current sample, step position, pad mute/solo)
- Note/pitch display in UI

---

## CLAP Plugin (`clap/`)

**Version**: `clap-sys` 0.3 + `modurust-core` backend  
**Build**: `cargo build -p modurust-clap --release`  
**Status**: Available.

### What It Does
- Exposes modurust_core as a CLAP instrument
- 16 parameters mapped to same physical values as VST2
- MIDI input: NoteOn/NoteOff mapped to oscillator pitch
- Mode selection: Synth / Drum / Sampler / FX (via parameter)
- UDP listener on port 9000 for M4L commands
- UDP status broadcaster on port 9002
- Audio output via `CoreEngine::process_block`

### GUI Status
- **No built-in editor**. The plugin has no UI inside the DAW.
- Users configure via the **standalone GUI** and save presets.
- Parameter automation works without editor (CLAP spec).
- A future M4L visualizer could present the graph UI.

### Acceptance Criteria
- Plugin loads in DAW 12+ without error
- All 16 params appear as standard automatable knobs
- MIDI Learn maps note input to oscillator freq
- UDP listener receives text commands without panic

---

## VST2 Plugin (`vst/`)

**Version**: `vst` 0.4 crate  
**Build**: `cargo build -p modurust-vst --release`  
**Status**: Available.

### What It Does
- Exposes modurust_core as a VST2 instrument
- 16 parameters mapped to same physical values as CLAP
- MIDI input: NoteOn/NoteOff mapped to oscillator pitch
- Audio output via `CoreEngine::process_block`
- UDP listener on port 9000, status on 9002
- **No effects mode** — VST2 instruments cannot process post-feedback; use delay/reverb as insert effects

### GUI Status
- **No built-in editor**. Same as CLAP — configure via standalone GUI, save presets.
- Parameter smoothing less critical than CLAP (no CIAP requirement), but basic smoothing added for automation lane continuity.

---

## Protocol Summary (UDP 9000–9005)

| Port | Protocol | Direction | Description |
|------|----------|-----------|-------------|
| 9000 | Binary + text fallback | Send+Receive | NoteOn/NoteOff, SetParam, SetMode, Graph commands |
| 9001 | OSC fallback (`/note_on`, etc.) | Receive | Max/MSP/Cycling '74 legacy |
| 9002 | Text status feed | Receive | Labeled datagrams: mode, voices, peak meter |
| 9003–9005 | Binary status packets | Receive | 31-byte packets: voice peak, env state, filter state, NN output |

**Binary packet format** (31 bytes):
```
byte 0–3: voice_peak_f32 (amplitude)
byte 4–7: env_stage_i32 (0=Idle, 1=Attack, 2=Decay, 3=Sustain, 4=Release)
byte 8–11: filter_cutoff_i32 (Hz × 1000, i.e. 1200 → 0x04B0)
byte 12–15: nn_output_f32 (norm. value -1..1)
byte 16–19: nn_phase_f32 (0..1)
byte 20–23: nn_sensitivity_f32 (0..1)
byte 24–27: prime_phase_f32 (0..1)
byte 28–30: reserved / unused (zero)
```