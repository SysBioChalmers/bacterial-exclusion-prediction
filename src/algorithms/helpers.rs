use image::{GrayImage, ImageBuffer, Luma};
use imageproc::{contours, drawing::draw_polygon_mut, point::Point};

/// Helper function that removes all contours that have to few pixels (determined by the minimum
/// area)
pub fn filter_by_minimum_area(mask: &GrayImage, minimum_area: usize) -> GrayImage {
    // Derive the mask contours for denoising in the next step
    let mut contours: Vec<contours::Contour<u32>> = contours::find_contours(mask);

    // Calculate the area of all the contours and remove those that are to small. This is to
    // reduce noise in the image
    contours.retain(|contour| {
        // Calculate the area of the polygon
        let mut area = 0.0;
        let mut previous_point = contour.points.last().unwrap();
        for point in &contour.points {
            area +=
                (previous_point.x + point.x) as f32 * (previous_point.y as f32 - point.y as f32);

            previous_point = point;
        }

        area = (area / 2.0).abs();

        // Remove all contours with to small of an area
        minimum_area < area.round() as usize
    });

    // Create a new mask where only the contours left are drawn
    let mut denoised_mask: GrayImage = ImageBuffer::new(mask.width(), mask.height());
    for contour in contours {
        draw_polygon_mut(
            &mut denoised_mask,
            &contour
                .points
                .into_iter()
                .map(|point| Point::new(point.x as i32, point.y as i32))
                .collect::<Vec<Point<i32>>>(),
            Luma::from([255]),
        );
    }

    denoised_mask
}

/// Helper that calculates the thresholded absolute contrast of a input image. Returns (threshold,
/// contrast)
///
/// Find sharp contrasts in each direction individually and then absolutely combine
/// them to find the edges. This differs from doing it combined with a single kernel
/// in that it favors contrast in only one direction to better find graphene flakes.
pub fn absolute_contrast_threshold(image: &GrayImage, threshold: f32) -> (GrayImage, GrayImage) {
    let mut contrast: GrayImage = ImageBuffer::new(image.width(), image.height());
    let mut thresholded_contrast: GrayImage = ImageBuffer::new(image.width(), image.height());
    for (x, y, pixel) in thresholded_contrast.enumerate_pixels_mut() {
        // Go through each opposite pair of pixels surrounding the current pixel. We do every pair
        // twice altough it shouldn't matter
        let mut summed_difference = 0.0;
        let mut count = 0;
        for vx in -1..2 {
            for vy in -1..2 {
                // Skip the current pixel
                if vx == 0 && vy == 0 {
                    continue;
                }

                let pixel = f32::from(
                    image
                        .get_pixel(
                            (x as i32 + vx).clamp(0, image.width() as i32 - 1) as u32,
                            (y as i32 + vy).clamp(0, image.height() as i32 - 1) as u32,
                        )
                        .0[0],
                );

                let opposite = f32::from(
                    image
                        .get_pixel(
                            (x as i32 - vx).clamp(0, image.width() as i32 - 1) as u32,
                            (y as i32 - vy).clamp(0, image.height() as i32 - 1) as u32,
                        )
                        .0[0],
                );

                // Calculate the "absolute difference" in all directions
                let absolute_difference = (opposite - pixel).abs();

                summed_difference += absolute_difference;
                count += 1;
            }
        }

        // Average the absolute difference sum to keep it between 0 and 255
        let absolute_difference = summed_difference / count as f32;

        // Here we threshold at the same time to not have to iterate through the image twice.
        // This would yield the same result as first finding contrasts and then threshold them
        pixel.0[0] = if threshold < absolute_difference {
            255
        } else {
            0
        };

        // Save the absolute difference
        contrast.get_pixel_mut(x, y).0[0] = absolute_difference.round() as u8;
    }

    (thresholded_contrast, contrast)
}
