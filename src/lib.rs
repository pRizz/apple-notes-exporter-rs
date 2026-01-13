//! Apple Notes Exporter Library
//!
//! A library for exporting Apple Notes folders to the file system via AppleScript.
//!
//! This library provides functions to list available Apple Notes folders and export them
//! recursively to HTML files. It works by invoking an embedded AppleScript that interacts
//! with the Notes app.
//!
//! ## Requirements
//!
//! - **macOS only** - This library relies on AppleScript and the Notes app, which are
//!   only available on macOS. Attempting to compile on other platforms will result in
//!   a compile-time error.
//! - Automation permissions for the Notes app must be granted in System Settings
//!
//! ## Quick Start
//!
//! ```no_run
//! use apple_notes_exporter_rs::{list_folders, export_folder, export_folder_from_account};
//!
//! // List all available folders
//! list_folders().expect("Failed to list folders");
//!
//! // Export a folder to a directory (searches all accounts)
//! export_folder("My Notes", "./exports").expect("Failed to export");
//!
//! // Export from a specific account (useful when folder names are duplicated)
//! export_folder_from_account("iCloud", "Work", "./exports").expect("Failed to export");
//! ```
//!
//! ## Using a Custom Script
//!
//! If you need to use a custom AppleScript (e.g., a modified version), use the [`Exporter`] struct:
//!
//! ```no_run
//! use apple_notes_exporter_rs::Exporter;
//!
//! let exporter = Exporter::with_script_path("./my_custom_script.applescript")
//!     .expect("Script not found");
//!
//! exporter.list_folders().expect("Failed to list folders");
//! exporter.export_folder("My Notes", "./exports").expect("Failed to export");
//! ```

#[cfg(not(target_os = "macos"))]
compile_error!(
    "apple-notes-exporter-rs only works on macOS. \
     It relies on AppleScript and the Notes app, which are not available on other platforms."
);

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// The embedded AppleScript used for exporting notes.
const EMBEDDED_SCRIPT: &str =
    include_str!("../vendor/apple-notes-exporter/scripts/export_notes.applescript");

/// Checks if the current platform is macOS and returns an error if not.
#[cfg(target_os = "macos")]
fn check_platform() -> Result<()> {
    Ok(())
}

/// Checks if the current platform is macOS and returns an error if not.
#[cfg(not(target_os = "macos"))]
fn check_platform() -> Result<()> {
    Err(ExportError::UnsupportedPlatform(std::env::consts::OS))
}

