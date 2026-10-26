# Modurust-Lite Roadmap

## What This Project IS

**Modurust-Lite** is a modular DSP audio engine with four frontends, each with a distinct purpose:

| Frontend | Build Command | Role |
|----------|--------------|------|
| Standalone (`apps/standalone`) | `cargo run -p modurust_standalone` | Full DAW-style editor: node graph + drum machine + FX rack, preset save/load |
| CLAP plugin (`clap/`) | `cargo build -p modurust-clap --release` | Primary plugin for modern DAWs (DAW 12+) |
| VST2 plugin (`vst/`) | `cargo build -p modurust-vst --release` | Universal DAW compatibility |
| M4L devices (`apps/m4l/*`) | Copy `.amxd` files into Live Set | External control surface + visual feedback via UDP |

The engine (`modurust_core`) provides 15 DSP nodes. Each frontend **selects** a subset of features — they are not identical wrappers:

| Feature | Standalone | CLAP | VST2 | M4L |
|---------|-----------|------|------|-----|
| Node graph editing | Full drag-drop editor | No (preset only) | No (preset only) | No (send commands) |
| Drum machine | 8-pad kit + 16-step seq | Drum mode presets | Drum mode presets | Drum pattern control |
| Parameter controls | XY pad + 16-param strip | 16 DAW params | 16 DAW params | XY pad + live dials |
| Preset save/load | JSON presets | DAW state chunks | DAW state chunks | External pattern recall |
| MIDI input | Note buttons | Full MIDI | Full MIDI | NoteOn/NoteOff |
| UDP control | Bridge built-in | Listener on 9000 | Listener on 9000 | Sender + receiver |

## Current State: 0.3.0 (2026-01-08 → 2026-10)

### Completed (this cycle)
- Headless modular DSP engine (`modurust_core`) with `DspGraph` (arena + Kahn topo sort), `SuperInstancePool`, `PrimeScheduler`, `ComplexNeuralNetwork`
- 15 DSP node types: Oscillator (saw/sine/square/triangle), BiquadFilter (LP/HP/BP/Notch), ADSR, Noise (white/pink/brown), Gain, Distortion, SimpleDelay, StereoDelay, Mixer, Splitter, Router, Sampler, LFO, Reverb
- UDP binary+text protocol (ports 9000–9005)
- egui standalone with node graph editor, XY morph pad, 16-param strip, piano keys, config hot-reload
- VST2 plugin (`modurust-vst`) and CLAP plugin (`modurust-clap`)
- 3 M4L bridges: Instrument, Analyzer, Modulator
- 47 tests passing
- AGENTS.md, architecture.md, node-system-features.md, ROADMAP.md, frontend-status.md, strategic-roadmap.md

### Research Completed (Oct 2026 deep-dive)
- **autokit** — sample-based drum machine: spectral features (attack/decay/spectral_centroid/sub_energy/high_freq), 8-pad kit, 16-step seq with ConditionTrig (Elektron-style), 16 patterns, 4-bus FX send architecture (DJ filter/SVF, reverb, delay), Volca-style step smoother
- **rudiments** — text pattern format (`track |x-x-|x-x-| 0.5`), rodio SampleSource, track binding unions
- **rust-synth** — FunDSP graph, EngineHandle with pre-allocated slots, genetic evolution (Genome: freq/cutoff/resonance/reverb_mix/pulse_depth/pattern_hits/character), named vibes (BladeRunner/Cathedral/DanceFloor), Euclidean rhythm generation
- **maolan** — iced GUI patterns (Application, subscriptions, update/dispatch)
- **nih-plug / truce / rs-vst-host** — plugin framework patterns

---

## Phase 1 — Plugin Stabilization (✅ In progress)

**Goal**: Production-ready CLAP + VST2 plugins with full parameter support, no drift between binary/text protocols.

| # | Task | Feature Flag |
|---|------|--------------|
| 1 | Fix protocol parity bug: binary `delay_time` uses 3 divisions, text uses 4 | none |
| 2 | Add parameter smoothing per-param (Logarithmic 10ms for gain, Linear 20ms for others) | `smoothing` |
| 3 | Add preset serialization (JSON/Serde) to plugin state chunks | `presets` |
| 4 | VST3 support via truce or nih-plug unified backend | — |
| 5 | Test CLAP plugin in DAW 12+ with MIDI Learn | manual |

**Acceptance Criteria**:
- Binary + text protocol agree on all 16 params (regression test suite)
- Smoothing doesn't glitch audio thread (allocs test passes)
- Presets load from DAW state chunks without panic

---

## Phase 2 — GUI Modernization (iced migration)

**Goal**: Migrate standalone from egui 0.26 to iced 0.14. Tabbed interface for three sections.

### Tab 1 — Modular Synth
- Node graph canvas (drag-drop nodes, cable drawing, click-to-select)
- Node palette (all 15 node types in categorized rows)
- Per-node parameter panel

