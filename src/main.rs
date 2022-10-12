//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use clap::Parser;
use sonogram::ColourGradient;
use sonogram::ColourTheme;
use sonogram::FrequencyScale;
use sonogram::SpecOptionsBuilder;
use std::cmp::Ordering;
use std::fs::File;
use std::path::PathBuf;

struct SpectogramImage {
    data: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl SpectogramImage {
    fn get_val(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }
    fn get_square(&self, x: usize, y: usize, width: usize) -> Vec<f32> {
        // returns values, centred on x,y coord
        let mut data = vec![];
        let min_x = if x < width / 2 { 0 } else { x - width / 2 };
        let max_x = if x + width / 2 > self.width {self.width-1} else {x + width / 2 - 1};
        let min_y = if y < width / 2 { 0 } else { y - width / 2 };
        let max_y = if y + width / 2 > self.height {self.height-1} else {y + width / 2 - 1};
        // println!("{} {} {} {}", min_x, max_x, min_y, max_y);
        for i in min_x..=max_x {
            for j in min_y..=max_y {
                // data.push(self.get_val(i, j));
            }
        }
        for i in 0..self.width {
            for j in 0..self.height {
                data.push(self.get_val(i, j));
            }
        }
        data
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, parse(from_os_str), default_value = "db.sqlite")]
    database: PathBuf,

    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    input_wav: PathBuf,
}

// Reads wav file, creates a spectrogram, saves this to a png and to a buffer in memory.
fn read_wav_to_png(input_wav: &PathBuf) -> Result<SpectogramImage, anyhow::Error> {
    // Read in the file info to determine length
    let mut inp_file = File::open(input_wav)?;
    let (header, data) = wav::read(&mut inp_file)?;
    println!("{:?}", header);
    let data_16 = data.as_sixteen().unwrap();

    // Compute the spectrogram giving the number of bins and the window overlap.
    let spec_builder = SpecOptionsBuilder::new(2048)
        .load_data_from_file(input_wav)
        .unwrap()
        .normalise();
    let mut spectrograph = spec_builder.build().unwrap().compute();

    // Save the spectrogram to PNG.
    let time_scale = 1000;
    let spec_width = data_16.len() / time_scale;
    let spec_height = 600;
    let png_file = std::path::Path::new("sonogram.png");
    let mut gradient = ColourGradient::create(ColourTheme::Rainbow);
    spectrograph.to_png(
        &png_file,
        FrequencyScale::Linear,
        &mut gradient,
        spec_width,  // Width
        spec_height, // Height
    )?;
    let spec_buffer =
        spectrograph.to_buffer(sonogram::FrequencyScale::Linear, spec_width, spec_height);

    Ok(SpectogramImage {
        data: spec_buffer,
        width: spec_width,
        height: spec_height,
    })
}

fn max_filter(spec: &SpectogramImage, kernel_size: usize) -> Vec<f32> {
    let mut filtered = vec![];
    for y in 0..spec.height {
        for x in 0..spec.width {
            let square = spec.get_square(x, y, kernel_size);
            let max = square.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            let max = spec.get_val(x, y);
            // match max {
            //     None => panic!("No maximum found"),
            //     Some(max) => filtered.push(*max),
            // }
            filtered.push(max);
        }
    }
    filtered
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let _database_path = PathBuf::from(args.database);
    let spectrograph = read_wav_to_png(&args.input_wav)?;

    let kernel_size = 5;

    dbg!(spectrograph.width, spectrograph.height);

    // let filtered = max_filter(&spectrograph, kernel_size);
    let filtered = spectrograph.data.clone();

    let spec_builder = SpecOptionsBuilder::new(2048).load_data_from_memory_f32(filtered, 11025);
    let mut s = spec_builder.build().unwrap().compute();

    // Save the spectrogram to PNG.
    let png_file = std::path::Path::new("sonogram_max.png");
    let mut gradient = ColourGradient::create(ColourTheme::Rainbow);
    s.to_png(
        &png_file,
        FrequencyScale::Linear,
        &mut gradient,
        spectrograph.width,  // Width
        spectrograph.height, // Height
    )?;

    Ok(())
}
