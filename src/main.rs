use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Export Apple Notes folders via AppleScript")]
struct Args {
    /// Output directory for exported notes.
    #[arg(short, long, value_name = "DIR")]
    output_dir: PathBuf,

    /// Apple Notes folder name to export. Can be specified multiple times.
    #[arg(short, long, value_name = "FOLDER", required = true)]
    folder: Vec<String>,

    /// Path to the AppleScript used for exporting.
    #[arg(long, value_name = "SCRIPT", default_value = "vendor/apple-notes-exporter/export-notes.applescript")]
    script: PathBuf,
}

fn main() {
    let args = Args::parse();

    if let Err(error) = run(args) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    ensure_script_exists(&args.script)?;
    create_output_dir(&args.output_dir)?;

    let script = args.script.canonicalize().map_err(|err| {
        format!(
            "Unable to resolve script path {}: {err}",
            args.script.display()
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
        .arg(output_dir_str)
        .args(&args.folder)
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
