use crate::logic::convert::{convert_single_file, FileOutcome};
use crate::models::metadata::FileMetadata;
use crate::utils::discovery::collect_dicom_files;
use crate::utils::excel::write_metadata_workbooks;
use crate::utils::logging::{write_logs, LogEntry};
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, serde::Serialize)]
pub struct ConversionReport {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped_non_image: usize,
    pub failed_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub output_folder: PathBuf,
}

#[derive(Clone, serde::Serialize)]
pub struct ProgressPayload {
    pub current: usize,
    pub total: usize,
    pub filename: String,
    pub status: String,
}

pub fn convert_dicom_to_png<F, G>(
    input_folder: &Path,
    output_folder: &Path,
    save_excel: bool,
    flatten_output: bool,
    progress_callback: F,
    log_callback: G,
) -> Result<ConversionReport>
where
    F: Fn(ProgressPayload) + Sync + Send,
    G: Fn(LogEntry) + Sync + Send,
{
    if !input_folder.exists() {
        bail!("Input folder '{}' does not exist", input_folder.display());
    }

    // Determine the input folder name for the output directory
    let input_name = input_folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dicom");

    let root_output_path = if flatten_output {
        output_folder.to_path_buf()
    } else {
        output_folder.join(format!("{}_output", input_name))
    };
    let png_output_path = root_output_path.join("png_file");

    fs::create_dir_all(&png_output_path).with_context(|| {
        format!(
            "Unable to create output folder {}",
            png_output_path.display()
        )
    })?;

    let dicom_files = collect_dicom_files(input_folder);
    let mut tasks = Vec::new();

    for path in dicom_files {
        let png_path = build_png_path(input_folder, &png_output_path, &path);
        // Removed pre-check: if png_path.exists() { continue; }

        let folder_relative = path
            .parent()
            .and_then(|p| p.strip_prefix(input_folder).ok())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        tasks.push((path, png_path, folder_relative));
    }

    let total = tasks.len();
    let mut successful = 0usize;
    let mut failed_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut all_metadata = Vec::new();
    let mut folder_metadata: BTreeMap<PathBuf, Vec<FileMetadata>> = BTreeMap::new();
    let mut logs: Vec<LogEntry> = Vec::new();
    let mut skipped_count = 0usize;

    let processed_count = AtomicUsize::new(0);

    let results: Vec<_> = tasks
        .par_iter()
        .map(|(dicom_path, png_path, folder_relative)| {
            let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
            let filename = dicom_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if png_path.exists() {
                progress_callback(ProgressPayload {
                    current,
                    total,
                    filename: filename.clone(),
                    status: "skipped".to_string(),
                });

                // Try to read metadata from DICOM file for the report
                let metadata = crate::logic::convert::extract_metadata(dicom_path).ok();

                return (
                    dicom_path,
                    Ok(FileOutcome::Skipped {
                        metadata: metadata.unwrap_or_default(), // Fallback if read fails
                        reason: "already exists".to_string(),
                    }),
                    folder_relative,
                );
            }

            progress_callback(ProgressPayload {
                current,
                total,
                filename,
                status: "converting".to_string(),
            });

            let outcome = convert_single_file(dicom_path, png_path);
            (dicom_path, outcome, folder_relative)
        })
        .collect();

    for (dicom_path, outcome, folder_relative) in results {
        // Helper to register metadata
        let mut register_metadata = |mut metadata: FileMetadata| {
            metadata.folder_relative = folder_relative.clone();
            folder_metadata
                .entry(folder_relative.clone())
                .or_default()
                .push(metadata.clone());
            all_metadata.push(metadata);
        };

        match outcome {
            Ok(FileOutcome::Converted(metadata)) => {
                register_metadata(metadata);
                successful += 1;
                let entry = LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: true,
                    status: "Success".to_string(),
                    message: "Converted successfully".to_string(),
                    conversion_type: "PNG".to_string(),
                };
                log_callback(entry.clone());
                logs.push(entry);
            }
            Ok(FileOutcome::Skipped { metadata, reason }) => {
                register_metadata(metadata);
                skipped_count += 1;
                skipped_files.push(
                    dicom_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| dicom_path.to_string_lossy().to_string()),
                );
                println!(
                    "{} Skipping {} ({reason})",
                    "∙".cyan(),
                    dicom_path.display()
                );
                let entry = LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: true,
                    status: "Skipped".to_string(),
                    message: reason.clone(),
                    conversion_type: "PNG".to_string(),
                };
                log_callback(entry.clone());
                logs.push(entry);
            }
            Ok(FileOutcome::Failed { metadata, error }) => {
                register_metadata(metadata);
                eprintln!(
                    "{} Failed to convert {}:\n{:#}",
                    "✖".red(),
                    dicom_path.display(),
                    error
                );
                failed_files.push(
                    dicom_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| dicom_path.to_string_lossy().to_string()),
                );
                logs.push(LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: false,
                    status: "Failed".to_string(),
                    message: error.to_string(),
                    conversion_type: "PNG".to_string(),
                });
                log_callback(LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: false,
                    status: "Failed".to_string(),
                    message: error.to_string(),
                    conversion_type: "PNG".to_string(),
                });
            }
            Err(err) => {
                // This case should be rare now as convert_single_file catches most errors
                eprintln!(
                    "{} Critical error processing {}:\n{:#}",
                    "✖".red(),
                    dicom_path.display(),
                    err
                );
                failed_files.push(
                    dicom_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| dicom_path.to_string_lossy().to_string()),
                );
                logs.push(LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: false,
                    status: "Failed".to_string(),
                    message: err.to_string(),
                    conversion_type: "PNG".to_string(),
                });
                log_callback(LogEntry {
                    file_name: dicom_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    file_path: dicom_path.to_string_lossy().to_string(),
                    success: false,
                    status: "Failed".to_string(),
                    message: err.to_string(),
                    conversion_type: "PNG".to_string(),
                });
            }
        }
    }

    if save_excel {
        write_metadata_workbooks(&all_metadata, &folder_metadata, &png_output_path)
            .context("Unable to write Excel metadata files")?;
    }

    // Write logs
    write_logs(&root_output_path, &logs).context("Unable to write logs")?;

    Ok(ConversionReport {
        total,
        successful,
        failed: total.saturating_sub(successful + skipped_count),
        skipped_non_image: skipped_count,
        failed_files,
        skipped_files,
        output_folder: root_output_path,
    })
}

fn build_png_path(input_folder: &Path, output_folder: &Path, dicom_path: &Path) -> PathBuf {
    let relative: PathBuf = dicom_path
        .strip_prefix(input_folder)
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dicom_path
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("unknown"))
        });

    let mut png_path = output_folder.join(relative);
    png_path.set_extension("png");
    png_path
}
