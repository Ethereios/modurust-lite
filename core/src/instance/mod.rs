/// SuperInstance-style voice pool.
///
/// Manages a fixed-capacity pool of `VoiceInstance`s (polyphony governed by
/// `instance_density`). Allocation performs voice stealing when the pool is
/// exhausted. `instance_spread` controls the per-voice detune/level variation
/// for a "super" layered sound.
pub struct VoiceInstance {
    pub id: u64,
    pub active: bool,
    pub pitch: f32,
    pub envelope: f32,
    pub velocity: f32,
    pub detune: f32,
    /// Per-voice oscillator phase (advanced only when the voice is active).
    pub phase: f32,
}

pub struct SuperInstancePool {
    instances: Vec<VoiceInstance>,
    next_id: u64,
    pub spread: f32,
}

impl SuperInstancePool {
    pub fn new(capacity: usize) -> Self {
        let mut instances = Vec::with_capacity(capacity);
        for id in 0..capacity {
            instances.push(VoiceInstance {
                id: id as u64,
                active: false,
                pitch: 440.0,
                envelope: 0.0,
                velocity: 1.0,
                detune: 0.0,
                phase: 0.0,
            });
        }
        Self {
            instances,
            next_id: capacity as u64,
            spread: 0.5,
        }
    }

    /// Total pool capacity (instance density).
    pub fn capacity(&self) -> usize {
        self.instances.len()
    }

    /// Number of currently active voices.
    pub fn active_count(&self) -> usize {
        self.instances.iter().filter(|v| v.active).count()
    }

    /// Resize the pool. If growing, new voices are appended as inactive.
    pub fn resize(&mut self, new_capacity: usize) {
        let new_capacity = new_capacity.max(1);
        if new_capacity > self.instances.len() {
            for id in self.next_id..self.next_id + (new_capacity - self.instances.len()) as u64 {
                self.instances.push(VoiceInstance {
                    id,
                    active: false,
                    pitch: 440.0,
                    envelope: 0.0,
                    velocity: 1.0,
                    detune: 0.0,
                    phase: 0.0,
                });
            }
            self.next_id = self.next_id + (new_capacity - self.instances.len()) as u64;
        } else {
            // Shrink: deactivate dropped voices.
            self.instances.truncate(new_capacity);
        }
    }

    /// Set the spread amount (0.0..=1.0). Higher spread = more detune/variance.
    pub fn set_spread(&mut self, spread: f32) {
        self.spread = spread.clamp(0.0, 1.0);
    }

    /// Allocate a voice for the given pitch (Hz) and velocity (0..=1).
    /// Returns the allocated voice id, stealing the oldest active voice if the
    /// pool is full.
    pub fn allocate(&mut self, pitch: f32, velocity: f32) -> u64 {
        // Try to find an inactive voice.
        if let Some(v) = self.instances.iter_mut().find(|v| !v.active) {
            v.active = true;
            v.pitch = pitch;
            v.envelope = 1.0;
            v.velocity = velocity.clamp(0.0, 1.0);
            v.detune = (v.id as f32 % 7.0 - 3.0) / 3.0 * self.spread;
            v.phase = 0.0;
            return v.id;
        }

        // Voice stealing: reuse the first active voice (simple oldest-first).
        if let Some(v) = self.instances.first_mut() {
            v.pitch = pitch;
            v.envelope = 1.0;
            v.velocity = velocity.clamp(0.0, 1.0);
            v.detune = (v.id as f32 % 7.0 - 3.0) / 3.0 * self.spread;
            v.phase = 0.0;
            return v.id;
        }
        0
    }

    /// Release all voices matching the given pitch (Hz).
    pub fn release_note(&mut self, pitch: f32) {
        for v in self.instances.iter_mut() {
            if v.active && (v.pitch - pitch).abs() < 1.0 {
                v.active = false;
                v.envelope = 0.0;
            }
        }
    }

    /// Release all active voices.
    pub fn release_all(&mut self) {
        for v in self.instances.iter_mut() {
            v.active = false;
            v.envelope = 0.0;
        }
    }

    /// Iterator over the pool (for UI/visualization).
    pub fn iter(&self) -> impl Iterator<Item = &VoiceInstance> {
        self.instances.iter()
    }

    /// Iterator over active voices (for rendering).
    pub fn iter_active(&self) -> impl Iterator<Item = &VoiceInstance> {
        self.instances.iter().filter(|v| v.active)
    }

    /// Mutable iterator over active voices (for per-sample rendering).
    pub fn iter_active_mut(&mut self) -> impl Iterator<Item = &mut VoiceInstance> {
        self.instances.iter_mut().filter(|v| v.active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_and_release() {
        let mut pool = SuperInstancePool::new(4);
        assert_eq!(pool.capacity(), 4);
        assert_eq!(pool.active_count(), 0);

        let _id = pool.allocate(440.0, 0.8);
        assert_eq!(pool.active_count(), 1);
        pool.release_note(440.0);
        assert_eq!(pool.active_count(), 0);
        assert!(pool.iter().all(|v| !v.active));
    }

    #[test]
    fn test_voice_stealing() {
        let mut pool = SuperInstancePool::new(2);
        pool.allocate(440.0, 0.5);
        pool.allocate(660.0, 0.5);
        // Pool full; this should steal the first voice.
        let stolen_id = pool.allocate(880.0, 0.5);
        assert_eq!(pool.active_count(), 2);
        assert_eq!(stolen_id, 0);
    }

    #[test]
    fn test_resize() {
        let mut pool = SuperInstancePool::new(2);
        pool.resize(8);
        assert_eq!(pool.capacity(), 8);
        pool.resize(3);
        assert_eq!(pool.capacity(), 3);
    }
}

