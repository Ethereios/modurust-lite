# Modurust-Lite — Honest Current State Review (Oct 2026)

This document is a brutal, no-spin inventory of what actually works, what's broken, and what's missing. Written for the long-haul development timeline (months), not a sprint.

---

## 1. What Actually Works — Engine Core (Solid)

The headless `modurust_core` crate is the strongest part of the codebase. It compiles, all 57 tests pass, and the architecture is sound.

### DSP Nodes (16 total)

| Node | File | Status | Notes |
|------|------|--------|-------|
| `OscillatorNode` (Sine/Saw/Square/Triangle) | `dsp/mod.rs:17` | Works | Phase-accumulator, no anti-aliasing |
| `BiquadFilterNode` (LP/HP/BP/Notch) | `dsp/mod.rs:59` | Works | RBJ cookbook coefficients |
| `AdsrEnvelopeNode` (A/D/S/R) | `dsp/mod.rs:~170` | Works | Linear segments |
| `NoiseNode` (White/Pink/Brown) | `dsp/extra_nodes.rs` | Works | Per-sample generation |
| `LfoNode` (Sine) | `dsp/extra_nodes.rs:3` | Works | Audio-rate only, no CV |
| `GainNode` | `dsp/extra_nodes.rs` | Works | Simple scalar |
| `DistortionNode` (Soft clip) | `dsp/extra_nodes.rs` | Works | Tanh |
| `ReverbNode` | `dsp/extra_nodes.rs:25` | Works | Schroeder allpass+comb, fixed buffer |
| `SimpleDelayNode` | `dsp/mod.rs:~200` | Works | Single-tap, feedback |
| `StereoDelayNode` | `dsp/extra_nodes.rs:170` | Works | L/R independent + cross-feedback |
| `SamplerNode` | `dsp/extra_nodes.rs` | Exists | **No file loading, no pitch tracking** |
| `MixerNode` | `dsp/extra_nodes.rs:58` | Works | Two gains, graph impl is self-mix |
| `SplitterNode` | `dsp/extra_nodes.rs:97` | Works | Passthrough, fan-out by topology |
| `RouterNode` | `dsp/extra_nodes.rs:127` | Works | **Matrix mixer, but only via `process_block`** (not `DspNode` trait) |
| `FilterMode` enum | `dsp/mod.rs:52` | Works | LP/HP/BP/Notch |
| `Waveform` enum | `dsp/mod.rs:10` | Works | Sine/Saw/Square/Triangle |

### Core Engine Systems (All Functional)

| System | Description | File |
|--------|-------------|------|
| **Voice Pool** (`SuperInstancePool`) | Polyphonic rendering, density→capacity, spread→detune, stealing | `instance/mod.rs` |
| **Prime Scheduler** | Prime-gated pattern buffers, base value, phase | `prime/mod.rs` |
| **Complex Neural Network** | Complex-valued, modReLU/cardioid activations, NnMode enum | `nn/mod.rs` |
| **Parameter Smoothing** | Per-param exponential smoothing (10-20ms), discrete params immediate | `engine.rs:86` |
| **DSP Graph** | Arena-based, Kahn topo sort, cycle fallback, zero-alloc steady state | `dsp/graph.rs` |
| **Command Bus** | Crossbeam channel, `EngineCommand` enum (19 variants) | `engine.rs:139` |
| **Config Hot-Reload** | `EngineConfig` + `ConfigWatcher` (notify) | `config.rs` |
| **Audio I/O** | `CpalEngineHost` wraps cpal 0.15 stream | `io/audio.rs` |
| **MIDI I/O** | `MidiManager` (midir) | `io/midi.rs` |
| **UDP/OSC Bridge** | Binary (9000) + OSC (9001) + status (9002) | `comm/mod.rs` |

### Parameter Surface (16 M4L-addressable)

