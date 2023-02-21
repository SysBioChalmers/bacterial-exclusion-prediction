use image::{DynamicImage, GrayImage};
use imageproc::{contours, contrast, drawing, filter, point::Point};
use plotters::prelude::{
    BitMapBackend, ChartBuilder, Circle, Color, IntoDrawingArea, Rectangle, BLACK, WHITE,
};

use std::{f32::consts, fs, ops::Range};

use crate::configuration::GrapheneAngles;

pub fn graphene_angles(
    input_image: &GrayImage,
    config: &GrapheneAngles,
    scale: f32,
    debug: bool,
    output_prefix: &str,
) -> Vec<f32> {
    // Blur and threshold the images to extract features from the background
    let mut mask = filter::gaussian_blur_f32(input_image, config.blur);
    contrast::threshold_mut(&mut mask, config.threshold);

    // Find the contours in the mask (should be contours of graphene)
    let contours: Vec<contours::Contour<u32>> = contours::find_contours(&mask);

    // Create a output image used in debug mode
    let mut furthest_points = DynamicImage::ImageLuma8(input_image.clone()).into_rgb8();

    // Find the average normal for every contour and display it using arrows
    let mut points_and_angles = Vec::new();
    let mut lengths = Vec::new();
    for contour in contours {
        // The normal of the line connecting the two furthest point in the contours is the angle of
        // the graphene flake, here we test only a subset of points to improve performance

        // Ignore all internal contours
        if contour.border_type == contours::BorderType::Hole {
            continue;
        }

        // Collect the sample points (one every Nth point)
        let sample_points: Vec<&Point<u32>> = contour.points.iter().step_by(5).collect();

        // Find the furthest two away points
        let mut point_1 = sample_points[0];
        let mut point_2 = sample_points[0];
        let mut current_maximum_distance = 0.0;
        for sample_p1 in &sample_points {
            for sample_p2 in &sample_points {
                let distance = ((sample_p1.x - sample_p2.x).pow(2) as f32
                    + (sample_p1.y - sample_p2.y).pow(2) as f32)
                    .sqrt();

                if current_maximum_distance < distance {
                    point_1 = sample_p1;
                    point_2 = sample_p2;

                    current_maximum_distance = distance;
                }
            }
        }

        // If the distance between the two points is below a threshold, ignore the entire contour
        if current_maximum_distance * scale < config.min_graphene_size {
            continue;
        }

        // Find the point furthest away from the line in the direction of the normal (or opposite)
        let mut maximum_distance_to_line = 0.0;
        let mut point_3 = sample_points[0];
        for point in &sample_points {
            let distance = ((point_2.y as f32 - point_1.y as f32)
                * (point_1.x as f32 - point.x as f32)
                - (point_1.y as f32 - point.y as f32) * (point_2.x as f32 - point_1.x as f32))
                .abs()
                / current_maximum_distance;

            if maximum_distance_to_line < distance {
                point_3 = point;
                maximum_distance_to_line = distance;
            }
        }

        // If the shape is too round ignore the contour as it is probably an error
        if current_maximum_distance / maximum_distance_to_line < config.min_graphene_ratio {
            continue;
        }

        // Find the center of the flake
        let center_x = point_1.x.abs_diff(point_2.x) as f32 / 2.0 + point_1.x.min(point_2.x) as f32;
        let center_y = point_1.y.abs_diff(point_2.y) as f32 / 2.0 + point_1.y.min(point_2.y) as f32;

        // Calculate the angle of the normal of the line between p1 and p2. We modulo PI as
        // completely opposite angles are the same for our purposes. Zero is horizontal and PI/2
        // is vertical. Positive angles are in the SE and NW directions while negative angles are
        // SW and NE
        let angle = consts::FRAC_PI_2
            - (point_2.y as f32 - point_1.y as f32)
                .atan2(point_2.x as f32 - point_1.x as f32)
                .rem_euclid(consts::PI);

        points_and_angles.push(((center_x, center_y), angle));
        lengths.push(current_maximum_distance * scale);

        if debug {
            // Draw the two furthest points
            drawing::draw_filled_circle_mut(
                &mut furthest_points,
                (point_1.x as i32, point_1.y as i32),
                2,
                image::Rgb::<u8>([0, 255, 0]),
            );

            drawing::draw_filled_circle_mut(
                &mut furthest_points,
                (point_2.x as i32, point_2.y as i32),
                2,
                image::Rgb::<u8>([255, 0, 0]),
            );

            // Draw the point furthest away on the normal of the line between p1 and p2
            drawing::draw_filled_circle_mut(
                &mut furthest_points,
                (point_3.x as i32, point_3.y as i32),
                2,
                image::Rgb::<u8>([0, 0, 255]),
            );

            // Draw the normal of the flake using an arrow
            let second_point_x = center_x + angle.cos() * 25.0;
            let second_point_y = center_y + angle.sin() * 25.0;

            drawing::draw_line_segment_mut(
                &mut furthest_points,
                (center_x, center_y),
                (second_point_x, second_point_y),
                image::Rgb::<u8>([255, 0, 255]),
            );
            drawing::draw_line_segment_mut(
                &mut furthest_points,
                (second_point_x, second_point_y),
                (
                    second_point_x - (angle + consts::FRAC_PI_8).cos() * 10.0,
                    second_point_y - (angle + consts::FRAC_PI_8).sin() * 10.0,
                ),
                image::Rgb::<u8>([255, 0, 255]),
            );
            drawing::draw_line_segment_mut(
                &mut furthest_points,
                (second_point_x, second_point_y),
                (
                    second_point_x - (angle - consts::FRAC_PI_8).cos() * 10.0,
                    second_point_y - (angle - consts::FRAC_PI_8).sin() * 10.0,
                ),
                image::Rgb::<u8>([255, 0, 255]),
            );
        }
    }

    if debug {
        furthest_points
            .save(output_prefix.to_string() + "angles.png")
            .unwrap();
    }

    // Vector of angles without radial length
    let angles: Vec<_> = points_and_angles.iter().map(|(_, a)| *a).collect();

    // Plot the histograms and export to a CSV files
    plot_angle_histogram(&angles, output_prefix);
    plot_length_histogram(&lengths, output_prefix);
    plot_angle_length_scatterplot(&angles, &lengths, output_prefix);

    // Save the angles as a CSV file
    let mut csv = csv::Writer::from_writer(
        fs::File::create(output_prefix.to_string() + "angles.csv")
            .expect("Failed to open CSV file"),
    );

    // Write header to file
    csv.write_record(["radial_distance", "angle"]).unwrap();

    for ((x, y), angle) in &points_and_angles {
        // The rounded distance in pixel from the current point (center of flake) to the center of
        // the radial sample
        let distance = ((input_image.width() as f32 - *x).powi(2)
            + (*y - input_image.height() as f32 / 2.0).powi(2))
        .sqrt()
        .round();

        csv.write_record(&[
            format!("{}", (distance as f32) * scale),
            format!("{:.3}", angle.to_degrees()),
        ])
        .expect("Failed to write angles");
    }

    csv.flush().unwrap();

    // Save the lengths as a CSV file
    let mut csv = csv::Writer::from_writer(
        fs::File::create(output_prefix.to_string() + "lengths.csv")
            .expect("Failed to open CSV file"),
    );

    for length in &lengths {
        csv.write_record(&[format!("{:.3}", length)])
            .expect("Failed to write lengths");
    }

    csv.flush().unwrap();

    angles
}

