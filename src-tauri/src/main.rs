// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use dicom_app_lib::cli::Cli;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // If there are arguments (more than just the program name), try to run as CLI
    if args.len() > 1 {
        match Cli::try_parse() {
            Ok(cli) => {
                dicom_app_lib::cli::run_cli(cli);
            }
            Err(e) => {
                // If parsing fails (e.g. --help, --version, or invalid args), print it
                // This will exit the process with appropriate code for help/version
                // For errors, it prints to stderr
                let _ = e.print();
            }
        }
    } else {
        // No arguments, run the GUI
        dicom_app_lib::run();
    }
}
