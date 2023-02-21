use git_version::git_version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct PreProcessing {
    pub equalize_histogram: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct TextRecognition {
    pub override_scale: bool,
    pub scale_bar_height: u32,
    pub override_scale_micrometers: f32,
    pub override_scale_pixels: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct BacteriaExclusion {
    pub enabled: bool,
    pub contrast_threshold: f32,
    pub minimum_edge_area: usize,
    pub exclusion_radius: f32,
    pub radius_adjusted: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct GrapheneAngles {
    pub enabled: bool,
    pub blur: f32,
    pub threshold: u8,
    pub min_graphene_size: f32,
    pub min_graphene_ratio: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Configuration {
    pub program_version: String,
    pub pre_processing: PreProcessing,
    pub text_recognition: TextRecognition,
    pub bacteria_exclusion: BacteriaExclusion,
    pub graphene_angles: GrapheneAngles,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            program_version: git_version!().to_string(),
            pre_processing: PreProcessing {
                equalize_histogram: false,
            },
            text_recognition: TextRecognition {
                override_scale: false,
                scale_bar_height: 0,
                override_scale_micrometers: 0.0,
                override_scale_pixels: 0,
            },
            bacteria_exclusion: BacteriaExclusion {
                enabled: true,
                contrast_threshold: 45.0,
                minimum_edge_area: 5,
                exclusion_radius: 0.9,
                radius_adjusted: false,
            },
            graphene_angles: GrapheneAngles {
                enabled: false,
                blur: 1.0,
                threshold: 150,
                min_graphene_size: 0.5,
                min_graphene_ratio: 3.0,
            },
        }
    }
}
