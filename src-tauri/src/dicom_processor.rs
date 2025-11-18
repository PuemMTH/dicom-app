use std::path::Path;

use dicom_object::open_file;
use dicom_pixeldata::PixelDecoder;
use dicom_transfer_syntax_registry::{TransferSyntaxIndex, TransferSyntaxRegistry};
use image::GrayImage;

use crate::models::{DicomSummary, DicomMetadata};

/// Helper function to calculate statistics (min, max, mean)
fn calculate_stats(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let min = values.iter().fold(f64::MAX, |a, &b| a.min(b));
    let max = values.iter().fold(f64::MIN, |a, &b| a.max(b));
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;
    (min, max, mean)
}

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
/// Returns: (DicomSummary, DicomMetadata)
pub fn convert_dicom_to_png(
    path: &Path,
    output_folder: &Path,
) -> Result<(DicomSummary, DicomMetadata), String> {
    use std::fs;
    
    // ============================================================
    // Step 1: Loading (อ่านไฟล์) - pydicom.dcmread(dicom_file_path, force=True)
    // ============================================================
    println!("\n{}", "=".repeat(60));
    println!("DEBUG RUST: Processing file: {}", path.file_name().and_then(|s| s.to_str()).unwrap_or("unknown"));
    println!("{}", "=".repeat(60));

    let obj = open_file(path).map_err(|e| e.to_string())?;
    let ts_uid = obj.meta().transfer_syntax();
    let ts_name = TransferSyntaxRegistry::default()
        .get(ts_uid)
        .map(|ts| ts.name())
        .unwrap_or(ts_uid);

    println!("DEBUG RUST: Transfer Syntax: {}", ts_uid);

    let file_label = path
        .file_name()
        .and_then(|v| v.to_str())
        .map(|name| name.to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Check if PixelData exists (Python pattern: if 'PixelData' in ds)
    // This ensures we know which tag to use before trying to decode
    if obj.element_by_name("PixelData").is_err() {
        return Err("Missing PixelData - file may not contain image data".to_string());
    }

    // Decode pixel data - now we know PixelData exists
    let pixel_data = obj.decode_pixel_data().map_err(|e| {
        format!("Could not decode pixel data (Transfer Syntax: {}): {}", ts_name, e)
    })?;
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

    if let Some(ref pi) = photometric_interpretation {
        println!("DEBUG RUST: PhotometricInterpretation = {}", pi);
    } else {
        println!("DEBUG RUST: PhotometricInterpretation not in ds");
    }

    // Extract PixelSpacing with fallback pattern (Python-style)
    let _pixel_spacing = get_pixel_spacing(&obj);

    // Convert to DynamicImage
    let dicom_image = pixel_data
        .to_dynamic_image(0)
        .map_err(|e| e.to_string())?;

    let (width, height) = (cols as u32, rows as u32);
    let raw_bytes = dicom_image.as_bytes();
    println!("DEBUG RUST: pixel_array shape: ({}, {}), bits_allocated: {}", rows, cols, bits_allocated);
    
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
        let max_val = pixel_values.iter().fold(f64::MIN, |a, &b| a.max(b));
        pixel_values = pixel_values.iter().map(|&v| max_val - v).collect();
    }

    // Debug: Before modality LUT
    let (min_before_mod, max_before_mod, mean_before_mod) = calculate_stats(&pixel_values);
    println!("DEBUG RUST: Before modality LUT - min: {}, max: {}, mean: {:.2}",
             min_before_mod, max_before_mod, mean_before_mod);

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

    println!("DEBUG RUST: RescaleSlope = {}, RescaleIntercept = {}",
             rescale_slope, rescale_intercept);

    // Apply modality LUT: pixel = raw * slope + intercept
    pixel_values = pixel_values
        .iter()
        .map(|&v| v * rescale_slope + rescale_intercept)
        .collect();

    // Debug: After modality LUT
    let (min_after_mod, max_after_mod, mean_after_mod) = calculate_stats(&pixel_values);
    println!("DEBUG RUST: After modality LUT - min: {}, max: {}, mean: {:.2}",
             min_after_mod, max_after_mod, mean_after_mod);

    // ============================================================
    // Step 4: Apply VOI LUT / Windowing
    // Parse WindowCenter and WindowWidth (may be string with comma)
    // if type(ds.WindowCenter) is str: parse_window_value
    // imdata = util.apply_voi_lut(imdata, ds, index=0)
    // ============================================================

    // Parse WindowCenter (handle string with comma separator)
    let window_center = obj
        .element_by_name("WindowCenter")
        .ok()
        .and_then(|elem| {
            // Debug: Show original value and type
            if let Ok(s) = elem.to_str() {
                println!("DEBUG RUST: WindowCenter (original) = {}, type = String", s);
                parse_window_value(&s)
            } else if let Ok(val) = elem.to_float64() {
                println!("DEBUG RUST: WindowCenter (original) = {}, type = f64", val);
                Some(val)
            } else if let Ok(vals) = elem.to_multi_float64() {
                if let Some(&first) = vals.first() {
                    println!("DEBUG RUST: WindowCenter (original) = {:?}, type = Vec<f64>", vals);
                    Some(first)
                } else {
                    None
                }
            } else {
                None
            }
        });

    // Parse WindowWidth (handle string with comma separator)
    let window_width = obj
        .element_by_name("WindowWidth")
        .ok()
        .and_then(|elem| {
            // Debug: Show original value and type
            if let Ok(s) = elem.to_str() {
                println!("DEBUG RUST: WindowWidth (original) = {}, type = String", s);
                parse_window_value(&s)
            } else if let Ok(val) = elem.to_float64() {
                println!("DEBUG RUST: WindowWidth (original) = {}, type = f64", val);
                Some(val)
            } else if let Ok(vals) = elem.to_multi_float64() {
                if let Some(&first) = vals.first() {
                    println!("DEBUG RUST: WindowWidth (original) = {:?}, type = Vec<f64>", vals);
                    Some(first)
                } else {
                    None
                }
            } else {
                None
            }
        });

    // Debug: Before VOI LUT
    let (min_before_voi, max_before_voi, mean_before_voi) = calculate_stats(&pixel_values);
    println!("DEBUG RUST: Before VOI LUT - min: {}, max: {}, mean: {:.2}",
             min_before_voi, max_before_voi, mean_before_voi);

    // Apply VOI LUT if both WindowCenter and WindowWidth are present
    // Python: imdata = util.apply_voi_lut(imdata, ds, index=0)
    // VOI LUT applies DICOM Standard LINEAR transformation (Part 3, C.11.2.1.2)
    // Formula:
    // if x <= c - 0.5 - (w-1)/2: y = y_min
    // else if x > c - 0.5 + (w-1)/2: y = y_max
    // else: y = ((x - (c - 0.5)) / (w - 1) + 0.5) * (y_max - y_min) + y_min
    if let (Some(center), Some(width)) = (window_center, window_width) {
        if width > 0.0 {
            println!("DEBUG RUST: Applying VOI LUT with center={}, width={}", center, width);

            // Determine output range (y_min, y_max) from pixel value range
            let current_min = pixel_values.iter().fold(f64::MAX, |a, &b| a.min(b));
            let current_max = pixel_values.iter().fold(f64::MIN, |a, &b| a.max(b));
            let y_min = current_min;
            let y_max = current_max;

            // Apply DICOM LINEAR VOI LUT transformation
            let lower_bound = center - 0.5 - (width - 1.0) / 2.0;
            let upper_bound = center - 0.5 + (width - 1.0) / 2.0;

            pixel_values = pixel_values
                .iter()
                .map(|&x| {
                    if x <= lower_bound {
                        y_min
                    } else if x > upper_bound {
                        y_max
                    } else {
                        // Linear interpolation
                        ((x - (center - 0.5)) / (width - 1.0) + 0.5) * (y_max - y_min) + y_min
                    }
                })
                .collect();
        }
    } else {
        println!("DEBUG RUST: Skipping VOI LUT (WindowCenter or WindowWidth not present)");
    }

    // Debug: After VOI LUT
    let (min_after_voi, max_after_voi, mean_after_voi) = calculate_stats(&pixel_values);
    println!("DEBUG RUST: After VOI LUT - min: {}, max: {}, mean: {:.2}",
             min_after_voi, max_after_voi, mean_after_voi);

    // ============================================================
    // Step 5: Normalization (แปลงเป็น 0-255)
    // imdata_scaled = ((imdata - imdata.min()) / (imdata.max() - imdata.min())) * 255.0
    // darray = imdata_scaled.astype("uint8")
    // ============================================================
    let min_val = pixel_values.iter().fold(f64::MAX, |a, &b| a.min(b));
    let max_val = pixel_values.iter().fold(f64::MIN, |a, &b| a.max(b));
    let range = max_val - min_val;

    // Normalize to 0-255
    let normalized_pixels: Vec<u8> = if range > 0.0 {
        pixel_values
            .iter()
            .map(|&v| (((v - min_val) / range) * 255.0) as u8)
            .collect()
    } else {
        vec![0; pixel_values.len()]
    };

    // Debug: After normalization
    let min_norm = *normalized_pixels.iter().min().unwrap_or(&0);
    let max_norm = *normalized_pixels.iter().max().unwrap_or(&0);
    println!("DEBUG RUST: After normalization - min: {}, max: {}", min_norm, max_norm);

    // Debug: Final uint8 array
    println!("DEBUG RUST: Final uint8 array - min: {}, max: {}, shape: ({}, {})",
             min_norm, max_norm, rows, cols);

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

    println!("DEBUG RUST: PNG saved to: {}", output_path.display());
    println!("{}", "=".repeat(60));

    // Extract metadata (Python: metadata = dicom_metadata(dicom_file_path))
    let metadata = extract_metadata(path, &obj);

    Ok((
        DicomSummary::success(
            file_label,
            rows,
            cols,
            bits_allocated,
            ts_name.to_string(),
        ),
        metadata,
    ))
}

