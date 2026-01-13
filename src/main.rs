//! Apple Notes Exporter CLI
//!
//! A command-line tool for exporting Apple Notes folders to the file system via AppleScript.
//! 
//! ## Quick Start
//! 
//! ```bash
//! cargo run -- export 'My Notes' ./exports
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use apple_notes_exporter_rs::{extract_attachments_from_directory, Exporter};

/// Relative path to the vendored AppleScript (used when running from source).
const VENDORED_SCRIPT_PATH: &str = "vendor/apple-notes-exporter/scripts/export_notes.applescript";

#[derive(Parser, Debug)]
#[command(author, version, about = "Export Apple Notes folders via AppleScript")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all available top-level folders across all accounts
    #[command(alias = "ls")]
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

        /// Skip extracting embedded images from HTML files.
        /// By default, images are extracted to "<note-name>-attachments/" subdirectories.
        #[arg(long)]
        no_extract_attachments: bool,
    },

    /// Extract embedded images from previously exported HTML files
    ///
    /// Scans a directory for HTML files and extracts base64-encoded images
    /// to "<note-name>-attachments/" subdirectories. Updates the HTML files
    /// to reference the extracted images.
    ExtractAttachments {
        /// Directory containing exported HTML files
        #[arg(value_name = "DIR")]
        dir: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(error) = run(cli) {
        eprintln!("Error: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run(cli: Cli) -> apple_notes_exporter_rs::Result<()> {
    // Try to use vendored script if available (when running from source),
    // otherwise fall back to embedded script
    let exporter = match Exporter::with_script_path(VENDORED_SCRIPT_PATH) {
        Ok(e) => e,
        Err(_) => Exporter::new(),
    };

    match cli.command {
        Commands::List => exporter.list_folders(),
        Commands::Export {
            folder,
            output_dir,
            no_extract_attachments,
        } => {
            if no_extract_attachments {
                exporter.export_folder(&folder, &output_dir)
            } else {
                let results = exporter.export_folder_with_attachments(&folder, &output_dir)?;
                let total: usize = results.iter().map(|r| r.attachments.len()).sum();
                if total > 0 {
                    eprintln!("Extracted {total} attachments from {} files", results.len());
                }
                Ok(())
            }
        }
        Commands::ExtractAttachments { dir } => {
            let results = extract_attachments_from_directory(&dir)?;
            let total: usize = results.iter().map(|r| r.attachments.len()).sum();
            let modified: usize = results.iter().filter(|r| r.html_modified).count();
            eprintln!(
                "Extracted {total} attachments from {modified} files ({} files scanned)",
                results.len()
            );
            Ok(())
        }
    }
}
