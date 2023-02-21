use std::fmt;

mod bacteria_exclusion;
mod graphene_angles;
mod helpers;
mod pre_processing;
mod text_recognition;

// Rexport all functions
pub use bacteria_exclusion::bacteria_exclusion;
pub use graphene_angles::graphene_angles;
pub use pre_processing::pre_processing;
pub use text_recognition::determine_scale;

#[derive(Debug)]
pub enum Error {
    FailedToDetectText(String),
    ToSmallExclusionDiameter,
    ExtremeLineIsNonHorizontal,
    LessThenTwoApplicableLinesFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::FailedToDetectText(detected) => format!("Couldn't detect the scale using OCR (detected: {detected})"),
                Error::ToSmallExclusionDiameter => "The bacteria exclusion diameter is smaller than 1 pixel which effectively makes it non-existent".to_string(),
                Error::ExtremeLineIsNonHorizontal => "The two lines creating the scale are not on the same y level".to_string(),
                Error::LessThenTwoApplicableLinesFound => "Less then two lines that meet the requirements were found when trying to detect scale".to_string(),
            }
        )
    }
}

impl std::error::Error for Error {}