/// Parse window value from string (handles comma-separated values)
/// ตามโค้ด Python: parse_window_value(ds.WindowCenter)
/// Python: cleaned_value = value.split(',')[0].strip()
fn parse_window_value(s: &str) -> Option<f64> {
    // Split by comma and take first value (Python uses comma, not backslash!)
    s.split(',')
        .next()
        .and_then(|val| val.trim().parse::<f64>().ok())
}

/// Extract PixelSpacing with fallback pattern (Python-style)
/// try:
///     if hasattr(ds, 'PixelSpacing'): pixel_spacing = ds.PixelSpacing
///     elif hasattr(ds, 'ImagerPixelSpacing'): pixel_spacing = ds.ImagerPixelSpacing
///     else: pixel_spacing = ds[0x28,0x30].value
/// except: pixel_spacing = None
fn get_pixel_spacing(obj: &dicom_object::InMemDicomObject) -> Option<Vec<f64>> {
    // Try PixelSpacing first (0028,0030)
    if let Ok(elem) = obj.element_by_name("PixelSpacing") {
        if let Ok(values) = elem.to_multi_float64() {
            return Some(values.to_vec());
        }
    }

    // Try ImagerPixelSpacing (0018,1164)
    if let Ok(elem) = obj.element_by_name("ImagerPixelSpacing") {
        if let Ok(values) = elem.to_multi_float64() {
            return Some(values.to_vec());
        }
    }

    // Try direct tag access (0x28, 0x30)
    if let Ok(elem) = obj.element((0x0028, 0x0030).into()) {
        if let Ok(values) = elem.to_multi_float64() {
            return Some(values.to_vec());
        }
    }

    // Return None if all attempts fail
    None
}

