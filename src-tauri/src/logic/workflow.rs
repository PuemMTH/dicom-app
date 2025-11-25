use crate::logic::convert::{convert_single_file, FileOutcome};
use crate::utils::discovery::collect_dicom_files;
use crate::utils::logging::LogEntry;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use rayon::prelude::*;
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
    G: Fn(LogEntry) + Sync + Send + 'static,
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
    let processed_count = AtomicUsize::new(0);

    // Channel for sending results to the writer thread
    let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, Result<FileOutcome>, PathBuf)>();

    // Spawn writer thread
    let writer_handle = std::thread::spawn({
        let png_output_path = png_output_path.clone();
        let root_output_path = root_output_path.clone();
        move || -> Result<ConversionReport> {
            let mut successful = 0usize;
            let mut failed_files = Vec::new();
            let mut skipped_files = Vec::new();
            let mut logs: Vec<LogEntry> = Vec::new();
            let mut skipped_count = 0usize;

            // Initialize metadata writer if needed
            let mut metadata_writer = if save_excel {
                Some(crate::utils::metadata_export::MetadataWriter::new(
                    &png_output_path,
                )?)
            } else {
                None
            };

            // Initialize log writer
            let mut log_writer = crate::utils::logging::LogWriter::new(&root_output_path)?;

            for (dicom_path, outcome, folder_relative) in rx {
                match outcome {
                    Ok(FileOutcome::Converted(mut metadata)) => {
                        metadata.folder_relative = folder_relative;
                        if let Some(writer) = &mut metadata_writer {
                            writer.write_record(&metadata)?;
                        }
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
                        log_writer.write_entry(&entry)?;
                        logs.push(entry);
                    }
                    Ok(FileOutcome::Skipped {
                        mut metadata,
                        reason,
                    }) => {
                        metadata.folder_relative = folder_relative;
                        if let Some(writer) = &mut metadata_writer {
                            writer.write_record(&metadata)?;
                        }
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
                        log_writer.write_entry(&entry)?;
                        logs.push(entry);
                    }
                    Ok(FileOutcome::Failed {
                        mut metadata,
                        error,
                    }) => {
                        metadata.folder_relative = folder_relative;
                        // Optionally write failed metadata too? Original code did register it.
                        if let Some(writer) = &mut metadata_writer {
                            writer.write_record(&metadata)?;
                        }
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
                        let entry = LogEntry {
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
                        };
                        log_callback(entry.clone());
                        log_writer.write_entry(&entry)?;
                        logs.push(entry);
                    }
                    Err(err) => {
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
                        let entry = LogEntry {
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
                        };
                        log_callback(entry.clone());
                        log_writer.write_entry(&entry)?;
                        logs.push(entry);
                    }
                }
            }

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
    });

    tasks
        .par_iter()
        .for_each_with(tx, |tx, (dicom_path, png_path, folder_relative)| {
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

                let _ = tx.send((
                    dicom_path.clone(),
                    Ok(FileOutcome::Skipped {
                        metadata: metadata.unwrap_or_default(), // Fallback if read fails
                        reason: "already exists".to_string(),
                    }),
                    folder_relative.clone(),
                ));
                return;
            }

            progress_callback(ProgressPayload {
                current,
                total,
                filename,
                status: "converting".to_string(),
            });

            let outcome = convert_single_file(dicom_path, png_path);
            let _ = tx.send((dicom_path.clone(), outcome, folder_relative.clone()));
        });

    // Wait for writer thread to finish
    writer_handle.join().unwrap()
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