fn plot_length_histogram(lengths: &[f32], output_prefix: &str) {
    let mut max_length = 0.0;
    for length in lengths {
        max_length = length.max(max_length);
    }

    plot_histogram(
        lengths,
        0.0..max_length,
        25,
        "Length (μm)",
        "Count (number of flakes)",
        output_prefix
            .trim_start_matches("./output/")
            .trim_end_matches('_'),
        &(output_prefix.to_string() + "length-histogram.png"),
    );
}

fn plot_angle_histogram(angles: &[f32], output_prefix: &str) {
    // Convert all the angles to degrees centered at 0, between -90 and 90
    let mut scaled_angles = Vec::new();
    for angle in angles {
        scaled_angles.push(angle.to_degrees());
    }

    plot_histogram(
        &scaled_angles,
        -90.0..90.0,
        25,
        "Direction (°)",
        "Count (number of flakes)",
        output_prefix
            .trim_start_matches("./output/")
            .trim_end_matches('_'),
        &(output_prefix.to_string() + "angle-histogram.png"),
    );
}

fn plot_angle_length_scatterplot(raw_angles: &[f32], lengths: &[f32], output_prefix: &str) {
    // Convert all the angles to degrees centered at 0, between -90 and 90
    let mut angles = Vec::new();
    for angle in raw_angles {
        angles.push(angle.to_degrees());
    }

    // The max graphene length
    let mut max_length = 0.0;
    for length in lengths {
        max_length = length.max(max_length);
    }

    // Find the largest and smallest for both axis
    let caption = output_prefix
        .trim_start_matches("./output/")
        .trim_end_matches('_');

    let filepath = output_prefix.to_string() + "angle-length-scatterplot.png";
    let canvas = BitMapBackend::new(&filepath, (640, 480)).into_drawing_area();
    canvas.fill(&WHITE).unwrap();

    // Create a chart with a caption
    let mut chart = ChartBuilder::on(&canvas)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .caption(caption, ("sans-serif", 30))
        .margin(15)
        .build_cartesian_2d(-90.0..90.0_f32, 0.0..max_length)
        .unwrap();

    // Add X and Y labels to the chart
    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .x_desc("Angle (°)")
        .y_desc("Length (μm)")
        .axis_desc_style(("sans-serif", 15))
        .draw()
        .unwrap();

    chart
        .draw_series(
            angles
                .iter()
                .zip(lengths.iter())
                .map(|(angle, length)| Circle::new((*angle, *length), 5, BLACK.filled())),
        )
        .unwrap();

    // Export the plot
    canvas.present().expect("Failed to save plot to file");
}

