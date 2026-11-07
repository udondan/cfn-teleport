//! File names, timestamps and output directories

use crate::template::TemplateFormat;
use console::style;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Generates a timestamp string in YYYYMMDD-HHMMSS format (UTC-based from system time, sortable)
pub(crate) fn get_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    format_timestamp(duration.as_secs())
}

/// Formats seconds since the Unix epoch as YYYYMMDD-HHMMSS (UTC)
pub(crate) fn format_timestamp(secs: u64) -> String {
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    // Using Unix epoch start (1970-01-01) as base
    let mut year = 1970;
    let mut days_left = secs / 86400;

    while days_left >= days_in_year(year) {
        days_left -= days_in_year(year);
        year += 1;
    }

    let february = if is_leap_year(year) { 29 } else { 28 };
    let days_in_months = [31, february, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    let mut month = 1;
    let mut day = days_left + 1;

    for (i, &days_in_month) in days_in_months.iter().enumerate() {
        if day <= days_in_month {
            month = i + 1;
            break;
        }
        day -= days_in_month;
    }

    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    )
}

fn days_in_year(year: u64) -> u64 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

pub(crate) fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Generates a template filename based on stack name, operation, and format
///
/// # Examples
/// - `generate_filename("MyStack", "refactor", TemplateFormat::Yaml)` → "MyStack-refactor-20260228-143022.yaml"
/// - `generate_filename("StackA", "retain", TemplateFormat::Json)` → "StackA-retain-20260228-143022.json"
#[allow(dead_code)]
pub(crate) fn generate_filename(
    stack_name: &str,
    operation: &str,
    format: TemplateFormat,
) -> String {
    format!(
        "{}-{}-{}.{}",
        stack_name,
        operation,
        get_timestamp(),
        format.extension()
    )
}

/// Resolves file name collisions by appending .1, .2, .3, etc. before the extension
///
/// # Arguments
/// * `dir` - Directory where file will be written
/// * `filename` - Base filename (e.g., "MyStack-refactor-20260228-143022.yaml")
///
/// # Returns
/// PathBuf with collision-free filename. If base exists, tries .1, .2, etc. up to 100 variants.
pub(crate) fn resolve_file_collision(
    dir: &Path,
    filename: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let base_path = dir.join(filename);

    if !base_path.exists() {
        return Ok(base_path);
    }

    let path_obj = Path::new(filename);
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    let ext = path_obj
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();

    let numbered_path = (1..=100)
        .map(|i| dir.join(format!("{}.{}{}", stem, i, ext)))
        .find(|path| !path.exists())
        .ok_or("Too many file collisions (tried up to .100)")?;

    println!(
        "  {} File collision detected, writing to: {}",
        style("⚠").yellow(),
        numbered_path.display()
    );
    Ok(numbered_path)
}

/// Validates and resolves an output directory path.
///
/// This function ensures that the specified path:
/// - Exists or can be created
/// - Is a directory (not a file)
/// - Is writable
///
/// # Arguments
/// * `path` - The directory path to validate
/// * `base_dir` - Directory that relative paths are resolved against
///
/// # Returns
/// * `Ok(PathBuf)` - The validated absolute path
/// * `Err(Box<dyn Error>)` - If the path is invalid or cannot be used
pub(crate) fn validate_output_directory(
    path: &Path,
    base_dir: &Path,
) -> Result<PathBuf, Box<dyn Error>> {
    // Joining an absolute path replaces the base
    let abs_path = base_dir.join(path);

    if !abs_path.exists() {
        fs::create_dir_all(&abs_path).map_err(|e| {
            format!(
                "Failed to create output directory: {} ({})",
                abs_path.display(),
                e
            )
        })?;
        return Ok(abs_path);
    }

    if !abs_path.is_dir() {
        return Err(format!("Path exists but is not a directory: {}", abs_path.display()).into());
    }

    // Check if directory is writable by attempting to create a temporary file
    // Use a unique name to avoid overwriting any existing .cfn-teleport-write-test file
    let test_file = abs_path.join(format!(".cfn-teleport-write-test-{}", Uuid::new_v4()));
    fs::File::create(&test_file)
        .map_err(|e| format!("Directory is not writable: {} ({})", abs_path.display(), e))?;
    let _ = fs::remove_file(&test_file);

    Ok(abs_path)
}
