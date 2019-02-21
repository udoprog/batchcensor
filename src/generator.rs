use std::ops;

/// Noise generator
pub trait Generator: Sync + Send {
    fn generate(&self, range: ops::Range<usize>, sample_rate: u32) -> Vec<i16>;
}

pub struct Silence(());

impl Silence {
    /// Construct a new generator that generates silence.
    pub fn new() -> Self {
        Silence(())
    }
}

impl Generator for Silence {
    fn generate(&self, range: ops::Range<usize>, _: u32) -> Vec<i16> {
        range.map(|_| i16::default()).collect::<Vec<_>>()
    }
}

pub struct Tone {
    /// Frequency of the tone.
    frequency: f32,
    /// Amplitude from 0..1
    amplitude: f32,
}

impl Tone {
    /// Construct a new default tone generator.
    pub fn new() -> Self {
        Self {
            frequency: 1000f32,
            amplitude: 0.3f32,
        }
    }
}

impl Generator for Tone {
    fn generate(&self, range: ops::Range<usize>, sample_rate: u32) -> Vec<i16> {
        use std::f32::consts::PI;

        let sample_rate = sample_rate as f32;

        range
            .into_iter()
            .enumerate()
            .map(|(i, _)| {
                let mag = (i as f32) * self.frequency * 2f32 * PI / sample_rate;
                (mag.sin() * self.amplitude * (std::i16::MAX as f32)) as i16
            })
            .collect()
    }
}
