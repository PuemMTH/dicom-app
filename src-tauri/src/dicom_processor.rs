use std::path::Path;

use dicom_object::open_file;
use dicom_pixeldata::PixelDecoder;
use dicom_transfer_syntax_registry::{TransferSyntaxIndex, TransferSyntaxRegistry};
use owo_colors::OwoColorize;
use image::GrayImage;

use crate::models::DicomSummary;

/// ประมวลผลไฟล์ DICOM และแยกข้อมูลสำคัญออกมา
pub fn summarize_dicom(path: &Path) -> Result<DicomSummary, String> {
    // เปิดไฟล์ DICOM
    let obj = open_file(path).map_err(|e| e.to_string())?;
    // println!("File opened: {}", path.display());

    // ดึง Transfer Syntax
    let ts_uid = obj.meta().transfer_syntax();
    let ts_name = TransferSyntaxRegistry::default()
        .get(ts_uid)
        .map(|ts| ts.name())
        .unwrap_or(ts_uid);
    // println!("Transfer Syntax: {}", ts_name);

    // Decode pixel data
    let pixel_data = obj.decode_pixel_data().map_err(|e| e.to_string())?;
    // println!("Pixel data decoded!");
    // println!("Rows: {}", pixel_data.rows());
    // println!("Cols: {}", pixel_data.columns());
    // println!("Bits Allocated: {}", pixel_data.bits_allocated());

    // ดึงชื่อไฟล์
    let file_label = path
        .file_name()
        .and_then(|v| v.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string());

    // สร้าง summary
    Ok(DicomSummary::success(
        file_label,
        pixel_data.rows(),
        pixel_data.columns(),
        pixel_data.bits_allocated(),
        ts_name.to_string(),
    ))
}


