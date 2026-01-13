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
//!   only available on macOS. Running on other platforms will return an error at runtime.
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

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::prelude::*;
use scraper::{Html, Selector};
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

    /// Failed to decode base64 image data.
    #[error("Failed to decode base64 image: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
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

    /// Exports a folder and extracts all embedded images to attachment folders.
    ///
    /// This combines [`export_folder`](Self::export_folder) with
    /// [`extract_attachments_from_directory`] for convenience.
    ///
    /// # Arguments
    ///
    /// * `folder` - The folder name to export.
    /// * `output_dir` - The directory where exported notes will be saved.
    ///
    /// # Returns
    ///
    /// Returns a vector of `ExtractionResult` for each HTML file processed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use apple_notes_exporter_rs::Exporter;
    ///
    /// let exporter = Exporter::new();
    /// let results = exporter.export_folder_with_attachments("My Notes", "./exports")
    ///     .expect("Failed to export");
    ///
    /// let total: usize = results.iter().map(|r| r.attachments.len()).sum();
    /// println!("Extracted {total} attachments");
    /// ```
    pub fn export_folder_with_attachments<P: AsRef<Path>>(
        &self,
        folder: &str,
        output_dir: P,
    ) -> Result<Vec<ExtractionResult>> {
        self.export_folder(folder, &output_dir)?;
        extract_attachments_from_directory(&output_dir)
    }

    /// Exports a folder from a specific account and extracts all embedded images.
    ///
    /// This combines [`export_folder_from_account`](Self::export_folder_from_account) with
    /// [`extract_attachments_from_directory`] for convenience.
    ///
    /// # Arguments
    ///
    /// * `account` - The account name (e.g., "iCloud", "Google", "On My Mac").
    /// * `folder` - The folder name to export.
    /// * `output_dir` - The directory where exported notes will be saved.
    ///
    /// # Returns
    ///
    /// Returns a vector of `ExtractionResult` for each HTML file processed.
    pub fn export_folder_from_account_with_attachments<P: AsRef<Path>>(
        &self,
        account: &str,
        folder: &str,
        output_dir: P,
    ) -> Result<Vec<ExtractionResult>> {
        self.export_folder_from_account(account, folder, &output_dir)?;
        extract_attachments_from_directory(&output_dir)
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

// =============================================================================
// Attachment Extraction
// =============================================================================

/// Information about an extracted attachment.
#[derive(Debug, Clone)]
pub struct ExtractedAttachment {
    /// The file path where the attachment was saved.
    pub path: PathBuf,
    /// The original data URL that was replaced.
    pub original_data_url: String,
    /// The MIME type of the attachment (e.g., "image/png").
    pub mime_type: String,
}

/// Result of extracting attachments from an HTML file.
#[derive(Debug)]
pub struct ExtractionResult {
    /// The HTML file that was processed.
    pub html_path: PathBuf,
    /// The attachments that were extracted.
    pub attachments: Vec<ExtractedAttachment>,
    /// Whether the HTML file was modified.
    pub html_modified: bool,
}

/// Extracts base64-encoded images from an HTML file and saves them to an attachments folder.
///
/// For an HTML file like `My Note -- abc123.html`, images are saved to
/// `My Note -- abc123-attachments/attachment-001.png`, etc.
///
/// The HTML file is updated in-place to reference the local files instead of data URLs.
///
/// # Arguments
///
/// * `html_path` - Path to the HTML file to process.
///
/// # Returns
///
/// Returns an `ExtractionResult` with details about what was extracted.
///
/// # Example
///
/// ```no_run
/// use apple_notes_exporter_rs::extract_attachments_from_html;
///
/// let result = extract_attachments_from_html("./exports/My Note -- abc123.html")
///     .expect("Failed to extract attachments");
///
/// println!("Extracted {} attachments", result.attachments.len());
/// ```
pub fn extract_attachments_from_html<P: AsRef<Path>>(html_path: P) -> Result<ExtractionResult> {
    let html_path = html_path.as_ref();
    let html_content = fs::read_to_string(html_path)?;

    let document = Html::parse_document(&html_content);
    let img_selector = Selector::parse("img").unwrap();

    let mut attachments = Vec::new();
    let mut modified_html = html_content.clone();
    let mut attachment_count = 0;

    // Determine the attachments folder name based on the HTML file stem
    let html_stem = html_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("note");
    let attachments_dir = html_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{html_stem}-attachments"));

    for element in document.select(&img_selector) {
        let Some(src) = element.value().attr("src") else {
            continue;
        };

        // Check if this is a data URL
        if !src.starts_with("data:image/") {
            continue;
        }

        // Parse the data URL: data:image/png;base64,iVBORw0...
        let Some((mime_part, base64_data)) = src.strip_prefix("data:").and_then(|s| s.split_once(",")) else {
            continue;
        };

        // Extract MIME type (e.g., "image/png;base64" -> "image/png")
        let mime_type = mime_part.split(';').next().unwrap_or("image/png");

        // Determine file extension from MIME type
        let extension = match mime_type {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "image/bmp" => "bmp",
            "image/tiff" => "tiff",
            _ => "bin",
        };

        // Decode base64 data
        let decoded_data = BASE64_STANDARD.decode(base64_data)?;

        // Create attachments directory if needed
        if !attachments_dir.exists() {
            fs::create_dir_all(&attachments_dir)?;
        }

        // Generate filename
        attachment_count += 1;
        let filename = format!("attachment-{attachment_count:03}.{extension}");
        let attachment_path = attachments_dir.join(&filename);

        // Write the attachment file
        fs::write(&attachment_path, &decoded_data)?;

        // Calculate relative path from HTML file to attachment
        let attachments_folder_name = attachments_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachments");
        let relative_path = format!("{attachments_folder_name}/{filename}");

        // Replace the data URL with the relative path in the HTML
        modified_html = modified_html.replace(src, &relative_path);

        attachments.push(ExtractedAttachment {
            path: attachment_path,
            original_data_url: src.to_string(),
            mime_type: mime_type.to_string(),
        });
    }

    // Write modified HTML if any attachments were extracted
    let html_modified = !attachments.is_empty();
    if html_modified {
        fs::write(html_path, &modified_html)?;
    }

    Ok(ExtractionResult {
        html_path: html_path.to_path_buf(),
        attachments,
        html_modified,
    })
}

/// Extracts attachments from all HTML files in a directory (recursively).
///
/// # Arguments
///
/// * `dir` - The directory to scan for HTML files.
///
/// # Returns
///
/// Returns a vector of `ExtractionResult` for each HTML file processed.
///
/// # Example
///
/// ```no_run
/// use apple_notes_exporter_rs::extract_attachments_from_directory;
///
/// let results = extract_attachments_from_directory("./exports")
///     .expect("Failed to extract attachments");
///
/// let total_attachments: usize = results.iter().map(|r| r.attachments.len()).sum();
/// println!("Extracted {total_attachments} attachments from {} files", results.len());
/// ```
pub fn extract_attachments_from_directory<P: AsRef<Path>>(dir: P) -> Result<Vec<ExtractionResult>> {
    let dir = dir.as_ref();
    let mut results = Vec::new();

    extract_attachments_recursive(dir, &mut results)?;

    Ok(results)
}

fn extract_attachments_recursive(dir: &Path, results: &mut Vec<ExtractionResult>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Skip attachment directories to avoid reprocessing
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|name| name.ends_with("-attachments"))
            {
                continue;
            }
            extract_attachments_recursive(&path, results)?;
        } else if path.extension().is_some_and(|ext| ext == "html") {
            let result = extract_attachments_from_html(&path)?;
            results.push(result);
        }
    }

    Ok(())
}