fn plot_histogram(
    elements: &[f32],
    range: Range<f32>,
    bucket_count: usize,
    x_desc: &str,
    y_desc: &str,
    caption: &str,
    filepath: &str,
) {
    let bucket_size = (range.end - range.start) / bucket_count as f32;

    // Calculate frequency for every bucket
    let mut buckets = vec![0; bucket_count];
    let mut largest_bucket = 0;
    for element in elements {
        let index =
            (((element - range.start) / bucket_size).floor() as usize).clamp(0, buckets.len() - 1);

        buckets[index] += 1;

        if buckets[largest_bucket] < buckets[index] {
            largest_bucket = index;
        }
    }

    // Create a blank canvas with a white background
    let canvas = BitMapBackend::new(&filepath, (640, 480)).into_drawing_area();
    canvas.fill(&WHITE).unwrap();

    // Create a chart with a caption
    let mut chart = ChartBuilder::on(&canvas)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .caption(caption, ("sans-serif", 30))
        .margin(15)
        .build_cartesian_2d(
            range.clone(),
            0..(buckets[largest_bucket] as f32 * 1.2).round() as usize,
        )
        .unwrap();

    // Add X and Y labels to the chart
    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .x_desc(x_desc)
        .y_desc(y_desc)
        .axis_desc_style(("sans-serif", 15))
        .draw()
        .unwrap();

    // Draw the histogram using the frequencies
    chart
        .draw_series((0..).zip(buckets.iter()).map(|(x, y)| {
            let mut bar = Rectangle::new(
                [
                    (x as f32 * bucket_size + range.start, 0),
                    ((x + 1) as f32 * bucket_size + range.start, *y),
                ],
                BLACK.filled(),
            );

            bar.set_margin(0, 0, 2, 0);

            bar
        }))
        .unwrap();

    // Export the plot
    canvas.present().expect("Failed to save plot to file");
}