/// Errors that can occur during Apple Notes export operations.
#[derive(Error, Debug)]
pub enum ExportError {
    /// The current platform is not supported (only macOS is supported).
    #[error(
        "This tool only works on macOS. It relies on AppleScript and the Notes app, \
         which are not available on {0}."
    )]
    UnsupportedPlatform(&'static str),

    /// The AppleScript file was not found at the specified path.
    #[error("AppleScript not found at {0}")]
    ScriptNotFound(PathBuf),

    /// Failed to create a temporary file for the embedded script.
    #[error("Failed to create temporary script file: {0}")]
    TempFileError(#[from] std::io::Error),

    /// The output directory path is not valid UTF-8.
    #[error("Output directory path is not valid UTF-8")]
    InvalidUtf8Path,

    /// Failed to launch the osascript process.
    #[error("Failed to launch osascript: {0}")]
    LaunchError(std::io::Error),

    /// The AppleScript exited with a non-zero status code.
    #[error("AppleScript exited with status {0}")]
    ScriptFailed(i32),
}

/// Result type alias for export operations.
pub type Result<T> = std::result::Result<T, ExportError>;

/// An Apple Notes exporter that can list folders and export notes.
///
/// Use [`Exporter::new()`] for the default embedded script, or
/// [`Exporter::with_script_path()`] for a custom script.
#[derive(Debug)]
pub struct Exporter {
    script_source: ScriptSource,
}

#[derive(Debug)]
enum ScriptSource {
    Embedded,
    Path(PathBuf),
}

impl Default for Exporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter {
    /// Creates a new exporter using the embedded AppleScript.
    ///
    /// This is the recommended way to create an exporter for most use cases.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::new();
    /// exporter.list_folders().expect("Failed to list folders");
    /// ```
    pub fn new() -> Self {
        Self {
            script_source: ScriptSource::Embedded,
        }
    }

    /// Creates a new exporter using a custom AppleScript at the specified path.
    ///
    /// Returns an error if the script file does not exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::with_script_path("./custom_script.applescript")
    ///     .expect("Script not found");
    /// ```
    pub fn with_script_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(ExportError::ScriptNotFound(path));
        }
        Ok(Self {
            script_source: ScriptSource::Path(path),
        })
    }

    /// Lists all available top-level folders across all Apple Notes accounts.
    ///
    /// The output is printed to stdout by the AppleScript.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::new();
    /// exporter.list_folders().expect("Failed to list folders");
    /// ```
    pub fn list_folders(&self) -> Result<()> {
        self.run_script(&["list"])
    }

    /// Exports a folder recursively to HTML files.
    ///
    /// The folder search uses breadth-first search and looks at all levels
    /// (not just top-level) to find the folder. Once found, it exports that
    /// folder and all its subfolders recursively.
    ///
    /// This method searches all accounts for the folder. If a folder with the
    /// same name exists in multiple accounts, use [`export_folder_from_account`](Self::export_folder_from_account)
    /// to specify which account to use.
    ///
    /// # Arguments
    ///
    /// * `folder` - The folder name to export.
    /// * `output_dir` - The directory where exported notes will be saved.
    ///   Will be created if it doesn't exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::new();
    /// exporter.export_folder("My Notes", "./exports").expect("Failed to export");
    /// ```
    pub fn export_folder<P: AsRef<Path>>(&self, folder: &str, output_dir: P) -> Result<()> {
        self.export_folder_impl(folder, output_dir)
    }

    /// Exports a folder from a specific account recursively to HTML files.
    ///
    /// This is useful when a folder with the same name exists in multiple accounts.
    /// The folder search uses breadth-first search and looks at all levels
    /// (not just top-level) to find the folder within the specified account.
    ///
    /// # Arguments
    ///
    /// * `account` - The account name (e.g., "iCloud", "Google", "On My Mac").
    /// * `folder` - The folder name to export.
    /// * `output_dir` - The directory where exported notes will be saved.
    ///   Will be created if it doesn't exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::new();
    ///
    /// // Export "Work" folder from iCloud account
    /// exporter.export_folder_from_account("iCloud", "Work", "./exports")
    ///     .expect("Failed to export");
    ///
    /// // Export "Work" folder from Google account
    /// exporter.export_folder_from_account("Google", "Work", "./google_exports")
    ///     .expect("Failed to export");
    /// ```
    pub fn export_folder_from_account<P: AsRef<Path>>(
        &self,
        account: &str,
        folder: &str,
        output_dir: P,
    ) -> Result<()> {
        let folder_spec = format!("{account}:{folder}");
        self.export_folder_impl(&folder_spec, output_dir)
    }

    fn export_folder_impl<P: AsRef<Path>>(&self, folder_spec: &str, output_dir: P) -> Result<()> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir)?;

        let output_dir = output_dir.canonicalize()?;
        let output_dir_str = output_dir.to_str().ok_or(ExportError::InvalidUtf8Path)?;

        self.run_script(&["export", folder_spec, output_dir_str])
    }

    fn run_script(&self, args: &[&str]) -> Result<()> {
        check_platform()?;

        match &self.script_source {
            ScriptSource::Embedded => self.run_embedded_script(args),
            ScriptSource::Path(path) => self.run_script_file(path, args),
        }
    }

    fn run_embedded_script(&self, args: &[&str]) -> Result<()> {
        // Create a temporary file for the embedded script
        let mut temp_file = tempfile::NamedTempFile::with_suffix(".applescript")?;
        temp_file.write_all(EMBEDDED_SCRIPT.as_bytes())?;
        temp_file.flush()?;

        let status = Command::new("osascript")
            .arg(temp_file.path())
            .args(args)
            .status()
            .map_err(ExportError::LaunchError)?;

        if !status.success() {
            return Err(ExportError::ScriptFailed(status.code().unwrap_or(-1)));
        }

        Ok(())
    }

    fn run_script_file(&self, script_path: &Path, args: &[&str]) -> Result<()> {
        let script = script_path.canonicalize()?;

        let status = Command::new("osascript")
            .arg(&script)
            .args(args)
            .status()
            .map_err(ExportError::LaunchError)?;

        if !status.success() {
            return Err(ExportError::ScriptFailed(status.code().unwrap_or(-1)));
        }

        Ok(())
    }
}

/// Lists all available top-level folders across all Apple Notes accounts.
///
/// This is a convenience function that uses the embedded AppleScript.
/// For more control, use the [`Exporter`] struct.
///
/// # Example
///
/// ```no_run
/// use apple_notes_exporter_rs::list_folders;
///
/// list_folders().expect("Failed to list folders");
/// ```
pub fn list_folders() -> Result<()> {
    Exporter::new().list_folders()
}

/// Exports a folder recursively to HTML files.
///
/// This is a convenience function that uses the embedded AppleScript.
/// For more control, use the [`Exporter`] struct.
///
/// This function searches all accounts for the folder. If a folder with the
/// same name exists in multiple accounts, use [`export_folder_from_account`]
/// to specify which account to use.
///
/// # Arguments
///
/// * `folder` - The folder name to export.
/// * `output_dir` - The directory where exported notes will be saved.
///
/// # Example
///
/// ```no_run
/// use apple_notes_exporter_rs::export_folder;
///
/// export_folder("My Notes", "./exports").expect("Failed to export");
/// ```
pub fn export_folder<P: AsRef<Path>>(folder: &str, output_dir: P) -> Result<()> {
    Exporter::new().export_folder(folder, output_dir)
}

/// Exports a folder from a specific account recursively to HTML files.
///
/// This is a convenience function that uses the embedded AppleScript.
/// For more control, use the [`Exporter`] struct.
///
/// This is useful when a folder with the same name exists in multiple accounts.
///
/// # Arguments
///
/// * `account` - The account name (e.g., "iCloud", "Google", "On My Mac").
/// * `folder` - The folder name to export.
/// * `output_dir` - The directory where exported notes will be saved.
///
/// # Example
///
/// ```no_run
/// use apple_notes_exporter_rs::export_folder_from_account;
///
/// // Export "Work" folder from iCloud account
/// export_folder_from_account("iCloud", "Work", "./exports").expect("Failed to export");
///
/// // Export "Work" folder from Google account
/// export_folder_from_account("Google", "Work", "./google_exports").expect("Failed to export");
/// ```
pub fn export_folder_from_account<P: AsRef<Path>>(
    account: &str,
    folder: &str,
    output_dir: P,
) -> Result<()> {
    Exporter::new().export_folder_from_account(account, folder, output_dir)
}
