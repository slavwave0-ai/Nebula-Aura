use nih_plug::prelude::Buffer;
use realfft::RealFftPlanner;
use std::f32::consts::PI;

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub input_db: f32,
    pub output_db: f32,
    pub harmonics: f32,
    pub frequency: f32,
    pub slope: f32,
    pub odd_db: f32,
    pub even_db: f32,
    pub mix: f32,
    pub oversampling: usize,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            input_db: 0.0,
            output_db: 0.0,
            harmonics: 0.2,
            frequency: 4000.0,
            slope: 24.0,
            odd_db: 2.0,
            even_db: 2.0,
            mix: 1.0,
            oversampling: 1,
        }
    }
}

#[derive(Clone, Default)]
pub struct SpectrumFrame {
    pub bins_hz: Vec<f32>,
    pub magnitudes_db: Vec<f32>,
}

pub struct ExciterCore {
    sample_rate: f32,
    snapshot: Snapshot,
    hp_state_l: f32,
    hp_state_r: f32,
    gain_reduction_db: f32,
    fft_input: Vec<f32>,
    fft_size: usize,
    spectrum: SpectrumFrame,
}

impl Default for ExciterCore {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            snapshot: Snapshot::default(),
            hp_state_l: 0.0,
            hp_state_r: 0.0,
            gain_reduction_db: 0.0,
            fft_input: vec![0.0; 2048],
            fft_size: 2048,
            spectrum: SpectrumFrame::default(),
        }
    }
}

impl ExciterCore {
    pub fn reset(&mut self, sample_rate: f32, max_buffer_size: usize) {
        self.sample_rate = sample_rate;
        self.fft_size = (max_buffer_size.max(1024)).next_power_of_two().min(8192);
        self.fft_input.resize(self.fft_size, 0.0);
        self.clear();
    }

    pub fn clear(&mut self) {
        self.hp_state_l = 0.0;
        self.hp_state_r = 0.0;
        self.gain_reduction_db = 0.0;
        self.fft_input.fill(0.0);
    }

    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = snapshot;
    }

    pub fn process(&mut self, buffer: &mut Buffer, phase_flip: bool) {
        let in_gain = db_to_gain(self.snapshot.input_db);
        let out_gain = db_to_gain(self.snapshot.output_db);
        let odd_gain = db_to_gain(self.snapshot.odd_db) - 1.0;
        let even_gain = db_to_gain(self.snapshot.even_db) - 1.0;

        let alpha = (-2.0 * PI * self.snapshot.frequency / self.sample_rate).exp();
        let slope_shape = (self.snapshot.slope / 100.0).clamp(0.0, 1.0);

        for (i, mut channel_samples) in buffer.iter_samples().enumerate() {
            let dry_l = *channel_samples.get_mut(0).expect("channel 0");
            let dry_r = if channel_samples.len() > 1 {
                *channel_samples.get_mut(1).expect("channel 1")
            } else {
                dry_l
            };

            let mut l = dry_l * in_gain;
            let mut r = dry_r * in_gain;

            self.hp_state_l = alpha * self.hp_state_l + (1.0 - alpha) * l;
            self.hp_state_r = alpha * self.hp_state_r + (1.0 - alpha) * r;

            let high_l = (l - self.hp_state_l) * (0.2 + 1.8 * slope_shape);
            let high_r = (r - self.hp_state_r) * (0.2 + 1.8 * slope_shape);

            let drive = 1.0 + self.snapshot.harmonics * 8.0;
            l = nonlinearity(high_l * drive, odd_gain, even_gain);
            r = nonlinearity(high_r * drive, odd_gain, even_gain);

            let mut wet_l = (dry_l + l).tanh();
            let mut wet_r = (dry_r + r).tanh();

            if phase_flip {
                wet_l = -wet_l;
                wet_r = -wet_r;
            }

            let mixed_l = dry_l * (1.0 - self.snapshot.mix) + wet_l * self.snapshot.mix;
            let mixed_r = dry_r * (1.0 - self.snapshot.mix) + wet_r * self.snapshot.mix;

            *channel_samples.get_mut(0).expect("channel 0") = mixed_l * out_gain;
            if channel_samples.len() > 1 {
                *channel_samples.get_mut(1).expect("channel 1") = mixed_r * out_gain;
            }

            self.fft_input[i % self.fft_size] = 0.5 * (mixed_l + mixed_r);
        }

        self.gain_reduction_db = -20.0
            * (self.snapshot.mix * self.snapshot.harmonics)
                .log10()
                .max(-24.0);
        self.update_spectrum();
    }

    pub fn latest_spectrum(&self) -> SpectrumFrame {
        self.spectrum.clone()
    }

    pub fn current_gain_reduction_db(&self) -> f32 {
        self.gain_reduction_db
    }

    fn update_spectrum(&mut self) {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(self.fft_size);
        let mut inbuf = self.fft_input.clone();
        let mut outbuf = r2c.make_output_vec();
        if r2c.process(&mut inbuf, &mut outbuf).is_ok() {
            let mut bins_hz = Vec::with_capacity(outbuf.len());
            let mut magnitudes_db = Vec::with_capacity(outbuf.len());
            for (i, c) in outbuf.iter().enumerate() {
                bins_hz.push(i as f32 * self.sample_rate / self.fft_size as f32);
                magnitudes_db.push((c.norm() / self.fft_size as f32).max(1.0e-9).log10() * 20.0);
            }
            self.spectrum = SpectrumFrame {
                bins_hz,
                magnitudes_db,
            };
        }
    }
}

fn nonlinearity(sample: f32, odd_gain: f32, even_gain: f32) -> f32 {
    let odd = sample - (sample.powi(3) / 3.0);
    let even = (sample.abs() * sample.signum()).sin();
    sample + odd * odd_gain + even * even_gain
}

fn db_to_gain(db: f32) -> f32 {
    (10.0f32).powf(db / 20.0)
}
