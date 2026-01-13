//! Apple Notes Exporter
//!
//! A command-line tool for exporting Apple Notes folders to the file system via AppleScript.
//! This tool allows you to list available folders and recursively export them
//! (including all subfolders and notes) to a specified output directory.
//!
//! The tool creates a mirrored directory tree and exports each note as an HTML file.
//! The folder search uses breadth-first search (BFS) and searches recursively at ALL levels
//! (not just top-level) to find the folder. Once found, it exports that folder and all its
//! subfolders recursively.
//!
//! ## Usage
//!
//! List all available top-level folders:
//!
//! ```bash
//! apple-notes-exporter list
//! ```
//!
//! Export a folder recursively:
//!
//! ```bash
//! apple-notes-exporter export "My Notes" ./exports
//! ```
//!
//! For folders with duplicate names across accounts, use "AccountName:FolderName" format:
//!
//! ```bash
//! apple-notes-exporter export "iCloud:My Notes" ./exports
//! ```
//!
//! **Note:** This tool requires Automation permissions for the Notes app.
//! Grant these permissions in System Settings > Privacy & Security > Automation.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};

const SCRIPT_PATH: &str = "vendor/apple-notes-exporter/scripts/export_notes.applescript";

#[derive(Parser, Debug)]
#[command(author, version, about = "Export Apple Notes folders via AppleScript")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all available top-level folders across all accounts
    List,

    /// Export a folder recursively to HTML files
    ///
    /// The folder search uses breadth-first search and searches recursively at ALL levels
    /// (not just top-level) to find the folder. By default, searches all accounts.
    /// If a folder name exists in multiple accounts, use "AccountName:FolderName" format
    /// (e.g., "iCloud:My Notes").
    Export {
        /// Apple Notes folder name to export recursively.
        /// Use "AccountName:FolderName" format for folders in specific accounts.
        #[arg(value_name = "FOLDER")]
        folder: String,

        /// Output directory for exported notes
        #[arg(value_name = "OUTPUT_DIR")]
        output_dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(error) = run(cli) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let script_path = PathBuf::from(SCRIPT_PATH);
    ensure_script_exists(&script_path)?;

    let script = script_path.canonicalize().map_err(|err| {
        format!(
            "Unable to resolve script path {}: {err}",
            script_path.display()
        )
    })?;

    match cli.command {
        Commands::List => run_list(&script),
        Commands::Export { folder, output_dir } => run_export(&script, &folder, &output_dir),
    }
}

fn run_list(script: &Path) -> Result<(), String> {
    let status = Command::new("osascript")
        .arg(script)
        .arg("list")
        .status()
        .map_err(|err| format!("Failed to launch osascript: {err}"))?;

    if !status.success() {
        return Err(format!(
            "AppleScript exited with status {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

fn run_export(script: &Path, folder: &str, output_dir: &Path) -> Result<(), String> {
    create_output_dir(output_dir)?;

    let output_dir = output_dir.canonicalize().map_err(|err| {
        format!(
            "Unable to resolve output directory {}: {err}",
            output_dir.display()
        )
    })?;

    let output_dir_str = output_dir
        .to_str()
        .ok_or_else(|| "Output directory path is not valid UTF-8".to_string())?;

    let status = Command::new("osascript")
        .arg(script)
        .arg("export")
        .arg(folder)
        .arg(output_dir_str)
        .status()
        .map_err(|err| format!("Failed to launch osascript: {err}"))?;

    if !status.success() {
        return Err(format!(
            "AppleScript exited with status {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

fn ensure_script_exists(script_path: &Path) -> Result<(), String> {
    if script_path.exists() {
        return Ok(());
    }

    Err(format!(
        "AppleScript not found at {}",
        script_path.display()
    ))
}

fn create_output_dir(output_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|err| {
        format!(
            "Unable to create output directory {}: {err}",
            output_dir.display()
        )
    })
}
