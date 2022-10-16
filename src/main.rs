//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use anyhow::Context;
use clap::Parser;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
mod audio_ops;
mod image_ops;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,

    // analysis parameters
    #[clap(long, default_value_t = 0.1)]
    window_length: f32, // in seconds

    #[clap(short, default_value_t = 30)]
    kernel_size: usize, // used for maximum filter
    #[clap(short, long, default_value_t = 0.1)]
    magnitude_threshold: f32, // used for maximum filter

    // actions
    #[clap(short, long, action, default_value_t = false)]
    save_png: bool,

    #[clap(long, action)]
    read_from_cache: bool,

    #[clap(long, action)]
    save_to_cache: bool,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);

    let base_wav_name = args.input_wav.file_stem().unwrap_or(OsStr::new(""));

    let windows = if !args.read_from_cache {
        audio_ops::read_wav_to_fft(&args.input_wav, args.window_length)?
    } else {
        audio_ops::read_cached_fft(&base_wav_name)?
    };

    if args.save_to_cache {
        audio_ops::save_to_cache(&base_wav_name, &windows)?;
    }

    let filtered = image_ops::max_filter(&windows, args.kernel_size);

    let output_dir = Path::new("output");
    fs::create_dir_all(output_dir)?;

    if args.save_png {
        let mut output_name = base_wav_name.to_os_string();
        output_name.push("_spec.png");
        let mut output_name_max = base_wav_name.to_os_string();
        output_name_max.push("_spec_max.png");
        let out_path = output_dir.join(output_name);
        let out_path_max = output_dir.join(output_name_max);

        image_ops::save_png(&windows, out_path);
        image_ops::save_png(&filtered, out_path_max);
    }

    // find peak locations
    println!("Finding peak locations");
    let peak_locations = image_ops::find_equal(&windows, &filtered);
    // filter for only peaks bigger than magnitude threshold

    let max_peak_locations: Vec<(usize, usize)> = peak_locations
        .iter()
        .filter(|&&loc| *windows.get(loc).unwrap() > args.magnitude_threshold)
        .map(|&loc| loc)
        .collect();

    let mut output_name = base_wav_name.to_os_string();
    output_name.push("_peaks.png");
    image_ops::plot_peaks(
        &max_peak_locations,
        windows.ncols(),
        windows.nrows(),
        args.window_length,
        output_dir.join(output_name),
    )
    .context(format!("Unable to plot peaks"))?;

    Ok(())
}
