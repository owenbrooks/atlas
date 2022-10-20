//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use anyhow::Context;
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
};
mod audio_ops;
mod database;
mod hash;
mod image_ops;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    action: Option<Action>,

    #[clap(short, long, parse(from_os_str), default_value = "database.db3")]
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

    // matching target zone parameters
    #[clap(short, long, default_value_t = 0.1)]
    target_zone_delay_sec: f32,
    #[clap(short, long, default_value_t = 750.0)]
    target_zone_height_hz: f32,
    #[clap(short, long, default_value_t = 3.0)]
    target_zone_width_sec: f32,

    // actions
    #[clap(short, long, action, default_value_t = false)]
    save_png: bool,

    #[clap(long, action)]
    read_from_cache: bool,

    #[clap(long, action)]
    save_to_cache: bool,
}

#[derive(clap::Subcommand, Debug, Clone, Copy)]
enum Action {
    Add,
    Match,
}

fn add(args: &Args) -> Result<(), anyhow::Error> {
    let wav_base_name = args
        .input_wav
        .file_stem()
        .context("Please provide a file not a directory.")?;

    let windows = audio_ops::read_wav_to_fft(&args.input_wav, args.window_length)?;
    let filtered = image_ops::max_filter(&windows, args.kernel_size);

    // find peak locations
    println!("Finding peak locations");
    let peak_locations = image_ops::find_equal(&windows, &filtered);
    // filter for only peaks bigger than magnitude threshold
    let max_peak_locations: Vec<(usize, usize)> = peak_locations
        .iter()
        .filter(|&&loc| *windows.get(loc).unwrap() > args.magnitude_threshold)
        .map(|&loc| loc)
        .collect();

    if args.save_png {
        // save image files of output if requested
        let output_dir = Path::new("output");
        fs::create_dir_all(output_dir)?;

        let mut output_name = wav_base_name.to_os_string();
        output_name.push("_spec.png");
        let mut output_name_max = wav_base_name.to_os_string();
        output_name_max.push("_spec_max.png");
        let out_path = output_dir.join(output_name);
        let out_path_max = output_dir.join(output_name_max);

        image_ops::save_png(&windows, out_path);
        image_ops::save_png(&filtered, out_path_max);

        let mut peaks_filename = wav_base_name.to_os_string();
        peaks_filename.push("_peaks.png");
        image_ops::plot_peaks(
            &max_peak_locations,
            windows.ncols(),
            windows.nrows(),
            args.window_length,
            output_dir.join(peaks_filename),
        )
        .context(format!("Unable to plot peaks"))?;
    }

    // add track to track list
    let conn = database::connect(&args.database)?;
    let track_name = wav_base_name.to_string_lossy().to_string();
    let track_id = database::add_track(&conn, &track_name)?;
    println!("Track {} added with id {}", track_name, track_id);

    // generate fingerprint
    println!("fingerprinting");
    let pair_records = hash::fingerprint(
        max_peak_locations,
        args.window_length,
        args.target_zone_delay_sec,
        args.target_zone_height_hz,
        args.target_zone_width_sec,
    );
    println!("done fingerprinting");
    // add fingerprint to database, deleting existing records
    // TODO: make this into a transaction
    let mut delete_statement = conn.prepare("DELETE FROM fingerprints WHERE track_id = (?1)")?;
    delete_statement.execute([track_id])?;

    let mut insert_statement =
        conn.prepare("INSERT INTO fingerprints (hash, time_a, track_id) values (?1, ?2, ?3)")?;
    for record in &pair_records {
        insert_statement
            .execute(&[
                &record.hash.to_string(),
                &record.time_a.to_string(),
                &track_id.to_string(),
            ])
            .context("Failed to insert.")?;
    }
    println!("Inserted {} fingerprints", pair_records.len());

    Ok(())
}

fn match_sample(args: &Args) -> Result<(), anyhow::Error> {
    let wav_base_name = args
        .input_wav
        .file_stem()
        .context("Please provide a file not a directory.")?;

    let windows = audio_ops::read_wav_to_fft(&args.input_wav, args.window_length)?;
    let filtered = image_ops::max_filter(&windows, args.kernel_size);

    // find peak locations
    println!("Finding peak locations");
    let peak_locations = image_ops::find_equal(&windows, &filtered);
    // filter for only peaks bigger than magnitude threshold
    let max_peak_locations: Vec<(usize, usize)> = peak_locations
        .iter()
        .filter(|&&loc| *windows.get(loc).unwrap() > args.magnitude_threshold)
        .map(|&loc| loc)
        .collect();

    // add track to track list
    let conn = database::connect(&args.database)?;

    // generate fingerprint
    let pair_records = hash::fingerprint(
        max_peak_locations,
        args.window_length,
        args.target_zone_delay_sec,
        args.target_zone_height_hz,
        args.target_zone_width_sec,
    );

    // retrieve all fingerprints with a matching hash, grouped by track_id

    todo!();

    // Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if let Some(action) = args.action {
        match action {
            Action::Add => {
                println!("Adding track to database.");
                add(&args)?;
            }
            Action::Match => {
                println!("Attempting to match sample to existing tracks");
                match_sample(&args)?;
            }
        }
    } else {
        let conn = database::connect(&args.database)?;
        let track_name = args
            .input_wav
            .file_stem()
            .context("Please provide a file not a directory.")?
            .to_string_lossy();
        let track_id = database::add_track(&conn, track_name.to_string().as_str())?;
        println!("Track {} added with id {}", track_name, track_id);
    }

    Ok(())
}
