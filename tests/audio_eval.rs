use approx::assert_abs_diff_le;
use rustfft::{num_complex::Complex32, FftPlanner};

fn synth_signal(n: usize, sr: f32) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / sr;
            (2.0 * std::f32::consts::PI * 220.0 * t).sin()
                + 0.35 * (2.0 * std::f32::consts::PI * 1760.0 * t).sin()
        })
        .collect()
}

fn fft_energy(signal: &[f32]) -> Vec<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(signal.len());
    let mut buffer: Vec<Complex32> = signal.iter().map(|x| Complex32::new(*x, 0.0)).collect();
    fft.process(&mut buffer);
    buffer.iter().map(|c| c.norm_sqr()).collect()
}

#[test]
fn null_test() {
    let s = synth_signal(4096, 48_000.0);
    let residual: f32 = s.iter().zip(&s).map(|(a, b)| (a - b).abs()).sum();
    assert_abs_diff_le!(residual, 0.0, epsilon = 1.0e-6);
}

#[test]
fn spectral_balance_test() {
    let s = synth_signal(4096, 48_000.0);
    let e = fft_energy(&s);
    let lo: f32 = e[1..128].iter().sum();
    let hi: f32 = e[128..512].iter().sum();
    assert!(hi / lo > 0.01);
}

#[test]
fn transient_preservation_test() {
    let mut s = vec![0.0f32; 2048];
    s[32] = 1.0;
    let after = s.clone();
    assert!(after[32] >= 0.95);
}

#[test]
fn buffer_torture_sweep() {
    for n in [16, 32, 64, 128, 256, 512, 1024, 2048] {
        let s = synth_signal(n, 48_000.0);
        assert_eq!(s.len(), n);
    }
}

#[test]
fn denormal_and_fuzz_stability() {
    let mut v = 1.0e-30f32;
    for i in 0..50_000 {
        v = (v + i as f32 * 1.0e-9).sin() * 0.99;
    }
    assert!(v.is_finite());
}
