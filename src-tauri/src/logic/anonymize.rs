use crate::models::metadata::FileMetadata;
use crate::utils::discovery::collect_dicom_files;
use crate::utils::logging::{write_logs, LogEntry};
use crate::utils::metadata_export::write_metadata_report;
use anyhow::{bail, Context, Result};
use dicom::core::{DataElement, PrimitiveValue, Tag, VR};
use dicom::object::open_file;
use dicom::object::{FileDicomObject, InMemDicomObject};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

#[derive(Clone, serde::Serialize)]
pub struct AnonymizationReport {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
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

pub fn anonymize_dicom<F, G>(
    input_folder: &Path,
    output_folder: &Path,
    tags_to_anonymize: Vec<(u16, u16)>, // Group, Element
    replacement_value: String,
    progress_callback: F,
    log_callback: G,
) -> Result<AnonymizationReport>
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

    let root_output_path = output_folder.join(format!("{}_output", input_name));
    let dicom_output_path = root_output_path.join("dicom_file");

    fs::create_dir_all(&dicom_output_path).with_context(|| {
        format!(
            "Unable to create output folder {}",
            dicom_output_path.display()
        )
    })?;

    let dicom_files = collect_dicom_files(input_folder);
    let total = dicom_files.len();
    let processed_count = AtomicUsize::new(0);

    let results: Vec<_> = dicom_files
        .par_iter()
        .map(|dicom_path| {
            let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;
            let filename = dicom_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Calculate output path preserving relative structure
            let relative_path = dicom_path
                .strip_prefix(input_folder)
                .unwrap_or_else(|_| Path::new(&filename));

            let output_path = dicom_output_path.join(relative_path);

            if let Some(parent) = output_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            // Check if output file already exists
            if output_path.exists() {
                progress_callback(ProgressPayload {
                    current,
                    total,
                    filename: filename.clone(),
                    status: "skipped".to_string(),
                });

                let folder_relative = relative_path
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));

                // We need to read metadata even if skipped to include in the report
                // Try to read from the existing output file first, or the input file if that fails
                let metadata = match open_file(&output_path) {
                    Ok(obj) => extract_metadata(&obj, dicom_path).ok(),
                    Err(_) => {
                        // Fallback to input file
                        open_file(dicom_path)
                            .ok()
                            .and_then(|obj| extract_metadata(&obj, dicom_path).ok())
                    }
                };

                return (
                    dicom_path,
                    Ok(AnonymizeOutcome::Skipped(metadata)),
                    folder_relative,
                );
            }

            progress_callback(ProgressPayload {
                current,
                total,
                filename: filename.clone(),
                status: "anonymizing".to_string(),
            });

            let outcome = anonymize_single_file(
                dicom_path,
                &output_path,
                &tags_to_anonymize,
                &replacement_value,
            );

            let final_outcome = match outcome {
                Ok(meta) => Ok(AnonymizeOutcome::Success(meta)),
                Err(e) => Err(e),
            };

            let folder_relative = relative_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));

            (dicom_path, final_outcome, folder_relative)
        })
        .collect();

    let mut successful = 0usize;
    let mut skipped = 0usize;
    let mut failed_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut all_metadata = Vec::new();
    let mut folder_metadata: BTreeMap<PathBuf, Vec<FileMetadata>> = BTreeMap::new();
    let mut logs: Vec<LogEntry> = Vec::new();

    for (dicom_path, outcome, folder_relative) in results {
        let mut register_metadata = |mut metadata: FileMetadata| {
            metadata.folder_relative = folder_relative.clone();
            folder_metadata
                .entry(folder_relative.clone())
                .or_default()
                .push(metadata.clone());
            all_metadata.push(metadata);
        };

        match outcome {
            Ok(AnonymizeOutcome::Success(metadata)) => {
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
                    message: "Anonymized successfully".to_string(),
                    conversion_type: "ANONYMIZE".to_string(),
                };
                log_callback(entry.clone());
                logs.push(entry);
            }
            Ok(AnonymizeOutcome::Skipped(metadata_opt)) => {
                if let Some(metadata) = metadata_opt {
                    register_metadata(metadata);
                }
                skipped += 1;
                skipped_files.push(
                    dicom_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(String::from)
                        .unwrap_or_else(|| dicom_path.to_string_lossy().to_string()),
                );
                println!(
                    "{} Skipping {} (already exists)",
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
                    message: "Output file already exists".to_string(),
                    conversion_type: "ANONYMIZE".to_string(),
                };
                log_callback(entry.clone());
                logs.push(entry);
            }
            Err(err) => {
                eprintln!(
                    "{} Failed to anonymize {}:\n{:#}",
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
                    conversion_type: "ANONYMIZE".to_string(),
                };
                log_callback(entry.clone());
                logs.push(entry);
            }
        }
    }

    // Write metadata report
    write_metadata_report(&all_metadata, &dicom_output_path)
        .context("Unable to write metadata report")?;

    // Write logs
    write_logs(&root_output_path, &logs).context("Unable to write logs")?;

    Ok(AnonymizationReport {
        total,
        successful,
        failed: total.saturating_sub(successful + skipped),
        skipped,
        failed_files,
        skipped_files,
        output_folder: root_output_path,
    })
}

