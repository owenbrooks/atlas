//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use anyhow::Context;
use clap::Parser;
use std::{path::PathBuf, ffi::OsStr};
mod image_ops;
mod audio_ops;

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

    let windows = audio_ops::read_wav_to_fft(&args.input_wav)?;
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
    image_ops::plot_peaks(&peak_locations, windows.ncols(), windows.nrows(), 44100, PathBuf::from(output_name))
        .context(format!("Unable to plot peaks"))?;

    Ok(())
}
