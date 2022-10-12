//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use sonogram::ColourGradient;
use sonogram::ColourTheme;
use sonogram::FrequencyScale;
use sonogram::SpecOptionsBuilder;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);

    // Compute the spectrogram giving the number of bins and the window overlap.
    let spec_builder = SpecOptionsBuilder::new(2048)
        .load_data_from_file(&args.input_wav)
        .unwrap();
    let mut spectrograph = spec_builder.build().unwrap().compute();

    // Save the spectrogram to PNG.
    let png_file = std::path::Path::new("sonogram.png");
    let mut gradient = ColourGradient::create(ColourTheme::Default);
    spectrograph
        .to_png(
            &png_file,
            FrequencyScale::Linear,
            &mut gradient,
            512, // Width
            512, // Height
        )
        .unwrap();

    Ok(())
}
