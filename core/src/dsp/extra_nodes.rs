use crate::dsp::DspNode;

pub struct LfoNode {
    pub freq: f32,
    pub phase: f32,
    pub sample_rate: f32,
}

impl LfoNode {
    pub fn new(freq: f32, sample_rate: f32) -> Self {
        Self { freq, phase: 0.0, sample_rate }
    }
}

impl DspNode for LfoNode {
    fn process(&mut self, _inputs: &[f32], outputs: &mut [f32]) {
        let phase_inc = self.freq / self.sample_rate;
        for out in outputs.iter_mut() {
            *out = (self.phase * 2.0 * std::f32::consts::PI).sin();
            self.phase = (self.phase + phase_inc) % 1.0;
        }
    }
}

pub struct ReverbNode {
    buffer: Vec<f32>,
    write_pos: usize,
    pub room_size: f32,
    pub damping: f32,
}

impl ReverbNode {
    pub fn new(sample_rate: f32) -> Self {
        let buf_size = (sample_rate * 0.1) as usize;
        Self {
            buffer: vec![0.0; buf_size],
            write_pos: 0,
            room_size: 0.85,
            damping: 0.5,
        }
    }
}

impl DspNode for ReverbNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        let buf_len = self.buffer.len();
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() { break; }
            let delayed = self.buffer[self.write_pos];
            let out_val = x + delayed * self.room_size;
            self.buffer[self.write_pos] = x * (1.0 - self.damping) + delayed * self.room_size;
            outputs[i] = out_val;
            self.write_pos = (self.write_pos + 1) % buf_len;
        }
    }
}

pub struct MixerNode {
    pub gain1: f32,
    pub gain2: f32,
}

impl MixerNode {
    pub fn new(gain1: f32, gain2: f32) -> Self {
        Self { gain1, gain2 }
    }

    /// Mix two input signals with per-channel gain.
    /// Used by the standalone `mix()` API; the `DspNode` impl below adapts
    /// this to the graph's single-input buffer by multiplying the input by
    /// `gain1` and adding a copy scaled by `gain2` (self-modulation).
    pub fn mix(&self, input1: &[f32], input2: &[f32], output: &mut [f32]) {
        for i in 0..output.len() {
            let in1 = input1.get(i).copied().unwrap_or(0.0);
            let in2 = input2.get(i).copied().unwrap_or(0.0);
            output[i] = in1 * self.gain1 + in2 * self.gain2;
        }
    }
}

impl DspNode for MixerNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        // In the graph, the mixer receives the accumulated upstream signal in
        // `inputs`. It applies gain1 to the input and adds a copy scaled by
        // gain2 (self-modulation / feedback). This preserves the semantics of
        // a stereo mixer in a single-input topology.
        for (i, &x) in inputs.iter().enumerate() {
            if i >= outputs.len() {
                break;
            }
            outputs[i] = x * self.gain1 + x * self.gain2;
        }
    }
}

/// Duplicates the input to two outputs (identical signal).
pub struct SplitterNode;

impl SplitterNode {
    pub fn new() -> Self {
        Self
    }

    pub fn split(&self, input: &[f32], out_a: &mut [f32], out_b: &mut [f32]) {
        for i in 0..out_a.len() {
            let v = input.get(i).copied().unwrap_or(0.0);
            out_a[i] = v;
            out_b[i] = v;
        }
    }
}

impl DspNode for SplitterNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        // In the graph, the splitter is a passthrough: the accumulated upstream
        // signal is passed through to the output. Downstream fan-out is handled
        // by the graph topology (multiple nodes reading from this node's buffer).
        let n = inputs.len().min(outputs.len());
        outputs[..n].copy_from_slice(&inputs[..n]);
        if outputs.len() > n {
            outputs[n..].fill(0.0);
        }
    }
}

/// A routing/matrix node: mixes N inputs into M outputs with per-route gains.
pub struct RouterNode {
    pub routes: Vec<(usize, usize, f32)>, // (input_index, output_index, gain)
    pub num_inputs: usize,
    pub num_outputs: usize,
}

impl RouterNode {
    pub fn new(num_inputs: usize, num_outputs: usize) -> Self {
        Self {
            routes: Vec::new(),
            num_inputs,
            num_outputs,
        }
    }

    pub fn connect(&mut self, input: usize, output: usize, gain: f32) {
        if input < self.num_inputs && output < self.num_outputs {
            self.routes.push((input, output, gain));
        }
    }

    pub fn clear(&mut self) {
        self.routes.clear();
    }

