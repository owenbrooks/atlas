//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use anyhow::Context;
use clap::Parser;
use hound::WavReader;
use ndarray::{s, Array, Array2, ArrayView2, Axis};
use ndarray_stats::QuantileExt;
use plotters::prelude::*;
use rustfft::{num_complex::Complex, FftPlanner};
use std::{path::PathBuf, ffi::OsStr};

// returns values, centred on x,y coord
fn get_square(data: &Array2<f32>, x: usize, y: usize, width: usize) -> Option<ArrayView2<f32>> {
    let rows = data.nrows();
    let cols = data.ncols();
    if x > cols || y > rows {
        return None;
    }

    let min_x = if x < width / 2 { 0 } else { x - width / 2 };
    let max_x = x + width / 2;
    let max_x = usize::min(max_x, cols - 1);
    let min_y = if y < width / 2 { 0 } else { y - width / 2 };
    let max_y = y + width / 2;
    let max_y = usize::min(max_y, rows - 1);

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

    #[clap(short, default_value_t = 30)]
    kernel_size: usize,

    #[clap(short, long, action, default_value_t = false)]
    save_png: bool,
}

fn max_filter(spec: &Array2<f32>, kernel_size: usize) -> Array2<f32> {
    let mut filtered = Array::zeros(spec.raw_dim());
    for x in 0..spec.ncols() {
        for y in 0..spec.nrows() {
            let square = get_square(spec, x, y, kernel_size).unwrap();
            let max = square.max();
            match max {
                Ok(_) => (),
                Err(_) => {
                    dbg!(x, y, kernel_size);
                }
            }
            let pos = filtered.get_mut((y, x)).expect("out of bounds");
            // let max = spec.get((x, y)).expect("out of bounds");
            *pos = *max.unwrap();
        }
    }
    filtered
}

fn find_equal(array_a: &Array2<f32>, array_b: &Array2<f32>) -> Vec<(usize, usize)> {
    let mut locations = vec![];
    for (loc, elem) in array_a.indexed_iter() {
        if *array_b.get(loc).unwrap() == *elem {
            locations.push(loc);
        }
    }

    locations
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);

    let windows = read_wav_to_fft(&args.input_wav)?;
    let filtered = max_filter(&windows, args.kernel_size);

    let base_wav_name = args.input_wav.file_stem().unwrap_or(OsStr::new(""));
    if args.save_png {
        let mut output_name = base_wav_name.to_os_string();
        output_name.push("_spec.png");
        let mut output_name_max = base_wav_name.to_os_string();
        output_name_max.push("_spec_max.png");
        let out_path = PathBuf::from(output_name);
        let out_path_max = PathBuf::from(output_name_max);

        save_png(&windows, out_path);
        save_png(&filtered, out_path_max);
    }

    // find peak locations
    println!("Finding peak locations");
    let peak_locations = find_equal(&windows, &filtered);

    let mut output_name = base_wav_name.to_os_string();
    output_name.push("_peaks.png");
    plot_peaks(&peak_locations, windows.ncols(), windows.nrows(), 4410, PathBuf::from(output_name))
        .context(format!("Unable to plot peaks"))?;

    Ok(())
}

fn plot_peaks(
    peak_locations: &[(usize, usize)],
    height: usize,
    width: usize,
    samples_per_second: usize,
    output_path: PathBuf,
) -> Result<(), anyhow::Error> {
    println!("Plotting peaks");
    dbg!(height, width);
    let root = BitMapBackend::new(&output_path, (width as u32 / 2, height as u32 / 2))
        .into_drawing_area();

    root.fill(&WHITE)?;

    let areas = root.split_by_breakpoints([width as u32 - 40], [40]);

    let mut scatter_ctx = ChartBuilder::on(&areas[2])
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0..width, 0..height*5)?;
    scatter_ctx
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .draw()?;
    scatter_ctx.draw_series(
        peak_locations
            .iter()
            .map(|(x, y)| Circle::new((*x/samples_per_second, *y*10/2), 2, GREEN.filled())),
    )?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", output_path.to_string_lossy());

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

fn save_png(windows: &Array2<f32>, output_path: PathBuf) {
    let height = windows.ncols();
    let width = windows.nrows();

    println!("Generating a {} wide x {} high image", width, height);

    let image_dimensions: (u32, u32) = (width as u32, height as u32);
    let root_drawing_area = BitMapBackend::new(
        &output_path,
        image_dimensions, // width x height. Worth it if we ever want to resize the graph.
    )
    .into_drawing_area();

    let spectrogram_cells = root_drawing_area.split_evenly((height, width));

    let window_size = (height - 1) * 2;
    let windows_scaled = windows.map(|i| i.abs() / (window_size as f32));
    let highest_spectral_density = windows_scaled.max_skipnan();

    // transpose and flip around to prepare for graphing
    let windows_flipped = windows_scaled.slice(ndarray::s![.., ..; -1]); // flips the
    let windows_flipped = windows_flipped.t();

    // Finally add a color scale
    let color_scale = colorous::PLASMA;

    for (cell, spectral_density) in spectrogram_cells.iter().zip(windows_flipped.iter()) {
        let spectral_density_scaled = 2.*spectral_density.sqrt() / highest_spectral_density.sqrt();
        let color = color_scale.eval_continuous(spectral_density_scaled as f64);
        cell.fill(&RGBColor(color.r, color.g, color.b)).unwrap();
    }

    root_drawing_area.present().unwrap();
}
