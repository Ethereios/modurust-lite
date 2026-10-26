# Node System Features

## Overview

Modurust-Lite's DSP core is a library of **15 real-time nodes** processed through a **live-patchable `DspGraph`**. Each node implements the `DspNode` trait (`process(inputs: &[f32], outputs: &mut [f32])`). The graph runs inside `CoreEngine::process_block` after the voice pool renders.

---

## Node Catalogue (implemented)

| Category | Node | Inputs | Outputs | Key Params | Modes |
|----------|------|--------|---------|------------|-------|
| Oscillator | OscillatorNode | — | 1 | waveform, freq | sine/saw/square/triangle |
| Filter | BiquadFilterNode | 1 | 1 | cutoff, resonance, mode | LP/HP/BP/Notch |
| Envelope | AdsrEnvelopeNode | 1 | 1 | attack, decay, sustain, release | idle/attack/decay/sustain/release |
| Noise | NoiseNode | — | 1 | color | white/pink/brown |
| Gain | GainNode | 1 | 1 | gain | — |
| Distortion | DistortionNode | 1 | 1 | drive, tone | tanh + one-pole tone |
| Delay | SimpleDelayNode | 1 | 1 | delay_samples, feedback | fixed-length ring buffer |
| Delay | StereoDelayNode | 1 | 1 | delay_left, delay_right, feedback, cross_feedback | L/R mono-summed |
| Reverb | ReverbNode | 1 | 1 | room_size, damping | Schroeder-style ring delay |
| LFO | LfoNode | — | 1 | freq | sine |
| Mixer | MixerNode | 1 | 1 | gain1, gain2 | mono self-modulation |
| Splitter | SplitterNode | 1 | 1 | — | passthrough |
| Router | RouterNode | N×M | N×M | route gain matrix | N×M routing matrix |
| Sampler | SamplerNode | — | 1 | buffer, playback_rate, trigger | one-shot sample |

### Already Present in `engine.rs`
The following nodes are instantiated directly inside `CoreEngine` and process the voice-pool block: `osc`, `filter`, `env`, `delay`, `reverb`, `master_gain`. The `graph` (if live) routes voice output through user nodes instead.

---

## Planned DSP Expansion

| Node | Description | Priority | Source Pattern |
|------|-------------|----------|----------------|
| StateVariableFilter | 4-mode simultaneous LP/HP/BP/Notch output | Medium | autokit `SvfChannel` bipolar DJ filter (−1 LP kill, +1 HP kill, 0 bypass) |
| LadderFilter | Moog-style 24 dB/octave resonance | High | rust-synth ladder filter research |
| WavetableOscillator | Wave morphing between stored waveforms | Medium | — |
| FMOperator | Modular FM synthesis operator | Low | — |
| ConvolutionReverb | IR-based reverb | Low | — |
| GranularProcessor | Grain cloud | Low | — |
| Compressor | Sidechain + dynamics | Medium | autokit step_smoother dynamics research |
| Gate | Threshold-based mute | Low | — |
| Limiter | Ceiling limiting | Medium | — |
| DrumMachine | Sample playback + 16-step trigger | High | autokit `sampler.rs` (32 voices, MAX_VOICES=32) |

### State-Variable Filter (SVF) — from autokit
Bipolar single-pole SVF per channel. Knob −1 = lowpass kill, 0 = bypass, +1 = highpass kill. Cutoff sweeps 40 Hz–18 kHz. Coefficients computed per-sample so knob sweeps freely without zipper noise. Lives inside `FxBus`, not on the voice path — used as master insert with dry_bypass.

---

## Graph System

### `DspGraph` (`graph.rs`)
- **Arena allocator** — nodes stored in `Vec<Option<Node>>`, holes reused
- **Kahn topological sort** — determines processing order on topology change
- **Cycle detection** — fallback: strongest edge broken to prevent infinite loop
- **Live patching commands** — `GraphAddNode`, `GraphRemoveNode`, `GraphConnect`, `GraphDisconnect`, `GraphSetParam`, `GraphTriggerNode`, `GraphClear`
- **Realtime safe** — topology changes happen on command bus, processed once per block before render

### Engine Modes
| Mode | Behavior |
|------|----------|
| **Synth** | Oscillator → Filter → ADSR → Delay → Reverb (default fixed chain) |
| **Sampler** | Voice pool plays SamplerNode buffers via MIDI notes |
| **Effect** | Bypasses voice pool; audio passes through delay/reverb chain |
| **Controller** | Lightweight: sends commands, no DSP (for M4L mapping) |

---

## Test Coverage

| Area | Status |
|------|--------|
| Oscillator waveform generation | Unit-tested |
| BiquadFilter all modes | Unit-tested |
| ADSR stages | Unit-tested |
| Reverb buffer ring | Unit-tested |
| StereoDelay dry-first | Unit-tested |
| Router routing matrix | Unit-tested |
| Graph topological sort | Integration-tested |
| Voice pool polyphony | Integration-tested |
| Graph text protocol coverage | Integration-tested |
| SetParam path | Integration-tested |

**Total**: 47 tests (27 unit + 20 engine comm). `cargo test --workspace` passes.