| Index | Param | Type | Range | Smoothed |
|-------|-------|------|-------|----------|
| 0 | FilterCutoff | f32 | 20–20k Hz | ✓ |
| 1 | FilterResonance | f32 | 0–1 | ✓ |
| 2 | FilterMode | u32 | 0–3 | ✗ |
| 3 | ReverbMix | f32 | 0–1 | ✓ |
| 4 | DelayTime | f32 | 0–1 (norm) | ✓ |
| 5 | DelayFeedback | f32 | 0–1 | ✓ |
| 6 | EnvAttack | f32 | 0–1 (norm) | ✓ |
| 7 | EnvRelease | f32 | 0–1 (norm) | ✓ |
| 8 | Waveform | u32 | 0–3 | ✗ |
| 9 | MasterGain | f32 | 0–1 | ✓ |
| — | PrimeBase | u32 | 1+ | — |
| — | PrimePhase | f32 | 0–1 | — |
| — | NnSensitivity | f32 | 0–1 | — |
| — | InstanceDensity | u32 | 1–16 | — |
| — | InstanceSpread | f32 | 0–1 | — |

---

## 2. What's Broken / Incomplete — Engine Core

### Missing Modular/Subtractive Synthesis Wiring

| Gap | Impact |
|-----|--------|
| **Voice → Graph routing not connected** | `CoreEngine::process_block` doesn't feed voice output through `DspGraph`. Nodes can be added via `GraphAddNode` but audio never flows through them. |
| **No modulation matrix** | LFO/Envelope output can't modulate filter cutoff, pitch, etc. `GraphSetParam` is static value, not per-sample CV. |
| **No polyphonic graph instances** | One graph for all voices. True modular needs per-voice graph or per-voice node instances. |
| **`RouterNode` not in `DspNode` trait** | Matrix mixer only works via `process_block` directly, not in graph topology. |
| **`SamplerNode` has no file I/O** | No sample loading, no key/pitch mapping, no loop/slice support. |
| **No anti-aliased oscillators** | Naive phase accumulation → aliasing at high notes. |

### Missing Synthesis Features (Standard in Subtractive Synths)

- Pulse-width modulation (PWM)
- Filter FM / oscillator sync / cross-modulation
- Unison/detune per voice (spread exists but not wired to multiple oscillators)
- Ring modulation / AM
- Envelope follower / audio-rate modulation
- Wavefolder / distortion types beyond tanh

---

## 3. What Works — Plugin Frontends (Compile, No GUI, Limited Modes)

### VST2 (`modurust-vst`, `vst` 0.4 crate)

