use std::f32::consts::PI;

use image::{DynamicImage, GrayImage, ImageBuffer, Rgb, RgbImage};
use imageproc::{
    contours::find_contours_with_threshold,
    distance_transform::euclidean_squared_distance_transform, drawing, geometry::convex_hull,
    point::Point,
};

use crate::{
    algorithms::helpers::{absolute_contrast_threshold, filter_by_minimum_area},
    algorithms::Error,
    configuration::BacteriaExclusion,
};

pub fn bacteria_exclusion(
    input_image: &GrayImage,
    config: &BacteriaExclusion,
    scale: f32,
    debug: bool,
    output_prefix: &str,
) -> Result<f32, Box<dyn std::error::Error>> {
    // Find sharp contrasts in each direction individually and then absolutely combine
    // them to find the edges. This differs from doing it combined with a single kernel
    // in that it favors contrast in only one direction to better find graphene flakes.
    let (edges, edge_sharpness) =
        absolute_contrast_threshold(input_image, config.contrast_threshold);

    if debug {
        // Save the edge sharpness
        edge_sharpness.save(output_prefix.to_string() + "edge_sharpness.png")?;
    }

    // Filter the edges by area to remove noise
    let filtered_edges = filter_by_minimum_area(&edges, config.minimum_edge_area);

    if debug {
        // Visualize the edges overlayed with the original image
        let mut color_image: RgbImage = DynamicImage::ImageLuma8(input_image.clone()).into_rgb8();
        for (x, y, pixel) in color_image.enumerate_pixels_mut() {
            if 0 < filtered_edges.get_pixel(x, y).0[0] {
                pixel.0 = [0, 255, 255];
            }
        }

        color_image.save(output_prefix.to_string() + "graphene.png")?;
    }

    // Create a bacteria exclusion zone around all edges by thresholding the distance to the
    // closests detected edge
    let bacteria_exclusion_radius = config.exclusion_radius / scale;
    if bacteria_exclusion_radius < 1.0 {
        return Err(Box::new(Error::ToSmallExclusionDiameter));
    }

    let distances = euclidean_squared_distance_transform(&filtered_edges);
    let mut bacteria_exclusion_zone: GrayImage =
        ImageBuffer::new(input_image.width(), input_image.height());

    // Count the percentage of white in the bacteria whitemask
    let mut non_zero_count = 0;
    for (x, y, pixel) in bacteria_exclusion_zone.enumerate_pixels_mut() {
        if distances.get_pixel(x, y).0[0] < bacteria_exclusion_radius.into() {
            pixel.0[0] = 255;
            non_zero_count += 1;
        }
    }

    // Calculate a percentage of whiteness
    let mut bacteria_exclusion_ratio = non_zero_count as f32
        / (bacteria_exclusion_zone.width() * bacteria_exclusion_zone.height()) as f32;

    // Export images for insight into algorithm
    if debug {
        bacteria_exclusion_zone.save(output_prefix.to_string() + "bacteria-exclusion.png")?;
    }

    // calculate the bacteria exclusion adjusted from a radius sample. We assume the image is a
    // stiched version going from the edge to the center. The circle center is assumed to be
    // halfway down in the y axis and all the way to the right in x axis
    if config.radius_adjusted {
        // Identify the regions outside of the stitch
        let outer_contours = find_contours_with_threshold(input_image, 1);

        // Find the convex hull of all contours, this will give us a contour enclosing all the
        // given contours. All points have to be within this hull to be valid
        let hull: Vec<Point<u32>> = convex_hull(
            &outer_contours
                .into_iter()
                .flat_map(|c| c.points)
                .collect::<Vec<_>>(),
        );

        // Export the hull as a image
        if debug {
            let mut color_image: RgbImage =
                DynamicImage::ImageLuma8(input_image.clone()).into_rgb8();
            let mut previous_point = *hull.last().unwrap();
            for point in &hull {
                drawing::draw_line_segment_mut(
                    &mut color_image,
                    (previous_point.x as f32, previous_point.y as f32),
                    (point.x as f32, point.y as f32),
                    Rgb::<u8>([255, 0, 0]),
                );

                previous_point = *point;
            }

            color_image.save(output_prefix.to_string() + "radius_hull.png")?;
        }

        let mut radius_buckets = vec![(0.0, 0); input_image.width() as usize];
        'outer: for (x, y, pixel) in bacteria_exclusion_zone.enumerate_pixels() {
            // First make sure the point is within the stitched image and not in the outside margin
            let mut previous_point = *hull.last().unwrap();
            for point in &hull {
                // Side of the point relative to the line
                let line_distance = (previous_point.x as f32 - point.x as f32)
                    * (y as f32 - point.y as f32)
                    - (x as f32 - point.x as f32) * (previous_point.y as f32 - point.y as f32);

                // Update the previous point
                previous_point = *point;

                // If the line is one the wrong side of the line, skip this point
                if 0.0 <= line_distance {
                    continue 'outer;
                }
            }

            // The rounded distance from the current point to the center
            let distance = (((input_image.width() - x).pow(2)
                + (y - input_image.height() / 2).pow(2)) as f32)
                .sqrt()
                .round() as usize;

            // If the distance is outside our circle ignore it
            if radius_buckets.len() <= distance {
                continue;
            }

            // If the pixel is white add one to the radius bucket, otherwise nothing
            if pixel.0[0] == 255 {
                radius_buckets[distance].0 += 1.0;
            }

            radius_buckets[distance].1 += 1;
        }

        // Calculate the average of each radius bucket
        for (sum, count) in &mut radius_buckets {
            if 0 < *count {
                *sum /= *count as f32;
            }
        }

        // Sum all radiuses weighted by the distance from the center (gives the area in pixels of
        // excluded area)
        let mut bacteria_exclusion = 0.0;
        for (distance, (value, _)) in radius_buckets.iter().enumerate() {
            let slice_area = (distance as f32).powi(2) * PI - (distance as f32 - 1.0).powi(2) * PI;
            bacteria_exclusion += slice_area * value;
        }

        // Calculate the exclusion ratio
        bacteria_exclusion_ratio =
            bacteria_exclusion / ((input_image.width() as f32 - 1.0).powi(2) * PI);

        // Export all the radius buckets as a CSV
        let mut csv = csv::Writer::from_writer(
            std::fs::File::create(output_prefix.to_string() + "graphene_by_radius.csv")
                .expect("Failed to open CSV file"),
        );

        // Write header to file
        csv.write_record(["radial_distance", "ratio"])?;

        for (distance, (value, _)) in radius_buckets.iter().enumerate() {
            csv.write_record(&[
                format!("{}", (distance as f32) * scale),
                format!("{}", value),
            ])
            .expect("Failed to write angles");
        }
    }

    Ok(bacteria_exclusion_ratio)
}
