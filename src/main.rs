#![warn(clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unused_async
)]

use clap::Parser;
use git_version::git_version;
use image::open;
use rayon::prelude::*;

use std::ffi::OsStr;
use std::fmt::{Debug, Write};
use std::{fs, net::SocketAddr, path::PathBuf};

use crate::algorithms::{bacteria_exclusion, determine_scale, graphene_angles, pre_processing};
use crate::configuration::Configuration;

/// The module containing all the actual algorithms
mod algorithms;

/// The module containing the different configuration parameters
mod configuration;

/// The module containing the interactive interface
mod web;

fn main() {
    let args = Args::parse();

    // Create a output directory if non exist
    fs::create_dir_all("./output/").expect("Failed to create output directory");

    // Run in interactive mode on "127.0.0.1:8080" if no action got provided
    let action = match args.action {
        Some(action) => action,
        None => Action::Interactive {
            address: "127.0.0.1:8080".parse().unwrap(),
        },
    };

    match action {
        Action::Analyse { config, path } => {
            // Load the configuration file (or use the default)
            let config = if let Some(path) = config {
                toml::from_str(&fs::read_to_string(path).expect("Failed to read the config file"))
                    .expect("Couldn't parse the config file as TOML")
            } else {
                Configuration::default()
            };

            // Warn about config using another version of the program
            if config.program_version != git_version!() {
                eprintln!("Warning: the config you have provided was made by another version of the program. It might not reproduce the same results (config: {}, program: {})", config.program_version, git_version!());
            }

            single(&config, &path);
        }
        Action::Batch {
            config,
            discard_error,
            path,
        } => {
            // Load the configuration file (or use the default)
            let config = if let Some(path) = config {
                toml::from_str(&fs::read_to_string(path).expect("Failed to read the config file"))
                    .expect("Couldn't parse the config file as TOML")
            } else {
                Configuration::default()
            };

            // Warn about config using another version of the program
            if config.program_version != git_version!() {
                eprintln!("Warning: the config you have provided was made by another version of the program. It might not reproduce the same results (config: {}, program: {})", config.program_version, git_version!());
            }

            batch(&config, path, discard_error);
        }
        Action::Interactive { address } => {
            web::start(address);
        }
        Action::Export { path } => {
            // Parse default configuration to TOML
            let config_string = toml::to_string_pretty(&Configuration::default())
                .expect("Failed to serialize default configuration");

            // Write string to the provided path
            fs::write(path, config_string).expect("Couldn't write to config file");
        }
    }
}

fn single(config: &Configuration, path: &PathBuf) {
    // Load image
    let image = open(path).expect("Could not load input image").to_luma8();

    // Create an output prefix from the filename
    let output_prefix = "./output/".to_string() + path.file_stem().unwrap().to_str().unwrap() + "_";

    // Determine scale (um / px)
    let (scale, um, px, scale_bar_height, image) =
        determine_scale(image, &config.text_recognition, true, &output_prefix)
            .expect("Failed to determine scale of image");
    println!(
        "Scale: {:.4} (px: {}, um: {}, height: {})",
        scale, px, um, scale_bar_height
    );

    // Preprocessing
    let image = pre_processing(image, config.pre_processing);

    // Find graphene and determine bacteria exclusion percentage
    if config.bacteria_exclusion.enabled {
        let bacteria_exclusion_ratio = bacteria_exclusion(
            &image,
            &config.bacteria_exclusion,
            scale,
            true,
            &output_prefix,
        )
        .expect("Calculating bacteria exclusion failed");

        println!(
            "Area within range of graphene edge (for correlation): {:.2}%",
            100.0 * bacteria_exclusion_ratio
        );
    }

    // Find angles of graphene in the image
    if config.graphene_angles.enabled {
        graphene_angles(&image, &config.graphene_angles, scale, true, &output_prefix);
    }

    // Write the configuration to the output directory
    fs::write(
        output_prefix + "config.toml",
        toml::to_string_pretty(&config).expect("Failed to serialize default configuration"),
    )
    .expect("Couldn't write to config file");
}