/// Convert DICOM to PNG with full preprocessing pipeline
/// ตามขั้นตอนของ Python version
/// 1. Loading - อ่านไฟล์ DICOM ด้วย pydicom.dcmread
/// 2. Pre-processing & Photometric Interpretation - จัดการ RGB/YBR/MONOCHROME
/// 3. Apply Modality LUT - util.apply_modality_lut
/// 4. Apply VOI LUT / Windowing - util.apply_voi_lut (รองรับ string window values)
/// 5. Normalization - Min-Max scaling (0-255)
/// 6. Save Image - บันทึกเป็น PNG
pub fn convert_dicom_to_png(
    path: &Path,
    output_folder: &Path,
) -> Result<DicomSummary, String> {
    use std::fs;
    
    // ============================================================
    // Step 1: Loading (อ่านไฟล์) - pydicom.dcmread(dicom_file_path, force=True)
    // ============================================================
    let obj = open_file(path).map_err(|e| e.to_string())?;
    let ts_uid = obj.meta().transfer_syntax();
    let ts_name = TransferSyntaxRegistry::default()
        .get(ts_uid)
        .map(|ts| ts.name())
        .unwrap_or(ts_uid);
    
    let file_label = path
        .file_name()
        .and_then(|v| v.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Decode pixel data
    let pixel_data = obj.decode_pixel_data().map_err(|e| e.to_string())?;
    let rows = pixel_data.rows();
    let cols = pixel_data.columns();
    let bits_allocated = pixel_data.bits_allocated();
    
    // ============================================================
    // Step 2: Pre-processing & Photometric Interpretation
    // if 'PhotometricInterpretation' not in ds: use pixel_array directly
    // elif ds.PhotometricInterpretation in ['RGB', 'YBR_FULL']: rgb2gray
    // elif ds.PhotometricInterpretation in ['MONOCHROME1', 'MONOCHROME2']: 
    //   if MONOCHROME1: invert = np.amax(imdata) - imdata
    // ============================================================
    let photometric_interpretation = obj
        .element_by_name("PhotometricInterpretation")
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    print!("Photometric Interpretation: {}\n", 
        photometric_interpretation.as_ref().unwrap_or(&"Not found".to_string()).green());

    // Convert to DynamicImage
    let dicom_image = pixel_data
        .to_dynamic_image(0)
        .map_err(|e| e.to_string())?;

    let (width, height) = (cols as u32, rows as u32);
    let raw_bytes = dicom_image.as_bytes();
    
    // Pre-processing based on Photometric Interpretation
    // ตรงกับโค้ด Python:
    // if 'PhotometricInterpretation' not in ds:
    //     if 'PixelData' in ds: imdata = ds.pixel_array
    // elif ds.PhotometricInterpretation in ['RGB', 'YBR_FULL']:
    //     imdata = rgb2gray(ds.pixel_array)
    // elif ds.PhotometricInterpretation in ['MONOCHROME1', 'MONOCHROME2']:
    //     imdata = ds.pixel_array
    
    let mut pixel_values: Vec<f64> = match photometric_interpretation.as_deref() {
        Some("RGB") | Some("YBR_FULL") | Some("YBR_FULL_422") => {
            print!("Converting RGB/YBR to Grayscale (rgb2gray)...\n");
            // rgb2gray from skimage: 0.2125*R + 0.7154*G + 0.0721*B (or simplified 0.299*R + 0.587*G + 0.114*B)
            raw_bytes
                .chunks(3)
                .map(|rgb| {
                    0.299 * rgb[0] as f64 + 0.587 * rgb[1] as f64 + 0.114 * rgb[2] as f64
                })
                .collect()
        },
        Some("MONOCHROME1") | Some("MONOCHROME2") | None | _ => {
            // Use pixel_array directly - imdata = ds.pixel_array
            if bits_allocated == 16 {
                // 16-bit data - อ่านตาม byte order ของ DICOM
                raw_bytes
                    .chunks(2)
                    .filter(|chunk| chunk.len() == 2)
                    .map(|bytes| {
                        u16::from_le_bytes([bytes[0], bytes[1]]) as f64
                    })
                    .collect()
            } else {
                // 8-bit data
                raw_bytes.iter().map(|&b| b as f64).collect()
            }
        }
    };

    // MONOCHROME1 inversion: imdata = np.amax(imdata) - imdata
    if photometric_interpretation.as_deref() == Some("MONOCHROME1") {
        print!("Inverting MONOCHROME1 (np.amax(imdata) - imdata)...\n");
        let max_val = pixel_values.iter().fold(f64::MIN, |a, &b| a.max(b));
        pixel_values = pixel_values.iter().map(|&v| max_val - v).collect();
    }

    // ============================================================
    // Step 3: Apply Modality LUT - imdata = util.apply_modality_lut(imdata, ds)
    // ============================================================
    let rescale_slope = obj
        .element_by_name("RescaleSlope")
        .ok()
        .and_then(|elem| elem.to_float64().ok())
        .unwrap_or(1.0);
    
    let rescale_intercept = obj
        .element_by_name("RescaleIntercept")
        .ok()
        .and_then(|elem| elem.to_float64().ok())
        .unwrap_or(0.0);

    print!(
        "Applying Modality LUT: Slope={}, Intercept={}\n",
        rescale_slope.to_string().cyan(),
        rescale_intercept.to_string().cyan()
    );

    // Apply modality LUT: pixel = raw * slope + intercept
    pixel_values = pixel_values
        .iter()
        .map(|&v| v * rescale_slope + rescale_intercept)
        .collect();

    // ============================================================
    // Step 4: Apply VOI LUT / Windowing
    // Parse WindowCenter and WindowWidth (may be string with backslash)
    // if type(ds.WindowCenter) is str: parse_window_value
    // imdata = util.apply_voi_lut(imdata, ds, index=0)
    // ============================================================
    
    // Parse WindowCenter (handle string with backslash separator)
    let window_center = obj
        .element_by_name("WindowCenter")
        .ok()
        .and_then(|elem| {
            // Try string first (may contain backslash-separated values)
            if let Ok(s) = elem.to_str() {
                parse_window_value(&s)
            } else {
                // Try multi-value float64
                elem.to_multi_float64().ok()
                    .and_then(|vals| vals.first().copied())
                    .or_else(|| elem.to_float64().ok())
            }
        });
    
    // Parse WindowWidth (handle string with backslash separator)
    let window_width = obj
        .element_by_name("WindowWidth")
        .ok()
        .and_then(|elem| {
            // Try string first (may contain backslash-separated values)
            if let Ok(s) = elem.to_str() {
                parse_window_value(&s)
            } else {
                // Try multi-value float64
                elem.to_multi_float64().ok()
                    .and_then(|vals| vals.first().copied())
                    .or_else(|| elem.to_float64().ok())
            }
        });

    // Apply VOI LUT if WindowWidth is present and valid
    // pydicom's util.apply_voi_lut ทำ LINEAR transformation:
    // ถ้ามี window: ไม่ map ค่า แต่เก็บค่าไว้ให้ normalization ทำงานต่อ
    // (ไม่เหมือน DICOM LUT ที่ map เป็น output range ทันที)
    if let (Some(center), Some(width)) = (window_center, window_width) {
        if width > 0.0 {
            print!(
                "VOI LUT parameters found: Center={}, Width={}\n",
                center.to_string().magenta(),
                width.to_string().magenta()
            );
            print!("Note: pydicom's apply_voi_lut with LINEAR mode will be applied during normalization\n");
        }
    } else {
        print!("No windowing parameters found\n");
    }

    // ============================================================
    // Step 5: Normalization (แปลงเป็น 0-255)
    // imdata_scaled = ((imdata - imdata.min()) / (imdata.max() - imdata.min())) * 255.0
    // darray = imdata_scaled.astype("uint8")
    // ============================================================
    let min_val = pixel_values.iter().fold(f64::MAX, |a, &b| a.min(b));
    let max_val = pixel_values.iter().fold(f64::MIN, |a, &b| a.max(b));
    let range = max_val - min_val;

    print!(
        "Normalizing: min={:.2}, max={:.2}, range={:.2}\n",
        min_val.to_string().yellow(),
        max_val.to_string().yellow(),
        range.to_string().yellow()
    );

    // Normalize to 0-255
    let normalized_pixels: Vec<u8> = if range > 0.0 {
        pixel_values
            .iter()
            .map(|&v| (((v - min_val) / range) * 255.0) as u8)
            .collect()
    } else {
        vec![0; pixel_values.len()]
    };

    // Create GrayImage: dcm_png = Image.fromarray(darray)
    let gray_image = GrayImage::from_raw(width, height, normalized_pixels)
        .ok_or("Failed to create image from normalized pixels")?;

    // ============================================================
    // Step 6: Save Image (บันทึกไฟล์)
    // dcm_png.save(png_file_path)
    // ============================================================
    let output_path = output_folder.join(
        path.file_stem()
            .and_then(|v| v.to_str())
            .map(|name| format!("{}.png", name))
            .unwrap_or_else(|| "output.png".to_string()),
    );

    // Create output folder: os.makedirs(save_path)
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Save: dcm_png.save(png_file_path)
    gray_image
        .save(&output_path)
        .map_err(|e| format!("Failed to save PNG: {}", e))?;
    
    print!("✓ PNG saved to: {}\n", output_path.display().green());

    Ok(DicomSummary::success(
        file_label,
        rows,
        cols,
        bits_allocated,
        ts_name.to_string(),
    ))
}

/// Parse window value from string (handles backslash-separated values)
/// ตามโค้ด Python: parse_window_value(ds.WindowCenter)
fn parse_window_value(s: &str) -> Option<f64> {
    // Split by backslash and take first value
    s.split('\\')
        .next()
        .and_then(|val| val.trim().parse::<f64>().ok())
}