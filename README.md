# Apple Notes Exporter

[![crates.io](https://img.shields.io/crates/v/apple-notes-exporter-rs.svg)](https://crates.io/crates/apple-notes-exporter-rs)
[![Repository](https://img.shields.io/badge/repo-github-blue)](https://github.com/pRizz/apple-notes-exporter-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A command-line tool for recursively exporting Apple Notes folders to the file system via AppleScript. This Rust-based tool allows you to export entire folder hierarchies from Apple Notes, creating a mirrored directory structure with each note saved as an HTML file.

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
- List all available folders across all accounts
- Creates a mirrored directory tree structure
- Exports each note as an HTML file
- Breadth-first search (BFS) to find folders at any level
- Support for multiple Apple Notes accounts
- Simple command-line interface with subcommands

## Installation

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

## Usage

The tool provides two subcommands: `list` and `export`.

### List Available Folders

List all top-level folders across all Apple Notes accounts:

```bash
apple-notes-exporter list
```

### Export a Folder

Export a folder recursively to HTML files:

```bash
apple-notes-exporter export <FOLDER> <OUTPUT_DIR>
```

### Examples

List all available folders:

```bash
apple-notes-exporter list
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

## Running from Source

If you want to run the tool directly from the source code without installing:

```bash
# Clone the repository with submodules
git clone --recursive https://github.com/pRizz/apple-notes-exporter-rs.git
cd apple-notes-exporter-rs

# List folders
cargo run -- list

# Export a folder
cargo run -- export "My Notes" ./exports
```

If you already cloned without `--recursive`, initialize the submodules:

```bash
git submodule update --init --recursive
```

## How It Works

1. **Folder Search**: The tool uses breadth-first search (BFS) to find the specified folder at any level in your Apple Notes hierarchy (not just top-level folders).

2. **Export Process**: Once found, it recursively exports that folder and all its subfolders, creating a mirrored directory tree.

3. **Output Format**: Each note is exported as an HTML file, preserving the folder structure in the output directory.

4. **Account Handling**: By default, the folder search looks in all accounts. If a folder name exists in multiple accounts, you can specify the account using the `AccountName:FolderName` format.

## Requirements

### macOS Only

This tool is macOS-specific as it relies on AppleScript to interact with the Notes app.

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
│   └── main.rs              # Main application code
├── vendor/
│   └── apple-notes-exporter/
│       └── scripts/
│           └── export_notes.applescript  # AppleScript used for export
├── Cargo.toml
└── README.md
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

This tool uses the AppleScript from the [apple-notes-exporter](https://github.com/pRizz/apple-notes-exporter) project, which is included as a git submodule.