fn batch(config: &Configuration, path: PathBuf, discard_error: bool) {
    // Determine all target images (within the target directory)
    println!("Targets");
    let mut targets = Vec::new();
    for path in fs::read_dir(path).expect("Failed to read the given directory, does it exist?") {
        let path = path.unwrap().path();

        let extension = path.extension().and_then(OsStr::to_str);
        if extension == Some("tif") || extension == Some("tiff") {
            targets.push(path);
        }
    }

    // Sort the images in alphabetical order for easier interpretation
    targets.sort_unstable();

    // Print the map between image paths and ids
    for (i, path) in targets.iter().enumerate() {
        println!(" - {i}: {}", path.display());
    }

    println!();

    // Run the targets in parallel and aggregate statistics
    let bacteria_exclusion_ratios: Vec<f32> = targets
        .par_iter()
        .enumerate()
        .filter_map(|(i, target)| -> Option<f32> {
            // Load image
            let image = open::<&PathBuf>(target)
                .unwrap_or_else(|_| panic!("Could not load image {}", target.display()))
                .to_luma8();

            // Create an output prefix from the filename
            let output_prefix =
                "./output/".to_string() + target.file_stem().unwrap().to_str().unwrap() + "_";

            // Create an output string which progressively gets more information, one for each stage
            let mut output_string = format!("{i}: ");

            // Determine scale (um / px)
            let (scale, um, px, scale_bar_height, image) =
                match determine_scale(image, &config.text_recognition, true, &output_prefix) {
                    Ok(result) => result,
                    Err(e) => {
                        let message = format!(
                            "{}: Failed to determine scale of image {} ({})",
                            i,
                            target.display(),
                            e
                        );

                        if discard_error {
                            println!("{message}");
                            return None;
                        }

                        panic!("{}", message);
                    }
                };
            write!(
                output_string,
                "Scale: {}um / {}px ({}). ",
                um, px, scale_bar_height
            )
            .unwrap();

            // Preprocessing
            let image = pre_processing(image, config.pre_processing);

            // Find angles of graphene in the image
            if config.graphene_angles.enabled {
                graphene_angles(&image, &config.graphene_angles, scale, true, &output_prefix);

                write!(
                    output_string,
                    "Calculated graphene angles, see image or .csv file. ",
                )
                .unwrap();
            }

            // Find graphene and determine bacteria exclusion percentage
            let mut return_status = None;
            if config.bacteria_exclusion.enabled {
                let bacteria_exclusion_ratio = match bacteria_exclusion(
                    &image,
                    &config.bacteria_exclusion,
                    scale,
                    true,
                    &output_prefix,
                ) {
                    Ok(result) => result,
                    Err(e) => {
                        let message = format!(
                            "{} Failed to calculate bacteria exclusion for {} ({})",
                            i,
                            target.display(),
                            e
                        );

                        if discard_error {
                            println!("{message}");
                            return None;
                        }

                        panic!("{}", message);
                    }
                };
                write!(
                    output_string,
                    "Graphene edge area: {:.2}%. ",
                    100.0 * bacteria_exclusion_ratio
                )
                .unwrap();

                return_status = Some(bacteria_exclusion_ratio * 100.0);
            }

            println!("{output_string}");

            // Write the configuration to the output directory
            fs::write(
                output_prefix + "config.toml",
                toml::to_string_pretty(&config).expect("Failed to serialize default configuration"),
            )
            .expect("Couldn't write to config file");

            return_status
        })
        .collect();

    println!(
        "\nImages, both output and intermediates, have been exported to the 'output' directory"
    );

    // Print out aggregated statistics
    println!("\nAggregated statistics:");
    if config.bacteria_exclusion.enabled {
        let mean = mean(&bacteria_exclusion_ratios);

        println!(
            " - Mean graphene edge exclusion area: {:.2}% (standard deviation: {:.5})",
            mean,
            standard_deviation(&bacteria_exclusion_ratios, mean)
        );
    }
}

/// The mean (average) of the input values
fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len() as f32
}

/// The standard deviation of the input values
fn standard_deviation(values: &[f32], mean: f32) -> f32 {
    (values.iter().fold(0.0, |sum, x| sum + (x - mean).powi(2)) / values.len() as f32).sqrt()
}

/// Analyse SEM pictures of graphene and bacteria to determine some metrics
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The action to perform, runs in interactive mode on port 8080 if nothing is provided
    #[clap(subcommand)]
    action: Option<Action>,
}

#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Analyse a single image
    Analyse {
        /// The path to the configuration file to load (TOML)
        #[clap(short, long, value_parser)]
        config: Option<PathBuf>,
        /// The path to the image to analyse
        #[clap(value_parser)]
        path: PathBuf,
    },
    /// Analyse all images in a folder and aggregate the result
    Batch {
        /// The path to the configuration file to load (TOML)
        #[clap(short, long, value_parser)]
        config: Option<PathBuf>,
        /// Discard all images that error in some way
        #[clap(short, long)]
        discard_error: bool,
        /// The path to the directory containing the images
        #[clap(value_parser)]
        path: PathBuf,
    },
    /// Start a web interface allowing for easy fine tuning of parameters
    Interactive {
        /// The address to serve the interface on
        #[clap(default_value = "127.0.0.1:8080")]
        address: SocketAddr,
    },
    /// Exports the default configuration
    Export {
        /// The path to write default configuration to
        #[clap(value_parser)]
        path: PathBuf,
    },
}
