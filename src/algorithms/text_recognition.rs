use image::{imageops, DynamicImage, GrayImage, Luma, RgbImage};
use imageproc::{contours, contrast, drawing, filter};

use std::{collections::HashMap, process::Command};

use crate::{algorithms::Error, configuration::TextRecognition};

pub fn determine_scale(
    mut input_image: GrayImage,
    config: &TextRecognition,
    debug: bool,
    output_prefix: &str,
) -> Result<(f32, f32, u32, u32, GrayImage), Error> {
    const SKIP_BOTTOM_PIXELS: u32 = 40;

    let width = input_image.width();
    let height = input_image.height();

    let scale_bar_height = if config.override_scale {
        height - config.scale_bar_height
    } else {
        // Show all the heights if debug
        let mut visualized_heights: RgbImage =
            DynamicImage::ImageLuma8(input_image.clone()).into_rgb8();

        // Determine the height of the text region by looking for a big change in the total lightness
        // of all non-white pixels in each row. Skip the first n pixels to try to skip white letters
        let mut heights = HashMap::new();
        for x in 0..width {
            let mut previous_brightness =
                input_image.get_pixel(x, height - SKIP_BOTTOM_PIXELS - 1).0[0];
            for y in (0..height - SKIP_BOTTOM_PIXELS).rev() {
                let brightness = input_image.get_pixel(x, y).0[0];

                // Ignore completely white pixels, or pixels where the previous pixel were brighter
                if 200 < previous_brightness || brightness < previous_brightness {
                    continue;
                }

                if 16 < brightness - previous_brightness {
                    if debug {
                        visualized_heights.get_pixel_mut(x, y).0 = [0, 255, 0];
                    }

                    // Count the number of this height
                    if let Some(previous_count) = heights.get(&y) {
                        heights.insert(y, previous_count + 1);
                    } else {
                        heights.insert(y, 1);
                    }

                    break;
                }

                previous_brightness = brightness;
            }
        }

        // Get the mode of the heights
        let mut scale_bar_height = 0;
        let mut mode_count = 0;
        for (height, value) in &heights {
            if mode_count < *value {
                scale_bar_height = *height;
                mode_count = *value;
            }
        }

        // Export the debug image
        if debug {
            visualized_heights
                .save(output_prefix.to_string() + "heights.png")
                .unwrap();
        }

        scale_bar_height
    };

    // Get the image without the bottom bar
    let image_without_text =
        imageops::crop(&mut input_image, 0, 0, width, scale_bar_height).to_image();

    // If scale is overwritten return early
    if config.override_scale {
        return Ok((
            config.override_scale_micrometers / (config.override_scale_pixels as f32),
            config.override_scale_micrometers,
            config.override_scale_pixels,
            config.scale_bar_height,
            image_without_text,
        ));
    }

    // Retrieve the bottom part of the image and make it black and white (by threshold)
    let mut image = imageops::crop(
        &mut input_image,
        width - 650,
        scale_bar_height,
        650,
        height - scale_bar_height,
    )
    .to_image();
    contrast::threshold_mut(&mut image, 240);

    // Erode the image to remove lines that are 1 pixel thick (as in some images)
    //morphology::open_mut(&mut image, distance_transform::Norm::LInf, 1);

    // Find all contours (lines) and remove those that are to short. At the same time find
    // the x coordinates of the most extreme lines to determine the scale. We also keep track
    // of the other y coordinate and the other x coordinate of the lines to later center the text
    // recognition around the actual text for a higher success rate with lower noise.
    let contours: Vec<contours::Contour<u32>> = contours::find_contours(&image);

    // Here the first value is the maximum / minimum x. The second one is the other x coordinate
    // of the line and the final one is the y coordinate (should be the same on both ends and lines)
    let mut minimum_line = (u32::MAX, 0, 0);
    let mut maximum_line = (u32::MIN, 0, 0);
    for contour in contours {
        // Go through each point in the contour and try to create the longest possible horizontal line
        let mut line_start = contour.points[0];
        let mut line_end = contour.points[0];

        for point in contour.points {
            // See if the point lays on the current line, if so add to the current line
            if point.y == line_start.y {
                line_end = point;
            // Close the last line and create a new
            } else {
                // Sort the two points by x coordinate
                let (minimum_point, maximum_point) = if line_start.x < line_end.x {
                    (line_start, line_end)
                } else {
                    (line_end, line_start)
                };

                // Make sure the line is long enough
                if maximum_point.x - minimum_point.x < 16 {
                    line_start = point;
                    line_end = point;

                    continue;
                }

                if debug {
                    drawing::draw_line_segment_mut(
                        &mut image,
                        (minimum_point.x as f32, minimum_point.y as f32),
                        (maximum_point.x as f32, maximum_point.y as f32),
                        Luma::<u8>([64]),
                    );
                }

                // Update the minimum line if needed
                if minimum_point.x < minimum_line.0 {
                    minimum_line = (minimum_point.x, maximum_point.x, minimum_point.y);
                }

                // Update the maximum line if needed
                if maximum_point.x > maximum_line.0 {
                    maximum_line = (maximum_point.x, minimum_point.x, maximum_point.y);
                }

                line_end = point;
                line_start = point;
            }
        }
    }

    if debug {
        image.save(output_prefix.to_string() + "lines.png").unwrap();
    }

    // Make sure that at least two lines accepted were found
    if minimum_line.0 == u32::MAX || maximum_line.0 == u32::MIN {
        return Err(Error::LessThenTwoApplicableLinesFound);
    }

    // Check that the combined line is horizontal enough (a max of 1:20 ratio)
    if maximum_line.0.abs_diff(minimum_line.0) < maximum_line.2.abs_diff(minimum_line.2) * 20 {
        return Err(Error::ExtremeLineIsNonHorizontal);
    }

    // The pixel distance is the difference between the extreme lines
    let pixel_distance = maximum_line.0 - minimum_line.0;

    // Crop the image to extract just the text part to make it as easy as possible
    // for Tesseract to recognize the text. Threshold it at the same time
    let mut image = imageops::crop(
        &mut input_image,
        width - 650 + minimum_line.1 + 2,
        scale_bar_height + maximum_line.2 - 18,
        maximum_line.1 - minimum_line.1 - 4,
        45,
    )
    .to_image();
    contrast::threshold_mut(&mut image, 240);
    let image = filter::gaussian_blur_f32(&image, 1.0);

    // A bit of a hack but here we export the image to a file so that Tesseract then
    // easily can analyse it
    let scale_path = output_prefix.to_string() + "scale.png";
    image.save(&scale_path).unwrap();

    // Run command line Tesseract to be platform agnostic
    let text = std::str::from_utf8(
        &Command::new("tesseract")
            .args([
                &scale_path,
                "stdout",
                "-l",
                "eng",
                "--psm",
                "7",
                "-c",
                "tessedit_char_whitelist=\"1234567890um\"",
            ])
            .output()
            .expect("Couldn't run Tesseract, is it on path and installed?")
            .stdout,
    )
    .unwrap()
    .to_string();

    // The text should end with "um" (or "mm"), start with a combination of digits and optionally
    // whitespace in between or on the outside of the string. This is validated below
    let micrometer_distance: f32 = if let Some(digits) = text.trim().strip_suffix("um") {
        match digits.trim().parse::<f32>() {
            Ok(distance) => distance,
            Err(_) => return Err(Error::FailedToDetectText(text.clone())),
        }
    } else {
        return Err(Error::FailedToDetectText(text.clone()));
    };

    // Return the scale (micrometer / pixel) and the image without the text at the bottom
    Ok((
        micrometer_distance / (pixel_distance as f32),
        micrometer_distance,
        pixel_distance,
        height - scale_bar_height,
        image_without_text,
    ))
}
