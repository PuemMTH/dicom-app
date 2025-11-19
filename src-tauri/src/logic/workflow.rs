use crate::logic::convert::{convert_single_file, FileOutcome};
use crate::utils::discovery::collect_dicom_files;
use crate::utils::excel::write_metadata_workbooks;
use crate::models::metadata::FileMetadata;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConversionReport {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped_non_image: usize,
    pub failed_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub output_folder: PathBuf,
}

pub fn convert_dicom_to_png(
    input_folder: &Path,
    output_folder: &Path,
    save_excel: bool,
) -> Result<ConversionReport> {
    if !input_folder.exists() {
        bail!("Input folder '{}' does not exist", input_folder.display());
    }

    fs::create_dir_all(output_folder)
        .with_context(|| format!("Unable to create output folder {}", output_folder.display()))?;

    let dicom_files = collect_dicom_files(input_folder);
    let mut tasks = Vec::new();

    for path in dicom_files {
        let png_path = build_png_path(input_folder, output_folder, &path);
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

    for (dicom_path, png_path, folder_relative) in tasks {
        let outcome = convert_single_file(&dicom_path, &png_path);

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
        write_metadata_workbooks(&all_metadata, &folder_metadata, output_folder)
            .context("Unable to write Excel metadata files")?;
    }

    Ok(ConversionReport {
        total,
        successful,
        failed: total.saturating_sub(successful + skipped_count),
        skipped_non_image: skipped_count,
        failed_files,
        skipped_files,
        output_folder: output_folder.to_path_buf(),
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
