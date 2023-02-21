use image::GrayImage;
use imageproc::contrast::equalize_histogram_mut;

use crate::configuration::PreProcessing;

pub fn pre_processing(mut input_image: GrayImage, config: PreProcessing) -> GrayImage {
    if config.equalize_histogram {
        equalize_histogram_mut(&mut input_image);
    };

    input_image
}
