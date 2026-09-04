# Node System Features

## Overview

Modurust-Lite is a **modular DSP audio engine**. Nodes are connected in a graph that processes audio from input to output.

## Implemented Node Types

### Oscillators
- [x] **Oscillator** - Sine, saw, square, triangle waveforms
- [ ] Wavetable oscillator (planned)
- [ ] FM synthesis (planned)
- [ ] Physical modeling / Karplus-Strong (planned)

### Filters
- [x] **BiquadFilter** - Low-pass, High-pass, Band-pass, Notch modes
- [ ] State-variable filter (planned - 4-mode simultaneous output)
- [ ] Ladder filter (planned - Moog-style resonance)
- [ ] Formant filter (planned - vocal synthesis)
- [ ] FFT-based spectral filter (planned)

### Envelopes & Dynamics
- [x] **ADSR** - Attack, Decay, Sustain, Release with per-voice auto-release
- [ ] Compressor (planned)
- [ ] Gate (planned)
- [ ] Limiter (planned)

### Effects
- [x] **SimpleDelay** - Basic delay effect
- [x] **Reverb** - Simple reverb effect
- [x] **StereoDelay** - Stereo delay with L/R channels
- [x] **Distortion** - Waveshaping distortion
- [ ] Convolution reverb (planned)
- [ ] Chorus / Phaser / Flanger (planned)
- [ ] Granular synthesis (planned)

### Sources
- [x] **Noise** - White, pink, brown noise generators
- [x] **Sampler** - Sample playback node

### Utility
- [x] **Gain** - Volume control
- [x] **LFO** - Low-frequency oscillator for modulation
- [x] **Mixer** - Multi-input mixing
- [x] **Splitter** - Single input to multiple outputs
- [x] **Router** - N×M routing matrix

### Neural Network
- [x] **ComplexNeuralNetwork** - Complex-valued NN with modReLU/cardioid activations
- [ ] ONNX model loading (planned)

## Graph System

- [x] Arena allocator for node storage
- [x] Kahn topological sort for processing order
- [x] Cycle detection with fallback (for feedback)
- [x] Live graph patching commands (text protocol)

## Node Control

### Parameters
| Parameter | Type | Range | Description |
|-----------|------|-------|-------------|
| filter_cutoff | f32 | 20-20000 Hz | Filter cutoff frequency |
| filter_resonance | f32 | 0-1 | Filter resonance/Q |
| filter_mode | u32 | 0-3 | LP/HP/BP/Notch |
| reverb_mix | f32 | 0-1 | Reverb wet/dry mix |
| delay_time | f32 | 0-1 | Delay time (normalized) |
| delay_feedback | f32 | 0-1 | Delay feedback amount |
| env_attack | f32 | 0-1 | ADSR attack time |
| env_release | f32 | 0-1 | ADSR release time |
| waveform | u32 | 0-3 | Sine/Saw/Square/Triangle |
| master_gain | f32 | 0-1 | Master output volume |

### Prime/Neural Control
| Parameter | Type | Range | Description |
|-----------|------|-------|-------------|
| prime_base | u32 | 1+ | Prime sequence base number |
| prime_phase | f32 | 0-1 | Prime sequence phase |
| nn_sensitivity | f32 | 0-1 | Neural network influence |
| instance_density | u32 | 1-16 | Voice pool capacity |
| instance_spread | f32 | 0-1 | Voice detune spread |

## Engine Modes

- [x] **Basic** - Simple oscillator through filter
- [x] **Prime** - Prime-sequenced gate patterns
- [x] **NN** - Neural network modulation
- [x] **Hybrid** - Combined prime + NN control

## Future Node Features to Implement

### Audio I/O Nodes (engine currently uses cpal directly):
- Audio input recording (input node)
- Audio file import (sampler with file support)
- Audio export (output node)

### MIDI Piano Roll / Sequencing:
- MIDI note recording (MIDI piano roll)
- Step sequencer (per-note timing grid)
- Arpeggiator (internally instead of M4L-only)

### Visual Editor Enhancements:
- Node graph UI for standalone (replacing XY pad)
- Cable patching visualization
- Oscilloscope/spectrum analyzer UI

### Additional Filter/Effect Types:
- State-variable filter (4-mode simultaneous output)
- Ladder filter (Moog-style resonance)
- Formant filter (vocal synthesis)
- FFT-based spectral filter
- Convolution reverb
- Chorus / Phaser / Flanger
- Granular synthesis node
- Compressor, Gate, Limiter nodes

## Test Coverage

All node types have unit tests and integration tests:
- 47 tests passing total
- Voice pool polyphony tested
- Graph topological sort tested
- Parameter routing tested
- DSP processing chain tested