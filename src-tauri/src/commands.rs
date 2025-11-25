use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn convert_dicom(
    app: AppHandle,
    input: String,
    output: String,
    skip_excel: bool,
    flatten_output: bool,
) -> Result<crate::logic::workflow::ConversionReport, String> {
    use crate::logic::workflow::convert_dicom_to_png;
    match convert_dicom_to_png(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        !skip_excel,
        flatten_output,
        |progress| {
            let _ = app.emit("conversion_progress", progress);
        },
        {
            let app = app.clone();
            move |log| {
                let _ = app.emit("log_event", log);
            }
        },
    ) {
        Ok(report) => {
            // Open the output folder after conversion completes
            let _ = app.opener().open_url(
                report.output_folder.to_string_lossy().as_ref(),
                None::<&str>,
            );
            Ok(report)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn anonymize_dicom(
    app: AppHandle,
    input: String,
    output: String,
    tags: Vec<(u16, u16)>,
    replacement: String,
) -> Result<crate::logic::anonymize::AnonymizationReport, String> {
    use crate::logic::anonymize::anonymize_dicom;
    match anonymize_dicom(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        tags,
        replacement,
        |progress| {
            let _ = app.emit("anonymization_progress", progress);
        },
        {
            let app = app.clone();
            move |log| {
                let _ = app.emit("log_event", log);
            }
        },
    ) {
        Ok(report) => {
            let _ = app.opener().open_url(
                report.output_folder.to_string_lossy().as_ref(),
                None::<&str>,
            );
            Ok(report)
        }
        Err(e) => Err(e.to_string()),
    }
}

use serde::Deserialize;

#[derive(Deserialize)]
pub struct DicomToPngInput {
    pub input: String,
    pub output: String,
    pub skip_excel: bool,
    pub flatten_output: bool,
}

#[derive(Deserialize)]
pub struct AnonymizeDicomInput {
    pub input: String,
    pub output: String,
    pub tags: Vec<(u16, u16)>,
    pub replacement: String,
}

#[derive(Deserialize, Default)]
pub struct DicomProcessInput {
    pub convert: Option<DicomToPngInput>,
    pub anonymize: Option<AnonymizeDicomInput>,
}

#[tauri::command]
pub async fn process_dicom(
    app: AppHandle,
    input: DicomProcessInput,
) -> Result<ProcessReport, String> {
    use crate::logic::anonymize::anonymize_dicom as do_anonymize;
    use crate::logic::workflow::convert_dicom_to_png;

    let mut report = ProcessReport::default();

    // Process conversion if requested
    if let Some(convert_input) = input.convert {
        match convert_dicom_to_png(
            std::path::Path::new(&convert_input.input),
            std::path::Path::new(&convert_input.output),
            !convert_input.skip_excel,
            convert_input.flatten_output,
            |progress| {
                let _ = app.emit("conversion_progress", progress);
            },
            {
                let app = app.clone();
                move |log| {
                    let _ = app.emit("log_event", log);
                }
            },
        ) {
            Ok(conversion_report) => {
                // let _ = app.opener().open_url(
                //     conversion_report.output_folder.to_string_lossy().as_ref(),
                //     None::<&str>,
                // );
                report.conversion = Some(conversion_report);
            }
            Err(e) => return Err(format!("Conversion failed: {}", e)),
        }
    }

    // Process anonymization if requested
    if let Some(anonymize_input) = input.anonymize {
        match do_anonymize(
            std::path::Path::new(&anonymize_input.input),
            std::path::Path::new(&anonymize_input.output),
            anonymize_input.tags,
            anonymize_input.replacement,
            |progress| {
                let _ = app.emit("anonymization_progress", progress);
            },
            {
                let app = app.clone();
                move |log| {
                    let _ = app.emit("log_event", log);
                }
            },
        ) {
            Ok(anonymization_report) => {
                // let _ = app.opener().open_url(
                //     anonymization_report
                //         .output_folder
                //         .to_string_lossy()
                //         .as_ref(),
                //     None::<&str>,
                // );
                report.anonymization = Some(anonymization_report);
            }
            Err(e) => return Err(format!("Anonymization failed: {}", e)),
        }
    }

    Ok(report)
}

#[derive(serde::Serialize, Default)]
pub struct ProcessReport {
    pub conversion: Option<crate::logic::workflow::ConversionReport>,
    pub anonymization: Option<crate::logic::anonymize::AnonymizationReport>,
}

#[tauri::command]
pub async fn get_dicom_tags(path: String) -> Result<Vec<crate::logic::tags::DicomTag>, String> {
    crate::logic::tags::read_all_tags(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_dicom_files(folder: String) -> Result<Vec<String>, String> {
    let path = std::path::Path::new(&folder);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }
    let files = crate::utils::discovery::collect_dicom_files(path);
    Ok(files
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub async fn get_pinned_tags_stats(
    app: AppHandle,
    cache: tauri::State<'_, crate::logic::stats::StatsCache>,
    folder: String,
    tags: Vec<(u16, u16)>,
) -> Result<Vec<crate::logic::stats::TagStat>, String> {
    let path = std::path::Path::new(&folder);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }

    // Check cache
    {
        let cache_lock = cache.0.lock().map_err(|e| e.to_string())?;
        if let Some(cached_result) = cache_lock.get(&(folder.clone(), tags.clone())) {
            return Ok(cached_result.clone());
        }
    }

    let result = crate::logic::stats::calculate_stats(path, tags.clone(), |progress| {
        let _ = app.emit("stats_progress", progress);
    })
    .map_err(|e| e.to_string())?;

    // Update cache
    {
        let mut cache_lock = cache.0.lock().map_err(|e| e.to_string())?;
        cache_lock.insert((folder, tags), result.clone());
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_tag_details(
    app: AppHandle,
    folder: String,
    group: u16,
    element: u16,
) -> Result<crate::logic::stats::TagDetails, String> {
    let path = std::path::Path::new(&folder);
    if !path.exists() || !path.is_dir() {
        return Err("Invalid folder path".to_string());
    }
    crate::logic::stats::get_tag_details(path, group, element, |progress| {
        let _ = app.emit("tag_details_progress", progress);
    })
    .map_err(|e| e.to_string())
}
