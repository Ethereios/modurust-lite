# Strategic Roadmap: Frontend & Plugin Architecture

## Architecture Decision: Four Frontends, One Engine

Modurust-Lite is **not** four identical wrappers. It is **one headless engine** with **four fronteds with distinct purposes**:

| Frontend | Why It Exists | Not Interchangeable Because |
|----------|---------------|----------------------------|
| **Standalone** | Full DAW-style editing environment | Has node graph editor + drum machine UI that plugins lack |
| **CLAP** | Modern DAW integration (portable, open, fast) | DAWs control it via automation, not direct UI |
| **VST2** | Legacy DAW compatibility | Older hosts, simpler parameter set |
| **M4L** | External Max for Live control surface | Bidirectional status feed, pattern editing, drum sequencing |

Each frontend **selects features** from the core — they are NOT interchangeable copies of the same thing.

---

## Source Study Findings (Deep-Dive, October 2026)

### autokit — full source read (`src/engine/`, `src/analysis/`)
- **Spectral features** (`analysis/features.rs`): `attack_time` (start→peak), `decay_time` (peak→−20dB), `spectral_centroid` Hz (low=kick, high=hat), `spectral_flatness` (white=1, tonal=0), `sub_energy_ratio` (<120Hz), `high_freq_ratio` (>4kHz), `peak`, `duration`, `is_percussive` (attack<15ms && decay<2s). Single FFT pass computes all.
- **Sample library** (`analysis/library.rs`): background scan thread, `LibraryCache` skips unchanged files, `MAX_DURATION_SECS = 4.0`, mono f32 Arc data.
- **Kit** (`engine/kit.rs`): `SampleCategory` (Kick, Snare, Hihat, Clap, Tom, Perc, Cymbal, Bass, Synth, Other), `NUM_PADS = 8`, GM MIDI notes 36+.
- **Sequencer** (`engine/sequencer.rs`): `ConditionTrig` {Always, Every(N), NotEvery(N), Fill, NotFill}. Step plocks: velocity/probability/pan/pitch/fx_rvb/fx_dly/fx_filter (None=inherit). Lane per pad (16 steps, muted/solo). PatternBank (16 patterns). MasterAutomation (per-pattern per-step FX). `Pattern::sanitize` clamps all.
- **Master FX** (`engine/fx.rs`): 4-bus: `voices → dry_bypass → dj_filter → master`. True sends to reverb/delay. Bipolar SVF (−1=LP kill, +1=HP kill, 0=bypass). Delay divisions: 1/32, 1/16, 1/8, 1/4 beat.

### rudiments — full source read
- Text pattern: `hi-hat |x-x-|...| 0.5` — instrument, 16-char step string (x/-), optional amplitude [0,1].
- `bind_tracks`: instruments sharing audio file union steps; min amplitude wins.

### rust-synth — full source read
- FunDSP "hacker" graph: typed closure DSP, `Net` rebuilt wholesale on preset swap; cheap for live use.
- `EngineHandle`: pre-allocated slots (MAX_TRACKS=8), `Arc<Mutex<Net>>`, `rebuild_graph()`, Shared atomic state, parking_lot Mutex.
- `Genome` = {freq, cutoff, resonance, reverb_mix, pulse_depth, pattern_hits, pattern_rotation, character}; mutate perturbs + freq snaps to golden pentatonic; crossover mixes genes.
- Vibes: named presets (BladeRunner, Cathedral, DanceFloor) that reconfigure all tracks + global → rebuild graph.

---

## Phase Breakdown

### Phase 1 — Plugin Stabilization (2–4 weeks)
- Protocol parity fix: binary/text agree on all 16 params
- Per-param smoothing (Logarithmic 10ms for gain, Linear 20ms for others)
- Preset serialization in PluginState chunks
- CIAP compliance testing for CLAP
- Verify no audio glitches under stress test (n=10_000 rapid param changes)

### Phase 2 — GUI Modernization (4–6 weeks)
**Migrate standalone from egui 0.26 → iced 0.14**

| Step | Task |
|------|------|
| A | Set up iced 0.14 dependencies, convert `eframe::App` impl to `iced::Application` trait |
| B | Replace side panels with `iced::widget::sidebar`, `Column![]` palette |
| C | Implement iced `canvas::Cache` for node graph (drag-drop, cable visuals) |
| D | Add tab container: **Modular** / **Drum Machine** / **FX Rack** |
| E | XY pad via `canvas` widget with drag interaction |

### Phase 3 — Drum Machine Core (2–3 weeks)
- `DrumKit` with `NUM_PADS = 8`, gm midi map, per-pad sample/pitch/decay/start/end
- `DrumSequencer`: 16 patterns × 8 lanes × 16 steps, velocity/probability/plocks
- `ConditionTrig` implementation (Elektron-style Every/NotEvery/Fill)
- Spectral sample classification from `features.rs`
- `SampleLibrary` with background thread scanning, 4s duration filter

### Phase 4 — M4L Enhancement (2 weeks)
- New device: `ModurustLite_Drum.amxd`
- Commands: `pad_trigger`, `sample_set`, `pattern_load`, `swing`
- Status: pad state, spectral centroid per pad, step position
- Live dial support in modulator device for drum params

### Phase 5 — Advanced Synthesis (2 weeks)
- State-variable filter (autokit `SvfChannel`)
- Ladder filter (Moog-style)
- Genetic evolution of presets (rust-synth `Genome`)

---

## Concrete Feature Matrix by Frontend

| Feature | Standalone | CLAP | VST2 | M4L |
|---------|-----------|------|------|-----|
| Node graph live editing | ✅ | ❌ | ❌ | ❌ |
| XY morph pad | ✅ | ❌ | ❌ | ✅ (mod device dials) |
| 16-param strip | ✅ | ✅ | ✅ | ✅ |
| Drum machine kit | ✅ (Phase 3) | ✅ (drum mode) | ✅ (drum mode) | ✅ (sequencer grid) |
| Sample browser | ✅ (drag-drop) | ❌ | ❌ | ✅ (sample_set cmd) |
| Pattern sequencer | ✅ | ❌ | ❌ | ✅ |
| Preset save/load | ✅ (JSON) | ✅ (DAW chunks) | ✅ (DAW chunks) | ❌ |
| Status display | ✅ (peak, voices) | ❌ | ❌ | ✅ (analyzer display) |

---

## References
- autokit: `src/plugin.rs` for nih-plug pattern, `src/engine/` for synth/seq
- rudiments: `src/pattern.rs` for text format, `src/main.rs` for CLI usage
- rust-synth: `src/audio/engine.rs` for EngineHandle, `src/math/genetic.rs` for mutation
- maolan: `src/gui/view.rs` for iced widget composition, `src/gui/subscriptions.rs` for IPC
- truce: single-codebase multi-format plugin scaffolding