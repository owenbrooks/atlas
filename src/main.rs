//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use hound::WavReader;
use ndarray::{s, Array, Array2, Axis, ArrayView2, array};
use ndarray_stats::QuantileExt;
use plotters::prelude::*;
use rustfft::{num_complex::Complex, FftPlanner};
use std::path::PathBuf;

// returns values, centred on x,y coord
fn get_square(data: &Array2<f32>, x: usize, y: usize, width: usize) -> Option<ArrayView2<f32>> {
    let rows = data.nrows();
    let cols = data.ncols();
    if x > cols || y > rows {
        return None;
    }

    let min_x = if x < width / 2 { 0 } else { x - width / 2 };
    let max_x = x + width / 2;
    let max_x = usize::min(max_x, cols-1);
    let min_y = if y < width / 2 { 0 } else { y - width / 2 };
    let max_y = y + width / 2;
    let max_y = usize::min(max_y, rows-1);

    // println!("x: {}, width/2: {}, rows: {}, cols: {}, {} {} {} {}", x, width/2, rows, cols, min_x, max_x, min_y, max_y);
    // dbg!(x, y, width/2, rows, cols, min_x, max_x, min_y, max_y);

    Some(data.slice(s![min_y..=max_y, min_x..=max_x]))
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,
}

fn max_filter(spec: &Array2<f32>, kernel_size: usize) -> Array2<f32> {
    let mut filtered = Array::zeros(spec.raw_dim());
    for x in 0..spec.ncols() {
        for y in 0..spec.nrows() {
            let square = get_square(spec, x, y, kernel_size).unwrap();
            let max = square.max();
            match max {
                Ok(_) => (),
                Err(_) => {dbg!(x, y, kernel_size);}
            }
            let pos = filtered.get_mut((y, x)).expect("out of bounds");
            // let max = spec.get((x, y)).expect("out of bounds");
            *pos = *max.unwrap();
        }
    }
    filtered
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);
    let kernel_size = 5;

    let windows = read_wav_to_fft(&args.input_wav)?;
    
    let filtered = max_filter(&windows, kernel_size);
    let out_path = PathBuf::from("output.png");
    let out_path_max = PathBuf::from("output_max.png");

    save_png(&windows, out_path);
    save_png(&filtered, out_path_max);

    Ok(())
}

const WINDOW_SIZE: usize = 44100 / 5;
const OVERLAP: f64 = 0.1;
const SKIP_SIZE: usize = (WINDOW_SIZE as f64 * (1f64 - OVERLAP)) as usize;

fn read_wav_to_fft(filename: &PathBuf) -> Result<Array2<f32>, anyhow::Error> {
    let mut wav = WavReader::open(filename).unwrap();
    let samples = wav.samples().collect::<Result<Vec<i16>, _>>().unwrap();

    println!("Creating windows {window_size} samples long from a timeline {num_samples} samples long, picking every {skip_size} windows with a {overlap} overlap for a total of {num_windows} windows.",
        window_size = WINDOW_SIZE, num_samples = samples.len(), skip_size = SKIP_SIZE, overlap = OVERLAP, num_windows = (samples.len() / SKIP_SIZE) - 1,
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

fn save_png(windows: &Array2<f32>, output_path: PathBuf) {
    let height = windows.nrows();
    let width = windows.ncols();

    println!("Generating a {} wide x {} high image", width, height);

    let image_dimensions: (u32, u32) = (width as u32, height as u32);
    let root_drawing_area = BitMapBackend::new(
        &output_path,
        image_dimensions, // width x height. Worth it if we ever want to resize the graph.
    )
    .into_drawing_area();

    let spectrogram_cells = root_drawing_area.split_evenly((height, width));

    let windows_scaled = windows.map(|i| i.abs() / (WINDOW_SIZE as f32));
    let highest_spectral_density = windows_scaled.max_skipnan();

    // transpose and flip around to prepare for graphing
    let windows_flipped = windows_scaled.slice(ndarray::s![.., ..; -1]); // flips the
    let windows_flipped = windows_flipped.t();

    // Finally add a color scale
    let color_scale = colorous::COOL;

    for (cell, spectral_density) in spectrogram_cells.iter().zip(windows_flipped.iter()) {
        let spectral_density_scaled = spectral_density.sqrt() / highest_spectral_density.sqrt();
        let color = color_scale.eval_continuous(spectral_density_scaled as f64);
        cell.fill(&RGBColor(color.r, color.g, color.b)).unwrap();
    }

    root_drawing_area.present().unwrap();
}