/// Extract DICOM metadata following Python's dicom_metadata() pattern
/// Python code (lines 55-151 in anonydicom.py):
/// - F_name: Path(f_path).stem + '.png'
/// - Study_date: try ds.StudyDate except AttributeError → None
/// - Modality: try ds.Modality except AttributeError → None
/// - Manufacturer: try ds.Manufacturer except → None
/// - Study_description: try ds[0x0008,0x1030].value except → None
/// - Series_description: try ds[0x0008,0x103E].value except → None
/// - Institution_name: try ds[0x0008,0x0080].value except → None
/// - Im_width: try pixel_array.shape[0] except → None
/// - Im_height: try pixel_array.shape[1] except → None
/// - Pixel_spacing: use get_pixel_spacing fallback pattern
pub fn extract_metadata(path: &Path, obj: &dicom_object::InMemDicomObject) -> DicomMetadata {
    // F_name: Path(f_path).stem + '.png'
    let f_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{}.png", s))
        .unwrap_or_else(|| "unknown.png".to_string());

    // Study_date: try ds.StudyDate except AttributeError → None
    let study_date = obj
        .element_by_name("StudyDate")
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Modality: try ds.Modality except AttributeError → None
    let modality = obj
        .element_by_name("Modality")
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Manufacturer: try ds.Manufacturer except → None
    let manufacturer = obj
        .element_by_name("Manufacturer")
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Study_description: try ds[0x0008,0x1030].value except → None
    let study_description = obj
        .element((0x0008, 0x1030).into())
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Series_description: try ds[0x0008,0x103E].value except → None
    let series_description = obj
        .element((0x0008, 0x103E).into())
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Institution_name: try ds[0x0008,0x0080].value except → None
    let institution_name = obj
        .element((0x0008, 0x0080).into())
        .ok()
        .and_then(|elem| elem.to_str().ok())
        .map(|s| s.to_string());

    // Im_width and Im_height: try pixel_array.shape except → None
    // ใน Python: dcmimg_uint.shape[0] = height, shape[1] = width
    // ใน DICOM: Rows (0028,0010) = height, Columns (0028,0011) = width
    let im_height = obj
        .element_by_name("Rows")
        .ok()
        .and_then(|elem| elem.to_int::<u32>().ok());

    let im_width = obj
        .element_by_name("Columns")
        .ok()
        .and_then(|elem| elem.to_int::<u32>().ok());

    // Pixel_spacing: use get_pixel_spacing fallback pattern
    let pixel_spacing = get_pixel_spacing(obj);

    DicomMetadata {
        f_name,
        study_date,
        modality,
        manufacturer,
        study_description,
        series_description,
        institution_name,
        im_width,
        im_height,
        pixel_spacing,
    }
}