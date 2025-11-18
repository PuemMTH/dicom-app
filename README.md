# DICOM Converter App

DICOM file processing application built with Tauri + Solid + TypeScript

## Features

- **DICOM to PNG Conversion**: Convert DICOM medical images to PNG format
- **Folder Structure Preservation**: Maintains original folder hierarchy in output
- **Batch Processing**: Process multiple DICOM files at once
- **Detailed Statistics**: Track successful/failed conversions with error details
- **Python-Compatible Output**: Output structure matches Python dicom-converter format

## DICOM to PNG Conversion

### Command: `dicom_to_png`

**Parameters:**
- `input_folder`: Path to folder containing DICOM files
- `output_folder`: Path where output will be saved

**Output Structure:**
```
output_folder/
└── output_{input_folder_name}/
    └── png_file/
        └── [preserved folder structure]/
            └── *.png files
```

**Supported Transfer Syntaxes:**
- JPEG Baseline (1.2.840.10008.1.2.4.50)
- JPEG 2000 (openjpeg-sys)
- RLE Lossless
- JPEG-XL
- Deflate
- JPEG Lossless (requires charls - see installation notes below)

**Note on JPEG Lossless Support:**
To enable JPEG Lossless transfer syntax support, you need to install system dependencies:
```bash
sudo apt-get update
sudo apt-get install build-essential cmake
```
Then rebuild the project.

**Return Value:**
```typescript
{
  mainOutputFolder: string,
  total: number,
  successful: number,
  failed: number,
  failedFiles: string[],
  errorDetails: FileDetail[],
  allFileDetails: FileDetail[]
}
```

### FileDetail Structure:
```typescript
{
  fileName: string,
  filePath: string,
  success: boolean,
  errorType?: string,
  errorMessage?: string,
  conversionType: "PNG" | "DICOM"
}
```

## Development

### If using VSCode and encountering bugs:

```bash
export GTK_PATH=""
export GIO_MODULE_DIR=""
npm run tauri dev
```

## Build

```bash
npm run tauri build
```