### Tab 2 — Drum Machine
- 8-pad kit view (sample slot per pad with label, status strip, waveform overview)
- 16-step sequencer grid (16 patterns × 8 tracks) with velocity strip per step
- Condition trig selector (Always / 1:2 / 1:4 / 1:8 / !1:2 / !1:4 / !1:8 / Fill / !Fill)
- Master automation (per-pattern FX sends: reverb mix, delay mix, DJ filter)
- Sample browser with drag-drop import + spectral classification

### Tab 3 — FX Rack
- Per-effect parameter strips (reverb, delay, filter, distortion, gain)
- Master FX strip for all 16 params
- XY morph pad (prime_phase / nn_sensitivity)

**Acceptance Criteria**:
- Tabs switch without state loss
- egui removed entirely from dependency tree
- Peak meter reflects live audio
- Config save/load works from the UI

---

## Phase 3 — Drum Machine Core

**Goal**: A real, sample-based drum machine subsystem in `modurust_core`.

### Features
- `SampleCategory` enum: Kick, Snare, Hihat, Clap, Tom, Perc, Cymbal, Bass, Synth, Other
- `DrumKit` struct: 8 pads, GM-mapped MIDI notes (36+), per-pad pitch/decay/volume/pan/start/end
- `DrumSequencer`: 16 patterns × 8 lanes × 16 steps, each step has velocity/probability/pan/pluck/fx plocks
- `ConditionTrig`: Elektron-style (Always, Every(N), NotEvery(N), Fill, NotFill)
- `EuclideanSequencer`: distribute hits across 16 steps with rotation (from rust-synth)
- `SampleLibrary`: background thread scan, 4.0s duration filter, Arc-shared mono audio data
- Spectral classifier: FFT on sample → classify as kick/snare/hihat/etc.

### Acceptance Criteria
- Drum machine triggers via MIDI notes 36–47 (GM drum map)
- 16-step sequencer plays pattern A → B with active/queued swap
- Probability gate uses per-call RNG (SmallRng seeded deterministically)
- Spectral classification accuracy: 80%+ on test corpus of 100 labeled samples

---

## Phase 4 — M4L Ecosystem

**Goal**: Complete the four M4L devices with full bidirectional control.

### Devices
| Device | Type | Controls |
|--------|------|----------|
| ModurustLite_Instrument.amxd | Send + Receive | 16-param dials, note buttons, mode selector |
| ModurustLite_Analyzer.amxd | Receive | Audio input analysis: peak, spectral centroid, note tracking |
| ModurustLite_Modulator.amxd | Send + Receive | Prime/base sweep, density/spread, NN sensitivity |
| ModurustLite_Drum.amxd | Send + Receive | 8-pad trigger, step sequencer grid, sample assignment, swing, pattern recall |

### Protocol Additions
```
# Drum control (sent from M4L → engine)
drum_pad <pad_id> <velocity>     - Trigger a pad
pad_assign <pad_id> <name>       - Load sample by name
pattern_load <id>                - Switch active pattern
swing <value>                    - 0.0 = straight, 1.0 = swung

# Drum status (engine → M4L)
status:drum <pad_id> <active> <pitch>    - Pad status
step:<pattern>:<track>:<step> <velocity>  - Step state
```

**Acceptance Criteria**:
- All four `.amxd` files load into Ableton Live set without error
- Drum pattern changes sync to transport timeline (no drift at 120 BPM)
- Status reflects within 50ms of audio state change

---

## Phase 5 — Advanced Synthesis (Planned)

| # | Feature | Source |
|---|---------|--------|
| 1 | State-variable filter (4-mode SVF) | autokit fx.rs (SvfChannel) |
| 2 | Ladder filter (Moog-style 24dB/octave) | rust-synth ladder filter patterns |
| 3 | Wavetable oscillator with morphing | rust-synth oscillator research |
| 4 | Convolution reverb (IR loading) | autokit reverb patterns |
| 5 | Granular processor | rust-synth granular research |
| 6 | Compressor / Gate / Limiter | autokit step_smoother + dynamics research |
| 7 | Genetic evolution of presets | rust-synth Genome (freq snaps to golden pentatonic) |
| 8 | Named vibe presets | rust-synth Vibes (BladeRunner, Cathedral, DanceFloor) |

---

## Tracking & Ownership

| Area | Owner | Labels |
|------|-------|--------|
| Core engine | TBD | `engine`, `dsp`, `graph` |
| Standalone GUI | TBD | `gui`, `iced`, `standalone` |
| CLAP plugin | TBD | `plugin`, `clap` |
| VST2 plugin | TBD | `plugin`, `vst` |
| M4L devices | TBD | `m4l`, `ableton` |
| Drum machine | TBD | `drum`, `sequencer`, `samples` |
| Tests | TBD | `test` |

### Build Commands
```bash
cargo check --workspace          # single rustc (jobs=1)
cargo test --workspace           # 47 tests
cargo run -p modurust_standalone # launch GUI
cargo build -p modurust-clap --release
cargo build -p modurust-vst --release
```

### Build Configuration Notes
- `.cargo/config.toml` enforces `jobs = 1` — single rustc process for race-free audio init
- Aliases: `dev`, `devr`, `devc`, `devt`, `devcc`
