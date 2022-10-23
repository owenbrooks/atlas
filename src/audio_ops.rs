use std::path::Path;

use anyhow::Context;
use hound::WavReader;
use ndarray::{Array, Array2, Axis};
use rustfft::{num_complex::Complex, FftPlanner};

pub fn read_wav_to_fft(
    filename: &Path,
    window_length: f32,
) -> Result<Array2<f32>, anyhow::Error> {
    println!("Reading wav file");
    let mut wav = WavReader::open(filename).context("Could not open file for reading.")?;
    let wav_spec = wav.spec();
    let sample_rate = wav_spec.sample_rate;
    let channels = wav_spec.channels.into();
    let samples = wav
        .samples()
        .step_by(channels)
        .collect::<Result<Vec<i16>, _>>()
        .context("Could not interpret file as 16 bit samples.")?;

    let window_size: usize = (sample_rate as f32 * window_length) as usize;
    const WINDOW_OVERLAP: f64 = 0.0;
    let skip_size: usize = (window_size as f64 * (1f64 - WINDOW_OVERLAP)) as usize;

    println!("Creating windows {window_size} samples long from a timeline {num_samples} samples long, picking every {skip_size} windows with a {overlap} overlap for a total of {num_windows} windows.",
        window_size = window_size, num_samples = samples.len(), skip_size = skip_size, overlap = WINDOW_OVERLAP, num_windows = (samples.len() / skip_size) - 1,
    );
    println!(
        "Sample rate is {sample_rate} Hz. Bit depth is {}. {} channels ",
        wav.spec().bits_per_sample,
        wav.spec().channels
    );

    // Convert to an ndarray. f32 for fft.
    let samples_array = Array::from(samples.clone());
    let windows = samples_array
        .windows(ndarray::Dim(window_size))
        .into_iter()
        .step_by(skip_size)
        .collect::<Vec<_>>();
    let windows = ndarray::stack(Axis(0), &windows)?;
    let mut windows = windows.map(|i| Complex::from(*i as f32));

    // Prepare fft
    println!("Performing fft");
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    // Since we have a 2-D array of our windows with shape [WINDOW_SIZE, (num_samples / WINDOW_SIZE) - 1], we can run an FFT on every row.
    // Next step is to do something multithreaded with Rayon, but we're not cool enough for that yet.
    windows.axis_iter_mut(Axis(0)).for_each(|mut frame| {
        fft.process(frame.as_slice_mut().unwrap());
    });

    // Get the real component of those complex numbers we get back from the FFT
    let windows = windows.map(|i| i.re);

    // And finally, only look at the first half of the spectrogram - the first (n/2)+1 points of each FFT
    // https://dsp.stackexchange.com/questions/4825/why-is-the-fft-mirrored
    let windows = windows.slice_move(ndarray::s![.., ..((window_size / 2) + 1)]);

    Ok(windows)
}
