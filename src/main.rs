//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use sonogram::ColourGradient;
use sonogram::ColourTheme;
use sonogram::FrequencyScale;
use sonogram::SpecOptionsBuilder;
use std::fs::File;
use std::path::PathBuf;

struct SpectogramImage {
    data: Vec<f32>,
    width: usize,
    height: usize,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,
}

fn read_wav_to_png(input_wav: &PathBuf) -> Result<SpectogramImage, anyhow::Error> {
    // Read in the file info to determine length
    let mut inp_file = File::open(input_wav)?;
    let (header, data) = wav::read(&mut inp_file)?;
    println!("{:?}", header);
    let data_16 = data.as_sixteen().unwrap();

    // Compute the spectrogram giving the number of bins and the window overlap.
    let spec_builder = SpecOptionsBuilder::new(2048)
        .load_data_from_file(input_wav)
        .unwrap();
    let mut spectrograph = spec_builder.build().unwrap().compute();

    // Save the spectrogram to PNG.
    let time_scale = 1000;
    let spec_width = data_16.len() / time_scale;
    let spec_height = 512;
    let png_file = std::path::Path::new("sonogram.png");
    let mut gradient = ColourGradient::create(ColourTheme::Default);
    spectrograph.to_png(
        &png_file,
        FrequencyScale::Linear,
        &mut gradient,
        spec_width,  // Width
        spec_height, // Height
    )?;
    let spec_buffer = spectrograph.to_buffer(sonogram::FrequencyScale::Linear, spec_width, spec_height);

    Ok(SpectogramImage {
        data: spec_buffer,
        width: spec_width,
        height: spec_height,
    })
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);
    let spectrograph = read_wav_to_png(&args.input_wav)?;

    println!("{:?}", &spectrograph.data);

    Ok(())
}
