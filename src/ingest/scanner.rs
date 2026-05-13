//! File scanner - walks directories and filters files

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Scanned file result
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub size_bytes: u64,
    pub content: String,
}

/// Skip patterns for directories
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".cache",
    "coverage",
    ".bytebell",
    "__pycache__",
    ".venv",
    "venv",
    ".idea",
    ".vscode",
    "target",
];

/// Skip patterns for files
const SKIP_FILES: &[&str] = &[
    ".DS_Store",
    "package-lock.json",
    "yarn.lock",
    "bun.lockb",
    "Cargo.lock",
    "Pipfile.lock",
    "poetry.lock",
    "Thumbs.db",
];

/// Binary file extensions
const BINARY_EXTENSIONS: &[&str] = &[
    // Images
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp", ".bmp", ".tiff", ".svg",
    // Video
    ".mp4", ".mov", ".avi", ".mkv", ".webm",
    // Audio
    ".mp3", ".wav", ".flac", ".ogg",
    // Archives
    ".zip", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".7z", ".rar",
    // Fonts
    ".ttf", ".otf", ".woff", ".woff2", ".eot",
    // Executables
    ".class", ".jar", ".exe", ".dll", ".so", ".dylib", ".wasm", ".bin",
    // Data
    ".png", ".jpg", ".pdf", ".doc", ".docx", ".xls", ".xlsx",
];

/// Maximum file size (1MB)
const MAX_FILE_SIZE: u64 = 1_048_576;

/// Scan a directory and yield all scannable files
pub async fn scan_directory(root: &Path) -> Result<Vec<ScannedFile>> {
    let mut files = Vec::new();
    scan_recursive(root, root, &mut files).await?;
    info!("Scanned {} files from {:?}", files.len(), root);
    Ok(files)
}

/// Recursively scan a directory (non-recursive version using VecDeque)
async fn scan_recursive(root: &Path, current: &Path, files: &mut Vec<ScannedFile>) -> Result<()> {
    let mut dirs_to_scan = vec![current.to_path_buf()];

    while let Some(dir) = dirs_to_scan.pop() {
        let mut entries = fs::read_dir(&dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // Skip certain directories
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                dirs_to_scan.push(path);
            } else if path.is_file() {
                // Skip certain files
                if SKIP_FILES.contains(&name.as_str()) {
                    continue;
                }

                // Check extension
                if let Some(ext) = path.extension() {
                    let ext_str = format!(".{}", ext.to_string_lossy().to_lowercase());
                    if BINARY_EXTENSIONS.contains(&ext_str.as_str()) {
                        continue;
                    }
                }

                // Check size
                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if metadata.len() > MAX_FILE_SIZE {
                    debug!("Skipping large file: {:?}", path);
                    continue;
                }

                // Check if binary
                if is_binary(&path).await? {
                    continue;
                }

                // Read content
                match fs::read_to_string(&path).await {
                    Ok(content) => {
                        let relative = path.strip_prefix(root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .replace('\\', "/"); // Normalize paths

                        files.push(ScannedFile {
                            relative_path: relative,
                            absolute_path: path,
                            size_bytes: metadata.len(),
                            content,
                        });
                    }
                    Err(e) => {
                        debug!("Could not read file {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a file is binary by looking for null bytes
async fn is_binary(path: &Path) -> Result<bool> {
    let content = fs::read(path).await?;
    let sample = &content[..content.len().min(4096)];

    for &byte in sample {
        if byte == 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Count scannable files without reading content
pub async fn count_files(root: &Path) -> Result<usize> {
    let files = scan_directory(root).await?;
    Ok(files.len())
}
