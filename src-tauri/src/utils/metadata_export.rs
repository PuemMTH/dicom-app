use crate::models::metadata::FileMetadata;
use anyhow::Result;
use std::path::Path;

pub fn write_metadata_report(all_metadata: &[FileMetadata], output_folder: &Path) -> Result<()> {
    if all_metadata.is_empty() {
        return Ok(());
    }

    let metadata_root = output_folder
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| output_folder.to_path_buf());

    let all_path = metadata_root.join("metadata_all.csv");
    write_metadata_csv(&all_path, all_metadata)?;

    Ok(())
}

fn write_metadata_csv(path: &Path, rows: &[FileMetadata]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write headers
    wtr.write_record(&[
        "F_name",
        "Study_date",
        "Modality",
        "Manufacturer",
        "Study_description",
        "Series_description",
        "Institution_name",
        "Pixel_data",
        "Im_width",
        "Im_height",
        "Pixel_spacing",
    ])?;

    for metadata in rows {
        wtr.write_record(&[
            &metadata.file_name,
            metadata.study_date.as_deref().unwrap_or(""),
            metadata.modality.as_deref().unwrap_or(""),
            metadata.manufacturer.as_deref().unwrap_or(""),
            metadata.study_description.as_deref().unwrap_or(""),
            metadata.series_description.as_deref().unwrap_or(""),
            metadata.institution_name.as_deref().unwrap_or(""),
            metadata.pixel_data.as_deref().unwrap_or(""),
            &metadata.im_width.map(|v| v.to_string()).unwrap_or_default(),
            &metadata
                .im_height
                .map(|v| v.to_string())
                .unwrap_or_default(),
            metadata.pixel_spacing.as_deref().unwrap_or(""),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
