use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert DICOM files to PNG
    Convert {
        /// Input folder containing DICOM files
        #[arg(short, long)]
        input: String,

        /// Output folder for PNG files
        #[arg(short, long)]
        output: String,

        /// Skip generating Excel metadata file
        #[arg(long, default_value_t = false)]
        skip_excel: bool,

        /// Flatten output directory structure
        #[arg(long, default_value_t = false)]
        flatten_output: bool,
    },
    /// Anonymize DICOM files
    Anonymize {
        /// Input folder containing DICOM files
        #[arg(short, long)]
        input: String,

        /// Output folder for anonymized DICOM files
        #[arg(short, long)]
        output: String,

        /// Tags to anonymize (format: "Group,Element", e.g., "0010,0010")
        /// Can be specified multiple times
        #[arg(short, long, value_parser = parse_tag)]
        tags: Vec<(u16, u16)>,

        /// Replacement value for anonymized tags
        #[arg(short, long, default_value = "ANONYMIZED")]
        replacement: String,
    },
}

fn parse_tag(s: &str) -> Result<(u16, u16), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid tag format: {}. Expected 'Group,Element' (hex)",
            s
        ));
    }
    let group = u16::from_str_radix(parts[0], 16).map_err(|e| format!("Invalid group: {}", e))?;
    let element =
        u16::from_str_radix(parts[1], 16).map_err(|e| format!("Invalid element: {}", e))?;
    Ok((group, element))
}

pub fn run_cli(cli: Cli) {
    match cli.command {
        Commands::Convert {
            input,
            output,
            skip_excel,
            flatten_output,
        } => {
            println!("Starting conversion...");
            println!("Input: {}", input);
            println!("Output: {}", output);

            let res = crate::logic::workflow::convert_dicom_to_png(
                std::path::Path::new(&input),
                std::path::Path::new(&output),
                !skip_excel,
                flatten_output,
                |progress| {
                    let percentage = if progress.total > 0 {
                        (progress.current as f64 / progress.total as f64) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "Progress: {}/{} ({:.1}%) - {} [{}]",
                        progress.current,
                        progress.total,
                        percentage,
                        progress.filename,
                        progress.status
                    );
                },
                |log| {
                    println!("[{}] {}", log.status, log.message);
                },
            );

            match res {
                Ok(report) => {
                    println!("Conversion completed successfully!");
                    println!("Total: {}", report.total);
                    println!("Successful: {}", report.successful);
                    println!("Skipped: {}", report.skipped_non_image);
                    println!("Failed: {}", report.failed);
                    println!("Output folder: {:?}", report.output_folder);
                }
                Err(e) => {
                    eprintln!("Conversion failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Anonymize {
            input,
            output,
            tags,
            replacement,
        } => {
            println!("Starting anonymization...");
            println!("Input: {}", input);
            println!("Output: {}", output);
            println!("Tags: {:?}", tags);

            let res = crate::logic::anonymize::anonymize_dicom(
                std::path::Path::new(&input),
                std::path::Path::new(&output),
                tags,
                replacement,
                |progress| {
                    let percentage = if progress.total > 0 {
                        (progress.current as f64 / progress.total as f64) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "Progress: {}/{} ({:.1}%) - {} [{}]",
                        progress.current,
                        progress.total,
                        percentage,
                        progress.filename,
                        progress.status
                    );
                },
                |log| {
                    println!("[{}] {}", log.status, log.message);
                },
            );

            match res {
                Ok(report) => {
                    println!("Anonymization completed successfully!");
                    println!("Total: {}", report.total);
                    println!("Successful: {}", report.successful);
                    println!("Skipped: {}", report.skipped);
                    println!("Failed: {}", report.failed);
                    println!("Output folder: {:?}", report.output_folder);
                }
                Err(e) => {
                    eprintln!("Anonymization failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
