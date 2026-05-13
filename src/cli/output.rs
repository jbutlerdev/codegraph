//! Output formatting utilities

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Write;

/// Print a success message
pub fn success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// Print an error message
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print a warning message
pub fn warning(msg: &str) {
    eprintln!("{} {}", "⚠".yellow(), msg);
}

/// Print info message
pub fn info(msg: &str) {
    println!("  {}", msg);
}

/// Create a progress bar for file ingestion
pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb
}

/// Create an indeterminate spinner
pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_message(msg.to_string());
    pb
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// Format duration to human-readable string
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        format!("{:.1}m", secs / 60.0)
    } else {
        format!("{:.1}h", secs / 3600.0)
    }
}

/// Print a table row
pub fn print_table_row(cols: &[&str], widths: &[usize]) {
    let mut row = String::new();
    for (col, width) in cols.iter().zip(widths.iter()) {
        write!(row, " │ {:<width$}", col, width = *width).unwrap();
    }
    row.push_str(" │");
    println!("{}", row);
}

/// Print table header
pub fn print_table_header(cols: &[&str], widths: &[usize]) {
    // Top border
    let mut border = String::new();
    border.push('┌');
    for width in widths {
        border.push_str(&"─".repeat(*width + 2));
        border.push('┬');
    }
    border.pop();
    border.push('┐');
    println!("{}", border);

    // Header
    print_table_row(cols, widths);

    // Separator
    let mut sep = String::new();
    sep.push('├');
    for width in widths {
        sep.push_str(&"─".repeat(*width + 2));
        sep.push('┼');
    }
    sep.pop();
    sep.push('┤');
    println!("{}", sep);
}

/// Print table footer
pub fn print_table_footer(widths: &[usize]) {
    let mut border = String::new();
    border.push('└');
    for width in widths {
        border.push_str(&"─".repeat(*width + 2));
        border.push('┴');
    }
    border.pop();
    border.push('┘');
    println!("{}", border);
}
