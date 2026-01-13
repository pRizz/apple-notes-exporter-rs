# Apple Notes Exporter

[![crates.io](https://img.shields.io/crates/v/apple-notes-exporter-rs.svg)](https://crates.io/crates/apple-notes-exporter-rs)
[![docs.rs](https://docs.rs/apple-notes-exporter-rs/badge.svg)](https://docs.rs/apple-notes-exporter-rs)
[![Repository](https://img.shields.io/badge/repo-github-blue)](https://github.com/pRizz/apple-notes-exporter-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A library and CLI tool for recursively exporting Apple Notes folders to the file system via AppleScript. This Rust-based tool allows you to export entire folder hierarchies from Apple Notes, creating a mirrored directory structure with each note saved as an HTML file.

## Quick Install

Install from [crates.io](https://crates.io/crates/apple-notes-exporter-rs) using Cargo:

```bash
cargo install apple-notes-exporter-rs
```

Or install directly from GitHub:

```bash
cargo install --git https://github.com/pRizz/apple-notes-exporter-rs.git
```

## Features

- Recursive export of Apple Notes folders (including all subfolders and notes)
- **Automatic image extraction** - embedded base64 images are extracted to separate files
- List all available folders across all accounts
- Creates a mirrored directory tree structure
- Exports each note as an HTML file with images in a companion folder
- Breadth-first search (BFS) to find folders at any level
- Support for multiple Apple Notes accounts
- Simple command-line interface with subcommands
- Library API for programmatic access

## CLI Installation

### Install from crates.io (Recommended)

```bash
cargo install apple-notes-exporter-rs
```

### Install from GitHub

Install directly from the GitHub repository:

```bash
cargo install --git https://github.com/pRizz/apple-notes-exporter-rs.git
```

### Build from Source

```bash
git clone --recursive https://github.com/pRizz/apple-notes-exporter-rs.git
cd apple-notes-exporter-rs
cargo build --release
```

The binary will be available at `target/release/apple-notes-exporter`.

## CLI Usage

The tool provides three subcommands: `list` (or `ls`), `export`, and `extract-attachments`.

### List Available Folders

List all top-level folders across all Apple Notes accounts:

```bash
apple-notes-exporter list
# or
apple-notes-exporter ls
```

### Export a Folder

Export a folder recursively to HTML files. By default, embedded images are automatically extracted to separate files:

```bash
apple-notes-exporter export <FOLDER> <OUTPUT_DIR>
```

This creates a structure like:

```
exports/
├── My Notes/
│   ├── Note Title -- abc123.html
│   ├── Note Title -- abc123-attachments/
│   │   ├── attachment-001.jpg
│   │   └── attachment-002.png
│   └── Another Note -- def456.html
```

To skip image extraction (keep images as embedded base64):

```bash
apple-notes-exporter export "My Notes" ./exports --no-extract-attachments
```

### Extract Attachments from Existing Exports

If you have previously exported notes without extracting images, you can extract them later:

```bash
apple-notes-exporter extract-attachments ./exports
```

### Examples

List all available folders:

```bash
apple-notes-exporter list   # or: apple-notes-exporter ls
```

Export a folder (searches all accounts):

```bash
apple-notes-exporter export "My Notes" ./exports
```

Export a folder from a specific account (useful when folder names exist in multiple accounts):

```bash
apple-notes-exporter export "iCloud:My Notes" ./exports
apple-notes-exporter export "Google:Work Notes" ./exports
```

Export without extracting images:

```bash
apple-notes-exporter export "My Notes" ./exports --no-extract-attachments
```

## Running from Source

If you want to run the tool directly from the source code without installing:

```bash
# Clone the repository with submodules
git clone --recursive https://github.com/pRizz/apple-notes-exporter-rs.git
cd apple-notes-exporter-rs

# List folders
cargo run -- list   # or: cargo run -- ls

# Export a folder
cargo run -- export "My Notes" ./exports
```

If you already cloned without `--recursive`, initialize the submodules:

```bash
git submodule update --init --recursive
```

## Library Usage

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
apple-notes-exporter-rs = "1.0"
```

### Quick Start

```rust
use apple_notes_exporter_rs::{list_folders, export_folder, export_folder_from_account};

fn main() -> apple_notes_exporter_rs::Result<()> {
    // List all available folders (prints to stdout)
    list_folders()?;

    // Export a folder to a directory (searches all accounts)
    export_folder("My Notes", "./exports")?;

    // Export from a specific account (useful when folder names are duplicated)
    export_folder_from_account("iCloud", "Work", "./exports")?;
    export_folder_from_account("Google", "Work", "./google_exports")?;

    Ok(())
}
```

### Using the Exporter Struct

For more control, use the `Exporter` struct:

```rust
use apple_notes_exporter_rs::Exporter;

fn main() -> apple_notes_exporter_rs::Result<()> {
    // Create an exporter with the embedded AppleScript
    let exporter = Exporter::new();

    exporter.list_folders()?;

    // Export and extract images (recommended)
    let results = exporter.export_folder_with_attachments("My Notes", "./exports")?;
    println!("Extracted {} images", results.iter().map(|r| r.attachments.len()).sum::<usize>());

    // Or export without extracting images
    exporter.export_folder("Work", "./work_exports")?;

    Ok(())
}
```

### Using a Custom AppleScript

If you need to use a modified AppleScript:

```rust
use apple_notes_exporter_rs::Exporter;

fn main() -> apple_notes_exporter_rs::Result<()> {
    let exporter = Exporter::with_script_path("./custom_script.applescript")?;

    exporter.list_folders()?;
    exporter.export_folder("My Notes", "./exports")?;

    Ok(())
}
```

### Extracting Attachments from Existing Exports

You can also extract images from previously exported HTML files:

```rust
use apple_notes_exporter_rs::{extract_attachments_from_html, extract_attachments_from_directory};

fn main() -> apple_notes_exporter_rs::Result<()> {
    // Extract from a single HTML file
    let result = extract_attachments_from_html("./exports/My Note -- abc123.html")?;
    println!("Extracted {} images", result.attachments.len());

    // Extract from all HTML files in a directory (recursive)
    let results = extract_attachments_from_directory("./exports")?;
    let total: usize = results.iter().map(|r| r.attachments.len()).sum();
    println!("Extracted {total} images from {} files", results.len());

    Ok(())
}
```

### Error Handling

The library provides a custom `ExportError` type:

```rust
use apple_notes_exporter_rs::{export_folder, ExportError};

fn main() {
    match export_folder("My Notes", "./exports") {
        Ok(()) => println!("Export successful!"),
        Err(ExportError::ScriptNotFound(path)) => {
            eprintln!("Script not found: {}", path.display());
        }
        Err(ExportError::ScriptFailed(code)) => {
            eprintln!("AppleScript failed with exit code: {}", code);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

## How It Works

1. **Folder Search**: The tool uses breadth-first search (BFS) to find the specified folder at any level in your Apple Notes hierarchy (not just top-level folders).

2. **Export Process**: Once found, it recursively exports that folder and all its subfolders, creating a mirrored directory tree.

3. **Output Format**: Each note is exported as an HTML file, preserving the folder structure in the output directory.

4. **Image Extraction**: By default, embedded base64 images in the HTML are extracted to separate files in a companion `<note-name>-attachments/` folder. The HTML is updated to reference the local files. Supported formats: PNG, JPEG, GIF, WebP, SVG, BMP, TIFF.

5. **Account Handling**: By default, the folder search looks in all accounts. If a folder name exists in multiple accounts, you can specify the account using the `AccountName:FolderName` format.

## Requirements

### macOS Only

This tool is macOS-specific as it relies on AppleScript to interact with the Notes app. While the library can be compiled on any platform, running it on non-macOS systems will return an error at runtime.

### Automation Permissions

**Important**: This tool requires Automation permissions for the Notes app. You'll need to grant these permissions when you first run the tool:

1. Go to **System Settings** > **Privacy & Security** > **Automation**
2. Find the application that invoked the script (e.g., Terminal, iTerm, or Script Editor)
3. Enable permissions for the **Notes** app

If permissions are not granted, the export will fail.

## Project Structure

```
apple-notes-exporter-rs/
├── src/
│   ├── lib.rs               # Library: export API + attachment extraction
│   └── main.rs              # CLI application
├── vendor/
│   └── apple-notes-exporter/
│       └── scripts/
│           └── export_notes.applescript  # AppleScript used for export
├── Cargo.toml
└── README.md
```

## See Also

- [apple-notes-exporter-ts](https://github.com/pRizz/apple-notes-exporter-ts) - A TypeScript/Node.js version of this tool

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

This tool uses the AppleScript from the [apple-notes-exporter](https://github.com/pRizz/apple-notes-exporter) project, which is included as a git submodule.
