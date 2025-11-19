use crate::logic::convert::{convert_single_file, FileOutcome};
use crate::models::metadata::FileMetadata;
use crate::utils::discovery::collect_dicom_files;
use crate::utils::excel::write_metadata_workbooks;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

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

pub fn convert_dicom_to_png<F>(
    input_folder: &Path,
    output_folder: &Path,
    save_excel: bool,
    progress_callback: F,
) -> Result<ConversionReport>
where
    F: Fn(ProgressPayload) + Sync + Send,
{
    if !input_folder.exists() {
        bail!("Input folder '{}' does not exist", input_folder.display());
    }

    // Determine the input folder name for the output directory
    let input_name = input_folder
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dicom");

    let root_output_path = output_folder.join(format!("{}_output", input_name));
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
        if png_path.exists() {
            continue;
        }

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
            }
        }
    }

    if save_excel {
        write_metadata_workbooks(&all_metadata, &folder_metadata, &png_output_path)
            .context("Unable to write Excel metadata files")?;
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
