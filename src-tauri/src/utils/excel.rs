use crate::models::metadata::FileMetadata;
use anyhow::Result;
use rust_xlsxwriter::{Format, Workbook, Worksheet, XlsxError};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn write_metadata_workbooks(
    all_metadata: &[FileMetadata],
    folder_metadata: &BTreeMap<PathBuf, Vec<FileMetadata>>,
    output_folder: &Path,
) -> Result<()> {
    if all_metadata.is_empty() {
        return Ok(());
    }

    let metadata_root = output_folder
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| output_folder.to_path_buf());

    let all_path = metadata_root.join("metadata_all.xlsx");
    write_metadata_sheet(&all_path, all_metadata)?;

    for (relative_folder, rows) in folder_metadata {
        if rows.is_empty() {
            continue;
        }

        let subfolder = output_folder.join(relative_folder);
        fs::create_dir_all(&subfolder)?;
        let sheet_path = subfolder.join("metadata.xlsx");
        write_metadata_sheet(&sheet_path, rows)?;
    }

    Ok(())
}

fn write_metadata_sheet(path: &Path, rows: &[FileMetadata]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let headers = [
        "F_name",
        "Study_date",
        "Modality",
        "Manufacturer",
        "Study_description",
        "Series_description",
        "Institution_name",
        "Im_width",
        "Im_height",
        "Pixel_spacing",
    ];

    let mut workbook = Workbook::new();
    let mut worksheet = workbook.add_worksheet();
    let header_format = Format::new().set_bold();

    for (col, &header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, header, &header_format)?;
    }

    for (idx, metadata) in rows.iter().enumerate() {
        let row = (idx + 1) as u32;
        worksheet.write_string(row, 0, &metadata.file_name)?;
        write_optional_string(&mut worksheet, row, 1, metadata.study_date.as_deref())?;
        write_optional_string(&mut worksheet, row, 2, metadata.modality.as_deref())?;
        write_optional_string(&mut worksheet, row, 3, metadata.manufacturer.as_deref())?;
        write_optional_string(&mut worksheet, row, 4, metadata.study_description.as_deref())?;
        write_optional_string(&mut worksheet, row, 5, metadata.series_description.as_deref())?;
        write_optional_string(&mut worksheet, row, 6, metadata.institution_name.as_deref())?;
        write_optional_number(&mut worksheet, row, 7, metadata.im_width)?;
        write_optional_number(&mut worksheet, row, 8, metadata.im_height)?;
        write_optional_string(&mut worksheet, row, 9, metadata.pixel_spacing.as_deref())?;
    }

    workbook.save(path)?;
    Ok(())
}

fn write_optional_string(
    worksheet: &mut Worksheet,
    row: u32,
    col: usize,
    value: Option<&str>,
) -> Result<(), XlsxError> {
    if let Some(text) = value {
        worksheet.write_string(row, col as u16, text)?;
    }
    Ok(())
}

fn write_optional_number(
    worksheet: &mut Worksheet,
    row: u32,
    col: usize,
    value: Option<u32>,
) -> Result<(), XlsxError> {
    if let Some(number) = value {
        worksheet.write_number(row, col as u16, number as f64)?;
    }
    Ok(())
}