    /// Process a 2D block (inputs x frames) into (outputs x frames).
    pub fn process_block(&self, input_block: &[Vec<f32>], output_block: &mut [Vec<f32>]) {
        for out in output_block.iter_mut() {
            out.fill(0.0);
        }
        for (in_idx, out_idx, gain) in &self.routes {
            if let (Some(input), Some(output)) = (input_block.get(*in_idx), output_block.get_mut(*out_idx)) {
                for (i, &v) in input.iter().enumerate() {
                    if let Some(slot) = output.get_mut(i) {
                        *slot += v * gain;
                    }
                }
            }
        }
    }
}

/// Stereo delay with independent left/right delay times and cross-feedback.
pub struct StereoDelayNode {
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    left_pos: usize,
    right_pos: usize,
    pub delay_left_samples: usize,
    pub delay_right_samples: usize,
    pub feedback: f32,
    pub cross_feedback: f32,
}

impl StereoDelayNode {
    pub fn new(
        max_delay_samples: usize,
        delay_left_samples: usize,
        delay_right_samples: usize,
        feedback: f32,
    ) -> Self {
        Self {
            left_buffer: vec![0.0; max_delay_samples],
            right_buffer: vec![0.0; max_delay_samples],
            left_pos: 0,
            right_pos: 0,
            delay_left_samples,
            delay_right_samples,
            feedback,
            cross_feedback: 0.0,
        }
    }

pub fn process_stereo(&mut self, left_in: &[f32], right_in: &[f32], left_out: &mut [f32], right_out: &mut [f32]) {
        let buf_len = self.left_buffer.len();
        for i in 0..left_out.len() {
            let l = left_in.get(i).copied().unwrap_or(0.0);
            let r = right_in.get(i).copied().unwrap_or(0.0);

            let l_read = (self.left_pos + buf_len - self.delay_left_samples) % buf_len;
            let r_read = (self.right_pos + buf_len - self.delay_right_samples) % buf_len;
            let delayed_l = self.left_buffer[l_read];
            let delayed_r = self.right_buffer[r_read];

            self.left_buffer[self.left_pos] = l + delayed_l * self.feedback + delayed_r * self.cross_feedback;
            self.right_buffer[self.right_pos] = r + delayed_r * self.feedback + delayed_l * self.cross_feedback;

            left_out[i] = l + delayed_l;
            right_out[i] = r + delayed_r;

            self.left_pos = (self.left_pos + 1) % buf_len;
            self.right_pos = (self.right_pos + 1) % buf_len;
        }
    }
}

impl DspNode for StereoDelayNode {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        // In the graph, the stereo delay receives a mono input and duplicates
        // it to both left and right delay lines. Output is summed back to mono
        // (left + right) / 2 for the single-output graph topology.
        let buf_len = self.left_buffer.len();
        for i in 0..outputs.len() {
            let x = inputs.get(i).copied().unwrap_or(0.0);
            let l_read = (self.left_pos + buf_len - self.delay_left_samples) % buf_len;
            let r_read = (self.right_pos + buf_len - self.delay_right_samples) % buf_len;
            let delayed_l = self.left_buffer[l_read];
            let delayed_r = self.right_buffer[r_read];

            self.left_buffer[self.left_pos] = x + delayed_l * self.feedback + delayed_r * self.cross_feedback;
            self.right_buffer[self.right_pos] = x + delayed_r * self.feedback + delayed_l * self.cross_feedback;

            outputs[i] = (x + delayed_l + x + delayed_r) * 0.5;

            self.left_pos = (self.left_pos + 1) % buf_len;
            self.right_pos = (self.right_pos + 1) % buf_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router() {
        let mut router = RouterNode::new(2, 2);
        router.connect(0, 0, 1.0);
        router.connect(1, 1, 0.5);
        let input = vec![vec![1.0; 4], vec![2.0; 4]];
        let mut output = vec![vec![0.0; 4], vec![0.0; 4]];
        router.process_block(&input, &mut output);
        assert_eq!(output[0][0], 1.0);
        assert_eq!(output[1][0], 1.0); // 2.0 * 0.5
    }

    #[test]
    fn test_stereo_delay() {
        let mut delay = StereoDelayNode::new(1024, 10, 12, 0.5);
        let mut out_l = vec![0.0; 4];
        let mut out_r = vec![0.0; 4];
        delay.process_stereo(&[1.0; 4], &[0.0; 4], &mut out_l, &mut out_r);
        // With delay > 0 the first outputs should be dry input.
        assert_eq!(out_l[0], 1.0);
    }
}

