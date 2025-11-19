use crate::logic::workflow::convert_dicom_to_png;

#[tauri::command]
pub async fn convert_dicom(input: String, output: String, skip_excel: bool) {
    let report = convert_dicom_to_png(
        std::path::Path::new(&input),
        std::path::Path::new(&output),
        !skip_excel,
    );
}
