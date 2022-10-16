//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use anyhow::Context;
use clap::Parser;
use hound::WavReader;
use ndarray::{Array, Array2, Axis};
use rustfft::{num_complex::Complex, FftPlanner};
use std::{path::PathBuf, ffi::OsStr};
mod image_ops;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,

    #[clap(short, default_value_t = 30)]
    kernel_size: usize,

    #[clap(short, long, action, default_value_t = false)]
    save_png: bool,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);

    let windows = read_wav_to_fft(&args.input_wav)?;
    let filtered = image_ops::max_filter(&windows, args.kernel_size);

    let base_wav_name = args.input_wav.file_stem().unwrap_or(OsStr::new(""));
    if args.save_png {
        let mut output_name = base_wav_name.to_os_string();
        output_name.push("_spec.png");
        let mut output_name_max = base_wav_name.to_os_string();
        output_name_max.push("_spec_max.png");
        let out_path = PathBuf::from(output_name);
        let out_path_max = PathBuf::from(output_name_max);

        image_ops::save_png(&windows, out_path);
        image_ops::save_png(&filtered, out_path_max);
    }

    // find peak locations
    println!("Finding peak locations");
    let peak_locations = image_ops::find_equal(&windows, &filtered);

    let mut output_name = base_wav_name.to_os_string();
    output_name.push("_peaks.png");
    image_ops::plot_peaks(&peak_locations, windows.ncols(), windows.nrows(), 4410, PathBuf::from(output_name))
        .context(format!("Unable to plot peaks"))?;

    Ok(())
}


fn read_wav_to_fft(filename: &PathBuf) -> Result<Array2<f32>, anyhow::Error> {
    const WINDOW_SIZE: usize = 44100 / 10;
    const WINDOW_OVERLAP: f64 = 0.0;
    const SKIP_SIZE: usize = (WINDOW_SIZE as f64 * (1f64 - WINDOW_OVERLAP)) as usize;

    println!("Reading wav file");
    let mut wav = WavReader::open(filename).unwrap();
    let samples = wav.samples().collect::<Result<Vec<i16>, _>>().unwrap();

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