- **Compiles**: ✓
- **Parameters exposed**: 16 (same as above) via `PluginParameters`
- **MIDI input**: NoteOn/Off → oscillator pitch (monophonic, single voice)
- **Audio**: `CoreEngine::process_block` runs
- **UDP listener**: Port 9000 for M4L text commands
- **Status broadcast**: Port 9002
- **Missing**: 
  - No effects mode (VST2 instruments can't process post-feedback reliably)
  - No drum/sampler modes — only Synth path (osc→filter→env→delay→reverb)
  - No GUI — DAW shows generic parameter list only
  - Parameter smoothing uses basic `AtomicFloat`, not core's `Smoothers`

### CLAP (`modurust-clap`, `clap-sys` 0.3)

- **Compiles**: ✓
- **Parameters**: 16, same mapping, with CLAP automation smoothing
- **Modes declared**: 4 (Synth, Drum, Sampler, FX) — but **not implemented in code**
- **MIDI**: NoteOn/Off mapped to pitch
- **UDP**: Port 9000 listener, 9002 status
- **Missing**:
  - Mode switching not implemented in `process` — always runs Synth path
  - Drum/Sampler/FX mode logic absent
  - No GUI — no CLAP editor extension
  - Preset serialization stubbed but not working

---

## 4. What Works — M4L Bridges (Active, Minimal)

| Device | File | Sends | Receives | Status |
|--------|------|-------|----------|--------|
| `ModurustLite_Instrument.amxd` | `apps/m4l/instrument` | NoteOn/Off, all 16 params | — | Works |
| `ModurustLite_Analyzer.amxd` | `apps/m4l/analyzer` | — | Status (9002) | Works |
| `ModurustLite_Modulator.amxd` | `apps/m4l/modulator` | Prime base/density/spread/mode | Status | Works |
| `ModurustLite_Drum.amxd` | **PLANNED** | Pad trigger, step grid, sample assign | Step position, pad state | **NOT BUILT** |

**Protocol**: Raw UDP text datagrams (`param filter_cutoff 0.7`, `note_on 60 0.8`). No OSC in practice — binary/OSC ports open but M4L uses text.

---

## 5. Standalone GUI — Actual State (iced 0.12)

### Compiles: ✓ (no warnings)
### Runs: Not tested (audio device required)

### Three Tabs — What Actually Renders

| Tab | Code Location | Actual UI Elements | Real Functionality |
|-----|---------------|-------------------|-------------------|
| **Modular** | `modular_graph.rs` | Two buttons: "Osc", "Filter" | Clicking adds node to graph + sends `GraphAddNode`; **no cables, no param panel, no visual graph, no signal preview** |
| **Drum Machine** | `drum_machine.rs` | Play/Stop, tempo slider, 16 step buttons, 8 pad labels | Step toggles stored in memory; Tick advances every other step and fires NoteOn; **no pattern bank, no sample assign, no probability, no swing, no plocks** |
| **FX Rack** | `fx_rack.rs` | 5 sliders (filter cutoff/res, reverb, delay, env, master) | Sliders update local state + send `SetParam`; **no FX chain routing, no insert order, no per-FX bypass, no visual signal flow** |

### Missing in Standalone (Critical for DAW-style Editor)

| Feature | Current | Needed |
|---------|---------|--------|
| **Node graph editor** | Two buttons | Drag-create from palette, click-drag cables, click node → param panel, delete nodes, zoom/pan canvas |
| **Pattern sequencer** | 16-step row | 16 patterns × 8 tracks, ConditionTrig (Every/N/Fill), per-step plocks (vel/prob/pan/pitch/FX), pattern queue |
| **Sample browser** | None | Drag-drop `.wav`, preview, assign to pad/sampler node, spectral classification |
| **Preset system** | Save/Load buttons (no-op) | JSON serialize: full graph + drum patterns + FX chain + engine config |
| **Signal visualizers** | Peak meter only | Oscilloscope, spectrum, voice activity per channel |
| **XY Morph Pad** | None | `prime_phase` vs `nn_sensitivity` pad with recording |
| **Global parameter strip** | Spread across tabs | Unified 16-param strip visible in all tabs |

---

## 6. Reference Repos — Patterns Available

Copied to `reference_repos/`:

| Repo | Key Patterns Extracted |
|------|------------------------|
| **autokit** (drum machine) | `DrumPad`/`DrumKit` with categories, `LibraryCache` for sample scan, `ConditionTrig` (Every/N/NotEvery/Fill), `Lane`/`Step`/`Pattern`/`PatternBank` (16 patterns), `MasterAutomation` per-pattern FX |
| **maolan** | Track-based DAW engine, pattern grid |
| **rust-synth** (FunDSP) | Closure-based graph `Net`, wholesale rebuild on preset swap, `EngineHandle` with pre-allocated tracks, `Genome` evolution (mutation/crossover) |
| **rudiments** | Minimal DSP building blocks |
| **rs-vst-host** | Plugin hosting patterns |
| **rust-audio-research** | Research prototypes |

---

## 7. Gaps Per Frontend — What Must Be Built

### Standalone GUI (Priority: High — Only Place for Graph Editing)

1. **Modular tab: Full node graph editor**
   - Palette of 16 node types (from `GraphNodeKind`)
   - Visual cable connections (output → input)
   - Click node → parameter panel (uses `GraphSetParam`)
   - Canvas zoom/pan, multi-select, delete, undo/redo
   - Graph state saved in preset

2. **Drum Machine tab: Complete 8-pad + 16-step sequencer**
   - Sample assignment per pad (drag-drop from browser)
   - 16 patterns, ConditionTrig per track
   - Per-step plocks: velocity, probability, pan, pitch, FX sends
   - Pattern queue (active/queued), copy/paste pattern
   - Swing per pattern, fill trigger

3. **FX Rack tab: Insert chain editor**
   - Drag FX units into ordered chain (filter → reverb → delay → etc.)
   - Per-FX param panel, bypass, reorder, copy/paste
   - Send/return routing (dry + FX buses)
   - DJ filter (LP kill / HP kill / bypass) on master

4. **Cross-tab shared components**
   - Global 16-param strip (always visible)
   - XY morph pad (prime_phase vs nn_sensitivity)
   - Preset save/load dialog
   - Transport (play/stop/bpm if internal clock)

### VST Plugin (Priority: Medium — Most DAW Users)

1. Implement 4 modes in `process`:
   - **Synth**: Voice → osc → filter → env → delay → reverb
   - **Drum**: 8-sample sequencer (needs sampler node + pattern engine)
   - **Sampler**: Load sample, pitch-track across keyboard
   - **FX**: Audio in → delay → reverb (no voice pool)

2. Add CLAP-style parameter smoothing to VST automation

3. Consider VST3 migration (VST2 deprecated)

### CLAP Plugin (Priority: Medium — Modern Target)

1. Implement 4 modes (same as VST)
2. Add `clap::gui` extension for future editor
3. Implement `clap::audio_ports` for proper FX mode (stereo in/out)
4. Preset save/load via CLAP state API

### M4L Devices (Priority: Medium — Ableton Integration)

1. **Build `ModurustLite_Drum.amxd`**: 8-pad grid, step sequencer, sample assign dials
2. Add visual status feedback (current step LED, pad mute/solo, sample name display)
3. Consider Max patcher integration for graph view (send graph JSON via UDP)

---

## 8. Architecture Decision: How Frontends Share Modules

### Current Problem

`apps/standalone/src/main.rs` owns everything: `ModularGraph`, `DrumMachine`, `FxRack` as structs with `view(&self, &StandaloneApp)` — tightly coupled to the audio host. Plugins have their own separate logic (or none). M4L is separate UDP only.

### Cleaner Target Architecture

```
modurust_core (headless engine)
    │
    ├── engine_adapter.rs  ── Arc<Mutex<CoreEngine>> + send_cmd() helpers
    │
    ├── standalone/
    │   ├── app.rs         ── iced Application, owns host, runs Tick
    │   ├── modular_view.rs   ── ModularGraph state + view(EngineAdapter)
    │   ├── drum_view.rs      ── DrumMachine state + view(EngineAdapter)
    │   ├── fx_view.rs        ── FxRack state + view(EngineAdapter)
    │   ├── preset.rs         ── Serialize full state
    │   └── components/       ── Shared widgets (param_slider, xy_pad, etc.)
    │
    ├── vst/
    │   └── lib.rs           ── Plugin trait, binds CoreEngine directly, 16 params
    │
    ├── clap/
    │   └── lib.rs           ── CLAP trait, 4 modes, automation smoothing
    │
    └── m4l/
        ├── instrument/
        ├── analyzer/
        ├── modulator/
        └── drum/            ── New: 8-pad + step grid + sample assign
```

**Key principle**: Each UI component (`ModularGraph`, `DrumMachine`, `FxRack`) should take `&EngineAdapter` in its `view()` and `update()` — not `&StandaloneApp`. This makes them portable across frontends.

---

## 9. Development Order (My Recommendation)

| Phase | Work | Why |
|-------|------|-----|
| **0** | Write `EngineAdapter` + extract UI components to take it | Decouples GUI from host; enables parallel work |
| **1** | Modular tab: graph editor (palette, cables, param panel) | Core differentiator; only standalone does this |
| **2** | Drum tab: sample browser + assign + 16-pattern sequencer | Reference repo `autokit` has exact patterns |
| **3** | FX tab: insert chain + send/return + DJ filter | Completes DAW-style editor |
| **4** | Preset system (JSON) across all three tabs | Persist work, share between standalone + plugins |
| **5** | VST/CLAP mode implementations (Synth/Drum/Sampler/FX) | Make plugins actually usable |
| **6** | M4L Drum device + status visual feedback | Close Ableton integration loop |

---

## 10. File Inventory — What Exists vs What's Missing

### Core (Complete)
- `core/src/lib.rs` ✓
- `core/src/config.rs` ✓
- `core/src/engine.rs` ✓ (engine, commands, smoothing, modes)
- `core/src/dsp/mod.rs` ✓ (osc, filter, env, noise, delay)
- `core/src/dsp/extra_nodes.rs` ✓ (lfo, reverb, mixer, splitter, router, stereo_delay, sampler, gain, distortion)
- `core/src/dsp/graph.rs` ✓ (arena graph, topo sort)
- `core/src/dsp/smoother.rs` ✓
- `core/src/instance/mod.rs` ✓ (voice pool)
- `core/src/prime/mod.rs` ✓
- `core/src/nn/mod.rs` ✓
- `core/src/io/audio.rs` ✓ (CpalEngineHost)
- `core/src/io/midi.rs` ✓
- `core/src/comm/mod.rs` ✓ (UdpOscBridge)

### Standalone (Compiles, Stubs Only)
- `apps/standalone/src/main.rs` — App shell, tab switching, engine wiring ✓
- `apps/standalone/src/modular_graph.rs` — **STUB**: 2 buttons, no graph editor
- `apps/standalone/src/drum_machine.rs` — **STUB**: step buttons, no patterns/samples
- `apps/standalone/src/fx_rack.rs` — **STUB**: 5 sliders, no chain/bypass

### Plugins (Compile, Limited)
- `vst/src/lib.rs` — VST2 wrapper, 16 params, 1 mode only
- `clap/src/lib.rs` — CLAP wrapper, 4 modes declared, 1 implemented

### M4L (3 of 4 Built)
- `apps/m4l/instrument/src/main.rs` ✓
- `apps/m4l/analyzer/src/main.rs` ✓
- `apps/m4l/modulator/src/main.rs` ✓
- `apps/m4l/drum/` — **MISSING**

### Docs (Updated)
- `docs/architecture.md` ✓ (needs this review appended)
- `docs/frontend-status.md` ✓ (needs this review appended)
- `docs/strategic-roadmap.md` ✓
- `docs/node-system-features.md` ✓

---

## 11. Next Concrete Steps (Not "Build Everything")

1. **Create `EngineAdapter` struct** in `core/src/engine_adapter.rs` or `standalone/src/engine_adapter.rs`
   ```rust
   pub struct EngineAdapter { engine: Arc<Mutex<CoreEngine>> }
   impl EngineAdapter { pub fn send(&self, cmd: impl Into<EngineCommand>) { ... } }
   ```

2. **Change `modular_graph.rs`, `drum_machine.rs`, `fx_rack.rs`** to:
   - Remove `view(&self, &StandaloneApp)`
   - Add `view(&self, &EngineAdapter) -> Element<Message>`
   - Update `main.rs` to pass adapter

3. **Build Modular graph editor** using reference patterns (node palette → drag → canvas → cables)

4. **Build Drum sample browser** using `autokit`'s `LibraryCache` + `SampleCategory` patterns

This review is the baseline. Every item above is verifiable in the code. Nothing is "done" that isn't marked ✓.

---

## 12. Timeline & Goals

### Phase 0 — Engineering Hygiene (Weeks 1–2)

- Extract `EngineAdapter` from standalone app
- Refactor all UI components to take `&EngineAdapter` (not `&StandaloneApp`)
- Run `cargo check --workspace` and `cargo test --workspace` clean

### Phase 1 — Modular Tab (Weeks 3–10)

- Node palette (16 nodes) → drag → canvas
- Cable drawing (output → input), click node → params
- Graph rendering via `DspGraph` state
- Signal preview (osc → filter → output)

### Phase 2 — Drum Tab (Weeks 11–18)

- Sample browser (autokit `LibraryCache` pattern)
- Assign samples to pads (sampler nodes)
- 16-pattern sequencer with ConditionTrig
- Per-step plocks

### Phase 3 — FX Tab (Weeks 19–24)

- Insert chain (drag, reorder, bypass)
- Send/return buses
- DJ filter on master

### Phase 4 — Presets (Weeks 25–28)

- Serialize full state: graph + patterns + FX chain + engine config
- JSON presets

### Phase 5 — Plugins (Weeks 29–38)

- VST: implement 4 modes (Synth/Drum/Sampler/FX)
- CLAP: implement 4 modes (Synth/Drum/Sampler/FX)
- VST3 consideration

### Phase 6 — M4L (Weeks 39–46)

- Build drum device
- Visual status feedback

### Final Goal (Months 3–4)

- Fully working modular subtractive + drum + FX DAW-style standalone
- Plugins fully functional across 4 modes
- M4L full integration
- All tests pass, CI green

---

*End of review.*