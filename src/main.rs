//! Apple Notes Exporter
//!
//! A command-line tool for exporting Apple Notes folders to the file system via AppleScript.
//! This tool allows you to specify one Apple Notes folder name and recursively export it
//! (including all subfolders and notes) to a specified output directory.
//!
//! The tool creates a mirrored directory tree and exports each note as an HTML file.
//! The folder search uses breadth-first search (BFS) and searches recursively at ALL levels
//! (not just top-level) to find the folder. Once found, it exports that folder and all its
//! subfolders recursively.
//!
//! ## Usage
//!
//! The tool requires an output directory and a folder name to export.
//! Only one folder can be exported at a time. The export is recursive and includes
//! all subfolders and notes within the specified folder.
//!
//! By default, the folder search looks in the "iCloud" account (most common case).
//! If a folder name exists in multiple accounts, you can specify the account using
//! the "AccountName:FolderName" format (e.g., "Google:My Notes").
//!
//! **Note:** This tool requires Automation permissions for the Notes app.
//! Grant these permissions in System Settings > Privacy & Security > Automation.
//!
//! ## Example
//!
//! ```bash
//! apple-notes-exporter -o ./exports "My Notes"
//! ```
//!
//! For folders with duplicate names across accounts:
//!
//! ```bash
//! apple-notes-exporter -o ./exports "iCloud:My Notes"
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;

const SCRIPT_PATH: &str = "vendor/apple-notes-exporter/scripts/export_notes_recursive.applescript";

#[derive(Parser, Debug)]
#[command(author, version, about = "Recursively export Apple Notes folders via AppleScript")]
struct Args {
    /// Output directory for exported notes.
    #[arg(short = 'o', long = "output-dir", value_name = "DIR")]
    output_dir: PathBuf,

    /// Apple Notes folder name to export recursively. Only one folder can be exported at a time.
    /// The export includes all subfolders and notes within the specified folder.
    ///
    /// The folder search uses breadth-first search and searches recursively at ALL levels
    /// (not just top-level) to find the folder. By default, searches in the "iCloud" account.
    /// If a folder name exists in multiple accounts, use "AccountName:FolderName" format
    /// (e.g., "Google:My Notes").
    #[arg(value_name = "NOTES_FOLDER")]
    folder: String,
}

fn main() {
    let args = Args::parse();

    if let Err(error) = run(args) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let script_path = PathBuf::from(SCRIPT_PATH);
    ensure_script_exists(&script_path)?;
    create_output_dir(&args.output_dir)?;

    let script = script_path.canonicalize().map_err(|err| {
        format!(
            "Unable to resolve script path {}: {err}",
            script_path.display()
        )
    })?;

    let output_dir = args.output_dir.canonicalize().map_err(|err| {
        format!(
            "Unable to resolve output directory {}: {err}",
            args.output_dir.display()
        )
    })?;

    let output_dir_str = output_dir
        .to_str()
        .ok_or_else(|| "Output directory path is not valid UTF-8".to_string())?;

    let status = Command::new("osascript")
        .arg(&script)
        .arg(&args.folder)
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
