use crate::models::metadata::{dicom_date, dicom_text, pixel_spacing, FileMetadata};
use anyhow::{Context, Result};
use dicom::core::Tag;
use dicom_object::{open_file, DefaultDicomObject};
use dicom_pixeldata::{image::DynamicImage, PixelDecoder as _};
use std::fs;
use std::path::{Path, PathBuf};

pub enum FileOutcome {
    Converted(FileMetadata),
    Skipped {
        metadata: FileMetadata,
        reason: String,
    },
    Failed {
        metadata: FileMetadata,
        error: anyhow::Error,
    },
}

pub fn convert_single_file(dicom_path: &Path, png_path: &Path) -> Result<FileOutcome> {
    let obj: DefaultDicomObject = open_file(dicom_path)
        .with_context(|| format!("Failed to open DICOM file {}", dicom_path.display()))?;

    let mut metadata = extract_metadata(dicom_path)?;

    if !has_pixel_data(&obj) {
        let modality = metadata
            .modality
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        let sop_class = dicom_text(&obj, Tag(0x0008, 0x0016)).unwrap_or_else(|| "Unknown".into());
        return Ok(FileOutcome::Failed {
            metadata,
            error: anyhow::anyhow!("no pixel data (Modality={modality}, SOPClass={sop_class})"),
        });
    }

    let pixel_data = match obj.decode_pixel_data() {
        Ok(data) => data,
        Err(e) => {
            return Ok(FileOutcome::Failed {
                metadata,
                error: e.into(),
            })
        }
    };

    let image = match pixel_data.to_dynamic_image(0) {
        Ok(img) => img,
        Err(e) => {
            return Ok(FileOutcome::Failed {
                metadata,
                error: e.into(),
            })
        }
    };

    if let Err(e) = save_image(&image, png_path) {
        return Ok(FileOutcome::Failed { metadata, error: e });
    }

    metadata.im_width = Some(image.width());
    metadata.im_height = Some(image.height());

    Ok(FileOutcome::Converted(metadata))
}

fn has_pixel_data(obj: &DefaultDicomObject) -> bool {
    const PIXEL_TAGS: [Tag; 3] = [
        Tag(0x7FE0, 0x0010),
        Tag(0x7FE0, 0x0008),
        Tag(0x7FE0, 0x0009),
    ];
    PIXEL_TAGS.iter().any(|tag| obj.element(*tag).is_ok())
}

fn save_image(image: &DynamicImage, png_path: &Path) -> Result<()> {
    if let Some(parent) = png_path.parent() {
        fs::create_dir_all(parent)?;
    }
    image
        .save(png_path)
        .with_context(|| format!("Unable to save PNG to {}", png_path.display()))?;
    Ok(())
}

pub fn extract_metadata(dicom_path: &Path) -> Result<FileMetadata> {
    let obj: DefaultDicomObject = open_file(dicom_path)
        .with_context(|| format!("Failed to open DICOM file {}", dicom_path.display()))?;

    Ok(FileMetadata {
        folder_relative: PathBuf::new(),
        file_name: dicom_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        study_date: dicom_date(&obj, Tag(0x0008, 0x0020)),
        modality: dicom_text(&obj, Tag(0x0008, 0x0060)),
        manufacturer: dicom_text(&obj, Tag(0x0008, 0x0070)),
        study_description: dicom_text(&obj, Tag(0x0008, 0x1030)),
        series_description: dicom_text(&obj, Tag(0x0008, 0x103E)),
        institution_name: dicom_text(&obj, Tag(0x0008, 0x0080)),
        pixel_data: Some(crate::models::metadata::extract_pixel_data_status(&obj)),
        im_width: obj
            .element(Tag(0x0028, 0x0011))
            .ok()
            .and_then(|e| e.to_int().ok()),
        im_height: obj
            .element(Tag(0x0028, 0x0010))
            .ok()
            .and_then(|e| e.to_int().ok()),
        pixel_spacing: pixel_spacing(&obj),
    })
}
