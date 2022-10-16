use std::path::PathBuf;

use anyhow::Context;
use hound::WavReader;
use ndarray::{Array, Array2, Axis};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn read_wav_to_fft(filename: &PathBuf) -> Result<Array2<f32>, anyhow::Error> {
    const WINDOW_SIZE: usize = 44100 / 10;
    const WINDOW_OVERLAP: f64 = 0.0;
    const SKIP_SIZE: usize = (WINDOW_SIZE as f64 * (1f64 - WINDOW_OVERLAP)) as usize;

    println!("Reading wav file");
    let mut wav = WavReader::open(filename).context("Could not open file for reading.")?;
    let samples = wav.samples().collect::<Result<Vec<i16>, _>>().context("Could not interpret file as 16 bit samples.")?;

    println!("Creating windows {window_size} samples long from a timeline {num_samples} samples long, picking every {skip_size} windows with a {overlap} overlap for a total of {num_windows} windows.",
        window_size = WINDOW_SIZE, num_samples = samples.len(), skip_size = SKIP_SIZE, overlap = WINDOW_OVERLAP, num_windows = (samples.len() / SKIP_SIZE) - 1,
    );

    // Convert to an ndarray. f32 for fft.
    let samples_array = Array::from(samples.clone());
    let windows = samples_array
        .windows(ndarray::Dim(WINDOW_SIZE))
        .into_iter()
        .step_by(SKIP_SIZE)
        .collect::<Vec<_>>();
    let windows = ndarray::stack(Axis(0), &windows)?;
    let mut windows = windows.map(|i| Complex::from(*i as f32));

    // Prepare fft
    println!("Performing fft");
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);

    // Since we have a 2-D array of our windows with shape [WINDOW_SIZE, (num_samples / WINDOW_SIZE) - 1], we can run an FFT on every row.
    // Next step is to do something multithreaded with Rayon, but we're not cool enough for that yet.
    windows.axis_iter_mut(Axis(0)).for_each(|mut frame| {
        fft.process(frame.as_slice_mut().unwrap());
    });

    // Get the real component of those complex numbers we get back from the FFT
    let windows = windows.map(|i| i.re);

    // And finally, only look at the first half of the spectrogram - the first (n/2)+1 points of each FFT
    // https://dsp.stackexchange.com/questions/4825/why-is-the-fft-mirrored
    let windows = windows.slice_move(ndarray::s![.., ..((WINDOW_SIZE / 2) + 1)]);

    Ok(windows)
}
