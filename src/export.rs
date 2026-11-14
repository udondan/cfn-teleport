//! Writing templates and debugging information to disk

use crate::files::{get_timestamp, resolve_file_collision, validate_output_directory};
use crate::template::{write_template_to_file, Template};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Exports CloudFormation templates to disk with automatic collision handling.
///
/// This function writes one or more templates to the specified output directory,
/// automatically handling filename collisions by appending .1, .2, etc.
///
/// # Arguments
/// * `templates` - Vector of (template, suffix) tuples to export
/// * `output_dir` - Directory to write templates to (will be validated/created)
/// * `base_dir` - Directory used when `output_dir` is missing or relative
/// * `stack_name` - Stack name for filename generation
/// * `operation` - Operation type ("refactor", "import", etc.) for filename generation
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - Paths to all written template files
/// * `Err(Box<dyn Error>)` - If any file operation fails
pub(crate) fn export_templates(
    templates: &[(Template, String)],
    output_dir: Option<&PathBuf>,
    base_dir: &Path,
    stack_name: &str,
    operation: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let dir = match output_dir {
        Some(d) => validate_output_directory(d, base_dir)?,
        None => base_dir.to_path_buf(),
    };

    let timestamp = get_timestamp();
    let mut written_paths = Vec::new();

    for (template, suffix) in templates {
        let filename = format!(
            "{}-{}-{}-{}.{}",
            stack_name,
            operation,
            suffix,
            timestamp,
            template.format.extension()
        );
        let final_path = resolve_file_collision(&dir, &filename)?;
        write_template_to_file(template, &final_path)?;
        written_paths.push(final_path);
    }

    Ok(written_paths)
}

/// Tells the user where the exported templates are
pub(crate) fn print_export_result(paths: &[PathBuf]) {
    println!("\n✅ Templates exported successfully:");
    for path in paths {
        println!("  📄 {}", path.display());
    }
    println!("\nNo AWS resources were modified (export mode).");
}

/// What was attempted when an operation failed
pub(crate) struct ErrorContext<'a> {
    pub(crate) error: &'a str,
    pub(crate) source_stack: &'a str,
    /// None for same-stack operations
    pub(crate) target_stack: Option<&'a str>,
    /// Operation type, e.g. "refactor-cross-stack" or "import"
    pub(crate) operation: &'a str,
    /// Resource IDs being migrated (old -> new)
    pub(crate) resources: &'a HashMap<String, String>,
}

/// Writes an error context file with debugging information
pub(crate) fn write_error_context(
    path: &Path,
    timestamp: &str,
    context: &ErrorContext,
) -> Result<(), Box<dyn Error>> {
    let mut text = format!(
        "Error: {}\nTimestamp: {}\nOperation: {}\nSource Stack: {}\n",
        context.error, timestamp, context.operation, context.source_stack
    );

    if let Some(target) = context.target_stack {
        text.push_str(&format!("Target Stack: {}\n", target));
    }

    text.push_str("Resources:\n");
    for (old_id, new_id) in context.resources {
        if old_id == new_id {
            text.push_str(&format!("  - {} (no rename)\n", old_id));
        } else {
            text.push_str(&format!("  - {} -> {}\n", old_id, new_id));
        }
    }

    text.push_str(&format!("\nFull Error Details:\n{}\n", context.error));

    fs::write(path, text)
        .map_err(|e| format!("Failed to write error context to {}: {}", path.display(), e))?;

    Ok(())
}

/// Saves the templates and an error context file to `work_dir` after a failed validation.
/// `context_label` names the context file: `<stack>-error-<context_label>-context-<timestamp>.txt`
pub(crate) fn save_debug_artifacts(
    work_dir: &Path,
    context_label: &str,
    templates: &[(Template, String)],
    context: &ErrorContext,
) {
    let (noun, heading) = if templates.len() == 1 {
        ("template", "Template")
    } else {
        ("templates", "Templates")
    };

    eprintln!(
        "\n⚠️  Template validation failed. Saving {} for debugging...",
        noun
    );

    let operation = format!("error-{}", context.operation);
    if let Ok(paths) = export_templates(templates, None, work_dir, context.source_stack, &operation)
    {
        eprintln!("📄 {} saved to:", heading);
        for path in &paths {
            eprintln!("   {}", path.display());
        }
    }

    let timestamp = get_timestamp();
    let context_path = work_dir.join(format!(
        "{}-error-{}-context-{}.txt",
        context.source_stack, context_label, timestamp
    ));

    if write_error_context(&context_path, &timestamp, context).is_ok() {
        eprintln!("📝 Error context saved to: {}", context_path.display());
    }
}