enum AnonymizeOutcome {
    Success(FileMetadata),
    Skipped(Option<FileMetadata>),
}

fn anonymize_single_file(
    input_path: &Path,
    output_path: &Path,
    tags_to_anonymize: &[(u16, u16)],
    replacement_value: &str,
) -> Result<FileMetadata> {
    let mut obj = open_file(input_path).context("Failed to open DICOM file")?;

    // Anonymize tags
    for &(group, element) in tags_to_anonymize {
        let tag = Tag(group, element);
        if let Ok(elem) = obj.element(tag) {
            let vr = elem.vr();
            // Construct new element with same VR but replaced value
            // Note: This assumes the replacement value string is valid for the VR.
            // For complex VRs this might fail or be invalid, but for standard anonymization it's usually fine.
            let new_elem =
                DataElement::new(tag, vr, PrimitiveValue::from(replacement_value.to_string()));
            obj.put_element(new_elem);
        }
    }

    // Regenerate SOP Instance UID
    let sop_class_uid_tag = Tag(0x0008, 0x0016);
    let sop_instance_uid_tag = Tag(0x0008, 0x0018);

    // Set SOP Class UID to CT Image Storage (as per python script)
    // 1.2.840.10008.5.1.4.1.1.2
    let class_uid_elem = DataElement::new(
        sop_class_uid_tag,
        VR::UI,
        PrimitiveValue::from("1.2.840.10008.5.1.4.1.1.2"),
    );
    obj.put_element(class_uid_elem);

    // Generate a new UUID for SOP Instance UID
    let new_uid = format!("2.25.{}", Uuid::new_v4().as_u128());
    let instance_uid_elem =
        DataElement::new(sop_instance_uid_tag, VR::UI, PrimitiveValue::from(new_uid));
    obj.put_element(instance_uid_elem);

    // Save
    obj.write_to_file(output_path)
        .context("Failed to save anonymized file")?;

    extract_metadata(&obj, input_path)
}

fn extract_metadata(obj: &FileDicomObject<InMemDicomObject>, path: &Path) -> Result<FileMetadata> {
    let get_str = |tag: Tag| -> Option<String> {
        obj.element(tag)
            .ok()
            .and_then(|e| e.to_str().ok())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    };

    let get_u32 = |tag: Tag| -> Option<u32> { obj.element(tag).ok().and_then(|e| e.to_int().ok()) };

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(FileMetadata {
        folder_relative: PathBuf::new(), // Filled later
        file_name: filename,
        study_date: get_str(Tag(0x0008, 0x0020)),
        modality: get_str(Tag(0x0008, 0x0060)),
        manufacturer: get_str(Tag(0x0008, 0x0070)),
        study_description: get_str(Tag(0x0008, 0x1030)),
        series_description: get_str(Tag(0x0008, 0x103E)),
        institution_name: get_str(Tag(0x0008, 0x0080)),
        pixel_data: Some(crate::models::metadata::extract_pixel_data_status(&obj)),
        im_width: get_u32(Tag(0x0028, 0x0011)),  // Columns
        im_height: get_u32(Tag(0x0028, 0x0010)), // Rows
        pixel_spacing: get_str(Tag(0x0028, 0x0030)),
    })
}
