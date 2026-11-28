use clap::{Parser, ValueEnum};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Mode {
    /// Safe, atomic CloudFormation Stack Refactoring API (supports fewer resource types)
    Refactor,
    /// Legacy import/export flow (supports more resource types but can orphan resources on failure)
    Import,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Name of the source stack
    #[arg(short, long)]
    pub(crate) source: Option<String>,

    /// Name of the target stack
    #[arg(short, long)]
    pub(crate) target: Option<String>,

    /// Logical ID of a resource from the source stack - optionally with a new ID for the target stack
    #[arg(short, long, value_name = "ID[:NEW_ID]")]
    pub(crate) resource: Option<Vec<String>>,

    /// Automatically confirm all prompts
    #[arg(short, long)]
    pub(crate) yes: bool,

    /// Operation mode for cross-stack moves
    #[arg(short, long, value_enum, default_value = "refactor")]
    pub(crate) mode: Mode,

    /// Output directory for exported templates (export mode only)
    #[arg(long, value_name = "PATH")]
    pub(crate) out_dir: Option<PathBuf>,

    /// Migration specification file with resource ID mappings (JSON format)
    #[arg(long, value_name = "PATH")]
    pub(crate) migration_spec: Option<PathBuf>,

    /// Source CloudFormation template file (alternative to fetching from AWS)
    #[arg(long, value_name = "PATH")]
    pub(crate) source_template: Option<PathBuf>,

    /// Target CloudFormation template file (alternative to fetching from AWS)
    #[arg(long, value_name = "PATH")]
    pub(crate) target_template: Option<PathBuf>,

    /// Export templates to disk without executing migration (dry-run mode)
    #[arg(long)]
    pub(crate) export: bool,
}

/// Checks flag combinations and that the given files can be read
pub(crate) fn validate_args(args: &Args) -> Result<(), Box<dyn Error>> {
    let conflicts = [
        (
            args.export && args.source_template.is_some(),
            "Cannot use --export with --source-template.\n\
             Export mode fetches templates from AWS and writes them to disk.\n\
             If you already have template files, you don't need export mode.",
        ),
        (
            args.export && args.target_template.is_some(),
            "Cannot use --export with --target-template.\n\
             Export mode fetches templates from AWS and writes them to disk.\n\
             If you already have template files, you don't need export mode.",
        ),
        (
            // --out-dir only makes sense with --export
            args.out_dir.is_some() && !args.export,
            "Cannot use --out-dir without --export.\n\
             The --out-dir flag specifies where to write exported templates.\n\
             Add --export to enable export mode, or remove --out-dir.",
        ),
        (
            // Migration spec can be used with both export and template input modes
            args.migration_spec.is_some() && args.resource.is_some(),
            "Cannot use --migration-spec with --resource.\n\
             The migration spec file defines resource mappings, so the --resource flag is not needed.",
        ),
    ];

    if let Some((_, message)) = conflicts.iter().find(|(conflict, _)| *conflict) {
        return Err((*message).into());
    }

    let files = [
        (&args.source_template, "Source template"),
        (&args.target_template, "Target template"),
        (&args.migration_spec, "Migration spec"),
    ];

    for (path, label) in files {
        if let Some(path) = path {
            ensure_readable_file(path, label)?;
        }
    }

    Ok(())
}

/// Fails unless `path` is an existing file that can be opened
fn ensure_readable_file(path: &Path, label: &str) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        return Err(format!("{} file does not exist: {}", label, path.display()).into());
    }
    if !path.is_file() {
        return Err(format!("{} path is not a file: {}", label, path.display()).into());
    }
    fs::File::open(path).map_err(|e| {
        format!(
            "Cannot read {} file: {} ({})",
            label.to_lowercase(),
            path.display(),
            e
        )
    })?;
    Ok(())
}
