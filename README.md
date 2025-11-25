# DICOM Converter App

A high-performance DICOM processing application built with **Tauri v2**, **SolidJS**, and **Rust**.

## 🚀 Features

- **Dual Mode**: Run as a modern GUI application or a headless CLI tool.
- **High Performance**: Utilizes multi-threaded processing (Rayon) for fast conversion and anonymization.
- **Incremental Saving**: Writes metadata and logs to disk immediately as files are processed, preventing data loss.
- **DICOM to PNG**: High-quality conversion preserving 16-bit depth information where applicable.
- **Anonymization**: robust de-identification of sensitive patient data with customizable tag replacement.
- **Smart Skipping**: Automatically skips already processed files to resume interrupted jobs efficiently.
- **Detailed Reporting**: Generates comprehensive CSV reports (`metadata_all.csv`, `logs.csv`) and JSON summaries.

## 🏗️ Architecture

The application follows a client-server architecture using Tauri's IPC protocol.

```mermaid
graph TD
    subgraph Frontend ["Frontend (SolidJS)"]
        Page["HomePage.tsx"]
        Store["State Management"]
        Invoke["Tauri Invoke"]
        Listener["Event Listener"]
    end

    subgraph Backend ["Backend (Rust)"]
        Command["process_dicom (commands.rs)"]
        
        subgraph Logic ["Business Logic"]
            Workflow["workflow.rs"]
            Anonymize["anonymize.rs"]
            Convert["convert.rs"]
        end

        subgraph ParallelProcessing ["Parallel Processing (Rayon)"]
            Worker["Worker Threads"]
            Channel["MPSC Channel"]
        end

        subgraph WriterThread ["Writer Thread"]
            WriterLogic["Result Collector"]
            MetaWriter["MetadataWriter"]
            LogWriter["LogWriter"]
        end

        subgraph IO ["Disk I/O"]
            Files["Output Files (PNG/DICOM)"]
            CSV["metadata_all.csv"]
            Logs["logs.csv"]
        end
    end

    %% User Interaction
    Page -->|"User Input"| Store
    Store -->|"Start Processing"| Invoke

    %% IPC Call
    Invoke -->|"invoke('process_dicom', input)"| Command

    %% Backend Processing
    Command -->|"If Convert"| Workflow
    Command -->|"If Anonymize"| Anonymize
    
    Workflow -->|"Spawn"| Worker
    Anonymize -->|"Spawn"| Worker
    
    Worker -->|"Convert Single File"| Convert
    Worker -->|"Write File"| Files

    %% Incremental Saving Flow
    Workflow -->|"Spawn"| WriterLogic
    Anonymize -->|"Spawn"| WriterLogic
    
    Worker --"Send Result"--> Channel
    Channel --"Receive Result"--> WriterLogic
    
    WriterLogic -->|"Write Record"| MetaWriter
    WriterLogic -->|"Write Entry"| LogWriter
    
    MetaWriter -->|"Append"| CSV
    LogWriter -->|"Append"| Logs

    %% Progress Reporting
    Worker -.->|"Callback"| Command
    Command -.->|"emit('progress')"| Listener

    %% Feedback Loop
    Listener -->|"Update UI"| Page
```

## 🛠️ Development

### Running Locally

```bash
# Install frontend dependencies
npm install

# Run in development mode (GUI)
npm run tauri dev

# Run CLI in development mode
cargo run -- --help
```