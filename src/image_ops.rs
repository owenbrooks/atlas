use std::path::PathBuf;

use ndarray::{Array2, Array, s, ArrayView2};
use ndarray_stats::QuantileExt;
use plotters::prelude::*;


pub fn save_png(windows: &Array2<f32>, output_path: PathBuf) {
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

pub fn max_filter(spec: &Array2<f32>, kernel_size: usize) -> Array2<f32> {
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

pub fn find_equal(array_a: &Array2<f32>, array_b: &Array2<f32>) -> Vec<(usize, usize)> {
    let mut locations = vec![];
    for (loc, elem) in array_a.indexed_iter() {
        if *array_b.get(loc).unwrap() == *elem {
            locations.push(loc);
        }
    }

    locations
}

pub fn plot_peaks(
    peak_locations: &[(usize, usize)],
    height: usize,
    width: usize,
    samples_per_second: usize,
    output_path: PathBuf,
) -> Result<(), anyhow::Error> {
    println!("Plotting peaks");
    dbg!(height, width);
    let root = BitMapBackend::new(&output_path, (width as u32 / 2 + 2000, height as u32 / 2 + 40))
        .into_drawing_area();

    root.fill(&WHITE)?;

    let areas = root.split_by_breakpoints([width as u32 - 40], [40]);

    let mut scatter_ctx = ChartBuilder::on(&areas[2])
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0..width*height/(samples_per_second*10), 0..height*5)?;
    scatter_ctx
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .draw()?;
    scatter_ctx.draw_series(
        peak_locations
            .iter()
            .map(|(x, y)| Circle::new((*x*height/(samples_per_second*10), *y*10/2), 2, GREEN.filled())),
    )?;

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", output_path.to_string_lossy());

    Ok(())
}