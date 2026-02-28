use aws_config::BehaviorVersion;
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use clap::{Parser, ValueEnum};
use console::style;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
mod cfn_yaml;
mod reference_updater;
mod spinner;
mod supported_resource_types;

const DEMO: bool = false;

// Dependency marker emojis
const EMOJI_INCOMING: &str = "➡️";
const EMOJI_OUTGOING: &str = "⬅️";
const EMOJI_BIDIRECTIONAL: &str = "↔️";
const EMOJI_OUTPUTS: &str = "⬆️";
const EMOJI_PARAMETERS: &str = "⬇️";
const EMOJI_BOTH_STACK_INTERFACE: &str = "↕️";

/// Holds dependency information for a resource
struct ResourceDependencyInfo {
    has_incoming_deps: bool,     // Other resources reference this one
    has_outgoing_deps: bool,     // This resource references others
    referenced_by_outputs: bool, // Outputs reference this one
    depends_on_parameters: bool, // This resource references stack parameters
}

/// CloudFormation template format
#[derive(Debug, Clone, Copy, PartialEq)]
enum TemplateFormat {
    Json,
    Yaml,
}

/// Wrapper for CloudFormation templates that preserves their original format
#[derive(Clone, Debug)]
struct Template {
    content: serde_json::Value,
    format: TemplateFormat,
}

impl Template {
    fn new(content: serde_json::Value, format: TemplateFormat) -> Self {
        Self { content, format }
    }

    /// Serialize the template to string in its original format.
    /// YAML templates are serialized as YAML, JSON templates as JSON.
    fn to_string(&self) -> Result<String, Box<dyn Error>> {
        match self.format {
            TemplateFormat::Json => Ok(serde_json::to_string(&self.content)?),
            TemplateFormat::Yaml => Ok(serde_yml::to_string(&self.content)?),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    /// Safe, atomic CloudFormation Stack Refactoring API (supports fewer resource types)
    Refactor,
    /// Legacy import/export flow (supports more resource types but can orphan resources on failure)
    Import,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the source stack
    #[arg(short, long)]
    source: Option<String>,

    /// Name of the target stack
    #[arg(short, long)]
    target: Option<String>,

    /// Logical ID of a resource from the source stack - optionally with a new ID for the target stack
    #[arg(short, long, value_name = "ID[:NEW_ID]")]
    resource: Option<Vec<String>>,

    /// Automatically confirm all prompts
    #[arg(short, long)]
    yes: bool,

    /// Operation mode for cross-stack moves
    #[arg(short, long, value_enum, default_value = "refactor")]
    mode: Mode,

    /// Output directory for exported templates (export mode only)
    #[arg(long, value_name = "PATH")]
    out_dir: Option<PathBuf>,

    /// Migration specification file with resource ID mappings (JSON format)
    #[arg(long, value_name = "PATH")]
    migration_spec: Option<PathBuf>,

    /// Source CloudFormation template file (alternative to fetching from AWS)
    #[arg(long, value_name = "PATH")]
    source_template: Option<PathBuf>,

    /// Target CloudFormation template file (alternative to fetching from AWS)
    #[arg(long, value_name = "PATH")]
    target_template: Option<PathBuf>,

    /// Export templates to disk without executing migration (dry-run mode)
    #[arg(long)]
    export: bool,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // Print error with proper formatting (interprets escape sequences)
        eprintln!("\n{}\n", err);
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Validate CLI argument combinations
    if args.export && args.source_template.is_some() {
        return Err("Cannot use --export with --source-template.\n\
             Export mode fetches templates from AWS and writes them to disk.\n\
             If you already have template files, you don't need export mode."
            .into());
    }

    if args.export && args.target_template.is_some() {
        return Err("Cannot use --export with --target-template.\n\
             Export mode fetches templates from AWS and writes them to disk.\n\
             If you already have template files, you don't need export mode."
            .into());
    }

    // Migration spec can be used with both export and template input modes
    if args.migration_spec.is_some() && args.resource.is_some() {
        return Err(
            "Cannot use --migration-spec with --resource.\n\
             The migration spec file defines resource mappings, so the --resource flag is not needed."
                .into(),
        );
    }

    // Validate template file paths exist and are readable
    if let Some(source_path) = &args.source_template {
        if !source_path.exists() {
            return Err(format!(
                "Source template file does not exist: {}",
                source_path.display()
            )
            .into());
        }
        if !source_path.is_file() {
            return Err(format!(
                "Source template path is not a file: {}",
                source_path.display()
            )
            .into());
        }
        // Test readability by attempting to open
        fs::File::open(source_path).map_err(|e| {
            format!(
                "Cannot read source template file: {} ({})",
                source_path.display(),
                e
            )
        })?;
    }

    if let Some(target_path) = &args.target_template {
        if !target_path.exists() {
            return Err(format!(
                "Target template file does not exist: {}",
                target_path.display()
            )
            .into());
        }
        if !target_path.is_file() {
            return Err(format!(
                "Target template path is not a file: {}",
                target_path.display()
            )
            .into());
        }
        // Test readability
        fs::File::open(target_path).map_err(|e| {
            format!(
                "Cannot read target template file: {} ({})",
                target_path.display(),
                e
            )
        })?;
    }

    // Validate migration spec file if provided
    if let Some(spec_path) = &args.migration_spec {
        if !spec_path.exists() {
            return Err(format!(
                "Migration spec file does not exist: {}",
                spec_path.display()
            )
            .into());
        }
        if !spec_path.is_file() {
            return Err(
                format!("Migration spec path is not a file: {}", spec_path.display()).into(),
            );
        }
    }

    let config = aws_config::load_defaults(BehaviorVersion::v2026_01_12()).await;
    let client = cloudformation::Client::new(&config);

    // Try to get stacks and handle credential errors specifically
    let stacks = match get_stacks(&client).await {
        Ok(stacks) => stacks,
        Err(err) => {
            // Check error source chain for credential-related errors
            let mut is_credentials_error = false;
            let mut source = err.source();

            while let Some(error) = source {
                let error_str = error.to_string();
                if error_str.contains("CredentialsNotLoaded")
                    || error_str.contains("no providers in chain provided credentials")
                {
                    is_credentials_error = true;
                    break;
                }
                source = error.source();
            }

            if is_credentials_error {
                return Err("AWS credentials not found.\n\nPlease ensure you're authenticated with AWS using one of the following methods:\n  • AWS CLI: Run 'aws configure'\n  • Environment variables: Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY\n  • IAM role (if running on EC2/ECS/Lambda)\n\nFor more information, visit:\n  https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html".into());
            } else {
                // Handle other AWS errors cleanly
                let message = err.message().unwrap_or("An AWS error occurred");

                if let Some(code) = err.code() {
                    return Err(format!("AWS Error ({}): {}", code, message).into());
                } else {
                    return Err(message.into());
                }
            }
        }
    };

    let stack_names: Vec<&str> = stacks
        .iter()
        .map(|s| s.stack_name().unwrap_or_default())
        .collect();

    let source_stack = args.source.unwrap_or_else(|| {
        select_stack("Select source stack", &stack_names)
            .unwrap()
            .to_string()
    });

    let resources = get_resources(&client, &source_stack).await?;

    if resources.is_empty() {
        return Err(format!("No resources found in stack '{}'", source_stack).into());
    }

    let target_stack = args.target.unwrap_or_else(|| {
        select_stack("Select target stack", &stack_names)
            .unwrap()
            .to_string()
    });

    let resource_refs = &resources.iter().collect::<Vec<_>>();

    // Get source template for dependency analysis
    let source_template =
        get_template(&client, &source_stack, args.source_template.as_ref()).await?;

    // Determine if this is a cross-stack move or same-stack rename
    let is_cross_stack = source_stack != target_stack;

    // Fetch target template early for cross-stack parameter validation
    let target_template = if is_cross_stack {
        Some(get_template(&client, &target_stack, args.target_template.as_ref()).await?)
    } else {
        None
    };

    // If migration spec file is provided, use it to determine resources and mappings
    // This overrides the --resource CLI flag
    let migration_spec_mappings = if let Some(spec_path) = &args.migration_spec {
        let mappings = parse_migration_spec(spec_path)?;
        validate_migration_spec_resources(&mappings, &source_template)?;
        Some(mappings)
    } else {
        None
    };

    let selected_resources = match migration_spec_mappings.as_ref() {
        // If migration spec provided, use those resource IDs
        Some(mappings) => {
            let source_ids: Vec<String> = mappings.keys().cloned().collect();
            filter_resources(resource_refs, &source_ids).await?
        }
        // Otherwise, use CLI args or interactive selection
        None => match args.resource.clone() {
            Some(resource) => {
                let source_ids = resource
                    .iter()
                    .map(|r| split_ids(r.clone()).0)
                    .collect::<Vec<_>>();

                let non_existing_ids: Vec<String> = source_ids
                    .iter()
                    .filter(|id| {
                        !resource_refs
                            .iter()
                            .any(|r| r.logical_resource_id().unwrap_or_default() == **id)
                    })
                    .map(|id| id.to_string())
                    .collect();

                if !non_existing_ids.is_empty() {
                    return Err(format!(
                        "ERROR: The following resources do not exist on stack '{}':\n - {}",
                        source_stack,
                        non_existing_ids.to_owned().join("\n - "),
                    )
                    .into());
                }
                filter_resources(resource_refs, &source_ids).await?
            }
            None => {
                select_resources(
                    "Select resources to copy",
                    resource_refs,
                    &source_template.content,
                    target_template.as_ref().map(|t| &t.content),
                    is_cross_stack,
                )
                .await?
            }
        },
    };

    if selected_resources.is_empty() {
        return Err("No resources have been selected".into());
    }

    let mut new_logical_ids_map = HashMap::new();
    //let mut resource_has_been_renamed = false;

    // If migration spec provided, use those mappings directly
    if let Some(mappings) = migration_spec_mappings {
        new_logical_ids_map = mappings;
    } else {
        // Otherwise, build mappings from CLI args or interactive prompts
        match args.resource.clone() {
            None => {
                for resource in selected_resources.clone() {
                    let old_logical_id = resource
                        .logical_resource_id()
                        .unwrap_or_default()
                        .to_owned();

                    let mut new_logical_id: String = Input::new()
                        .with_prompt(format!(
                            "Optionally provide a new logical ID for resource '{}'",
                            old_logical_id
                        ))
                        .default(old_logical_id.clone())
                        .interact_text()?;

                    if new_logical_id.is_empty() {
                        new_logical_id = resource
                            .logical_resource_id()
                            .unwrap_or_default()
                            .to_string();
                    }

                    new_logical_ids_map.insert(old_logical_id, new_logical_id);
                }
                //            println!()
            }
            Some(resources) => {
                for resource in resources {
                    let ids = split_ids(resource.clone());
                    let source_id = ids.0.clone();
                    let target_id = ids.1.clone();
                    new_logical_ids_map.insert(source_id, target_id);
                }
            }
        };
    }

    // Reuse the template we already fetched at line 144
    let template_source = source_template;

    if source_stack == target_stack {
        // Same-stack operation: must be renaming, not just moving
        let mut has_any_rename = false;
        for (old_id, new_id) in &new_logical_ids_map {
            if old_id != new_id {
                has_any_rename = true;
                break;
            }
        }

        if !has_any_rename {
            return Err(
                "Source and target stack are the same, but no resources are being renamed. \
                 Same-stack operations require renaming at least one resource."
                    .into(),
            );
        }

        // Check for resources that aren't being renamed
        let mut duplicate_ids = Vec::new();
        for (old_id, new_id) in &new_logical_ids_map {
            if old_id == new_id {
                duplicate_ids.push(old_id);
            }
        }

        if !duplicate_ids.is_empty() {
            let error_message = format!(
                "Unable to proceed, because you said you want to rename resources in stack {} but did not provide new logical IDs for: {}",
                source_stack,
                duplicate_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
            );
            return Err(error_message.into());
        }

        // Check for name collisions: new name already exists in stack
        let existing_resources = if let Some(resources) = template_source.content.get("Resources") {
            if let Some(obj) = resources.as_object() {
                obj.keys()
                    .cloned()
                    .collect::<std::collections::HashSet<_>>()
            } else {
                std::collections::HashSet::new()
            }
        } else {
            std::collections::HashSet::new()
        };

        let mut collisions = Vec::new();
        for (old_id, new_id) in &new_logical_ids_map {
            // Check if the new name exists and is NOT the old name being renamed
            if existing_resources.contains(new_id) && old_id != new_id {
                collisions.push(format!(
                    "{} -> {} (resource '{}' already exists)",
                    old_id, new_id, new_id
                ));
            }
        }

        if !collisions.is_empty() {
            let error_message = format!(
                "Cannot rename resources: target name(s) already exist in stack {}:\n  {}",
                source_stack,
                collisions.join("\n  ")
            );
            return Err(error_message.into());
        }

        println!(
            "The following resources in stack {} will be renamed:",
            source_stack
        );
    } else {
        println!(
            "The following resources will be moved from stack {} to {}:",
            source_stack, target_stack
        );
    }

    for resource in
        format_resources(&selected_resources, Some(new_logical_ids_map.clone()), None).await?
    {
        println!("  {}", resource);
    }

    if !args.yes {
        user_confirm()?;
    }

    let template_source_str = template_source.to_string()?;

    // Validate that resources being moved don't have dangling references
    // (i.e., resources staying in source stack that reference moving resources)
    // Only validate for cross-stack moves, not same-stack renames
    if source_stack != target_stack {
        validate_move_references(&template_source.content, &new_logical_ids_map)?;
    }

    // Same-stack rename: Use CloudFormation Stack Refactoring API
    if source_stack == target_stack {
        // If export mode, write template and return early
        if args.export {
            let templates = vec![(template_source.clone(), "refactored".to_string())];
            let paths = export_templates(
                &templates,
                args.out_dir.as_ref(),
                &source_stack,
                "refactor-same-stack",
            )?;

            println!("\n✅ Templates exported successfully:");
            for path in paths {
                println!("  📄 {}", path.display());
            }
            println!("\nNo AWS resources were modified (export mode).");
            return Ok(());
        }

        return refactor_stack_resources(
            &client,
            &source_stack,
            template_source,
            new_logical_ids_map,
        )
        .await;
    }

    // Cross-stack move: Use refactor or import based on --mode
    if matches!(args.mode, Mode::Refactor) {
        // Use CloudFormation Stack Refactoring API (safer, atomic, but fewer supported resource types)
        // Reuse the target template we fetched earlier for cross-stack moves
        let template_target = if let Some(tmpl) = target_template {
            tmpl
        } else {
            get_template(&client, &target_stack, args.target_template.as_ref()).await?
        };

        // If export mode, generate templates and write them to disk
        if args.export {
            // Generate the same templates that would be used in refactor mode
            let resource_ids: Vec<String> = new_logical_ids_map.keys().cloned().collect();

            let source_without_resources =
                remove_resources(template_source.content.clone(), resource_ids.clone());

            let (_, target_with_resources) = add_resources(
                template_target.content.clone(),
                template_source.content.clone(),
                new_logical_ids_map.clone(),
            );

            let source_final = reference_updater::update_template_references(
                source_without_resources,
                &new_logical_ids_map,
            );
            let target_final = reference_updater::update_template_references(
                target_with_resources,
                &new_logical_ids_map,
            );

            let templates = vec![
                (
                    Template::new(source_final, template_source.format),
                    format!("source-{}", source_stack),
                ),
                (
                    Template::new(target_final, template_target.format),
                    format!("target-{}", target_stack),
                ),
            ];

            let paths = export_templates(
                &templates,
                args.out_dir.as_ref(),
                &source_stack,
                "refactor-cross-stack",
            )?;

            println!("\n✅ Templates exported successfully:");
            for path in paths {
                println!("  📄 {}", path.display());
            }
            println!("\nNo AWS resources were modified (export mode).");
            return Ok(());
        }

        return refactor_stack_resources_cross_stack(
            &client,
            &source_stack,
            &target_stack,
            template_source,
            template_target,
            new_logical_ids_map,
        )
        .await;
    }

    // Legacy import/export flow (mode == Mode::Import)
    // IMPORTANT: Import mode does NOT copy parameters from source to target stack.
    // Resources that depend on parameters will fail to import because the parameter
    // reference will be invalid in the target stack, causing "No updates" errors
    // and leaving the resource in a broken state (deleted from source, not in target).
    // We MUST block any resources with parameter dependencies in import mode.
    let mut blocked_resources_with_params = Vec::new();
    for old_id in new_logical_ids_map.keys() {
        if let Some(resource) = selected_resources
            .iter()
            .find(|r| r.logical_resource_id().unwrap_or_default() == old_id)
        {
            // Check if this resource depends on parameters
            let all_references = reference_updater::find_all_references(&template_source.content);
            let resource_references = all_references
                .get(old_id.as_str())
                .cloned()
                .unwrap_or_default();

            // Get parameter names from source template
            let parameter_names: std::collections::HashSet<String> = template_source
                .content
                .get("Parameters")
                .and_then(|params| params.as_object())
                .map(|params_obj| {
                    params_obj
                        .keys()
                        .map(|k| k.to_string())
                        .collect::<std::collections::HashSet<String>>()
                })
                .unwrap_or_default();

            // Check if resource references any parameters
            let depends_on_params: Vec<String> = resource_references
                .iter()
                .filter(|ref_name| parameter_names.contains(*ref_name))
                .map(|s| s.to_string())
                .collect();

            if !depends_on_params.is_empty() {
                blocked_resources_with_params.push((
                    old_id.clone(),
                    resource.resource_type().unwrap_or_default().to_string(),
                    depends_on_params,
                ));
            }
        }
    }

    if !blocked_resources_with_params.is_empty() {
        let mut error_msg =
            String::from("Cannot use import mode for resources that depend on stack parameters:\n");
        for (id, typ, params) in &blocked_resources_with_params {
            error_msg.push_str(&format!(
                "  - {} ({}) - depends on parameters: {}\n",
                id,
                typ,
                params.join(", ")
            ));
        }
        error_msg.push_str(
            "\nImport mode does NOT copy parameters between stacks, which causes resources to be\n",
        );
        error_msg.push_str("deleted from source stack but fail to import to target stack, leaving them orphaned.\n\n");
        error_msg.push_str(
            "Use --mode refactor instead, or remove parameter dependencies from these resources.",
        );
        return Err(error_msg.into());
    }

    let resource_ids_to_remove: Vec<_> = new_logical_ids_map.keys().cloned().collect();

    let template_retained = retain_resources(
        template_source.content.clone(),
        resource_ids_to_remove.clone(),
    );
    let template_retained_str =
        Template::new(template_retained.clone(), template_source.format).to_string()?;

    let template_removed = remove_resources(
        template_source.content.clone(),
        resource_ids_to_remove.clone(),
    );

    // Fetch target template if not already available to get its format
    let target_template_actual = if let Some(tmpl) = target_template {
        tmpl
    } else {
        get_template(&client, &target_stack, args.target_template.as_ref()).await?
    };

    let (template_target_with_deletion_policy, template_target_final) = add_resources(
        target_template_actual.content.clone(),
        template_source.content.clone(),
        new_logical_ids_map.clone(),
    );

    // Update all resource references in the target templates
    let template_target_with_deletion_policy = reference_updater::update_template_references(
        template_target_with_deletion_policy,
        &new_logical_ids_map,
    );
    let template_target =
        reference_updater::update_template_references(template_target_final, &new_logical_ids_map);

    for template in [
        template_retained.clone(),
        template_removed.clone(),
        template_target.clone(),
        template_target_with_deletion_policy.clone(),
    ] {
        let result = validate_template(&client, template).await;
        if result.is_err() {
            let error_msg = result.err().unwrap();

            // Save templates for debugging
            eprintln!("\n⚠️  Template validation failed. Saving templates for debugging...");

            let templates_to_save = vec![
                (
                    Template::new(template_retained.clone(), template_source.format),
                    "retained".to_string(),
                ),
                (
                    Template::new(template_removed.clone(), template_source.format),
                    "removed".to_string(),
                ),
                (
                    Template::new(
                        template_target_with_deletion_policy.clone(),
                        target_template_actual.format,
                    ),
                    "target-with-policy".to_string(),
                ),
                (
                    Template::new(template_target.clone(), target_template_actual.format),
                    "target-final".to_string(),
                ),
            ];

            if let Ok(paths) = export_templates(
                &templates_to_save,
                None, // Use current directory
                &source_stack,
                "error-import",
            ) {
                eprintln!("📄 Templates saved to:");
                for path in &paths {
                    eprintln!("   {}", path.display());
                }
            }

            // Save error context
            let timestamp = get_timestamp();
            let context_filename =
                format!("{}-error-import-context-{}.txt", source_stack, timestamp);
            let context_path = std::env::current_dir()?.join(context_filename);

            if write_error_context(
                &context_path,
                &error_msg.to_string(),
                &source_stack,
                Some(&target_stack),
                "import",
                &new_logical_ids_map,
            )
            .is_ok()
            {
                eprintln!("📝 Error context saved to: {}", context_path.display());
            }

            return Err(format!(
                "Unable to proceed, because the template is invalid: {}\n\
                 Templates and error context have been saved for debugging.",
                error_msg
            )
            .into());
        }
    }

    // If export mode, write all 4 templates and return early
    if args.export {
        let templates = vec![
            (
                Template::new(template_retained.clone(), template_source.format),
                format!("source-{}-retained", source_stack),
            ),
            (
                Template::new(template_removed.clone(), template_source.format),
                format!("source-{}-removed", source_stack),
            ),
            (
                Template::new(
                    template_target_with_deletion_policy.clone(),
                    target_template_actual.format,
                ),
                format!("target-{}-with-deletion-policy", target_stack),
            ),
            (
                Template::new(template_target.clone(), target_template_actual.format),
                format!("target-{}-final", target_stack),
            ),
        ];

        let paths = export_templates(&templates, args.out_dir.as_ref(), &source_stack, "import")?;

        println!("\n✅ Templates exported successfully:");
        for path in paths {
            println!("  📄 {}", path.display());
        }
        println!("\nNo AWS resources were modified (export mode).");
        return Ok(());
    }

    let spinner = spinner::Spin::new(
        format!(
            "Removing {} resources from stack {}",
            resource_ids_to_remove.len(),
            source_stack
        )
        .as_str(),
    );

    if template_source_str != template_retained_str {
        update_stack(
            &client,
            &source_stack,
            template_retained,
            template_source.format,
        )
        .await?;
        wait_for_stack_update_completion(&client, &source_stack, None).await?;
    }

    update_stack(
        &client,
        &source_stack,
        template_removed,
        template_source.format,
    )
    .await?;
    wait_for_stack_update_completion(&client, &source_stack, Some(spinner)).await?;

    let spinner = spinner::Spin::new(&format!(
        "Importing {} resources into stack {}",
        resource_ids_to_remove.len(),
        target_stack,
    ));

    let changeset_name = create_changeset(
        &client,
        &target_stack,
        template_target_with_deletion_policy,
        target_template_actual.format,
        selected_resources,
        new_logical_ids_map,
    )
    .await?;

    wait_for_changeset_created(&client, &target_stack, &changeset_name).await?;
    execute_changeset(&client, &target_stack, &changeset_name).await?;
    wait_for_stack_update_completion(&client, &target_stack, None).await?;

    update_stack(
        &client,
        &target_stack,
        template_target,
        target_template_actual.format,
    )
    .await?;
    wait_for_stack_update_completion(&client, &target_stack, Some(spinner)).await?;

    Ok(())
}

fn split_ids(id: String) -> (String, String) {
    if id.contains(&":".to_string()) {
        let parts: Vec<String> = id.split(':').map(String::from).collect();
        (parts[0].clone(), parts[1].clone())
    } else {
        (id.clone(), id)
    }
}

// ============================================================================
// Template File I/O Utilities
// ============================================================================

/// Generates a timestamp string in YYYYMMDD-HHMMSS format (local time, sortable)
fn get_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Calculate date/time components from Unix timestamp
    // This is a simplified calculation for local time approximation
    let total_days = secs / 86400;
    let remaining_secs = secs % 86400;

    // Calculate hours, minutes, seconds
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Approximate year/month/day (simplified - doesn't account for leap years perfectly)
    // Using Unix epoch start (1970-01-01) as base
    let mut year = 1970;
    let mut days_left = total_days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_left >= days_in_year {
            days_left -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    // Calculate month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

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

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Generates a template filename based on stack name, operation, and format
///
/// # Examples
/// - `generate_filename("MyStack", "refactor", TemplateFormat::Yaml)` → "MyStack-refactor-20260228-143022.yaml"
/// - `generate_filename("StackA", "retain", TemplateFormat::Json)` → "StackA-retain-20260228-143022.json"
#[allow(dead_code)]
fn generate_filename(stack_name: &str, operation: &str, format: TemplateFormat) -> String {
    let timestamp = get_timestamp();
    let extension = match format {
        TemplateFormat::Json => "json",
        TemplateFormat::Yaml => "yaml",
    };
    format!("{}-{}-{}.{}", stack_name, operation, timestamp, extension)
}

/// Resolves file name collisions by appending .1, .2, .3, etc. before the extension
///
/// # Arguments
/// * `dir` - Directory where file will be written
/// * `filename` - Base filename (e.g., "MyStack-refactor-20260228-143022.yaml")
///
/// # Returns
/// PathBuf with collision-free filename. If base exists, tries .1, .2, etc. up to 100 variants.
fn resolve_file_collision(dir: &Path, filename: &str) -> Result<PathBuf, Box<dyn Error>> {
    let base_path = dir.join(filename);

    // If file doesn't exist, use base name
    if !base_path.exists() {
        return Ok(base_path);
    }

    // Extract base name and extension
    let path_obj = Path::new(filename);
    let stem = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    let ext = path_obj.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Try numbered variants: .1, .2, .3, etc.
    for i in 1..=100 {
        let numbered_filename = if ext.is_empty() {
            format!("{}.{}", stem, i)
        } else {
            format!("{}.{}.{}", stem, i, ext)
        };
        let numbered_path = dir.join(numbered_filename);

        if !numbered_path.exists() {
            println!(
                "  {} File collision detected, writing to: {}",
                style("⚠").yellow(),
                numbered_path.display()
            );
            return Ok(numbered_path);
        }
    }

    Err("Too many file collisions (tried up to .100)".into())
}

/// Writes a template to disk in its original format (JSON or YAML)
///
/// # Arguments
/// * `template` - Template to write
/// * `path` - File path where template will be written
///
/// # Returns
/// Ok(()) on success, error if file cannot be written
fn write_template_to_file(template: &Template, path: &Path) -> Result<(), Box<dyn Error>> {
    let template_str = template
        .to_string()
        .map_err(|e| format!("Failed to serialize template: {}", e))?;

    let mut file = fs::File::create(path)
        .map_err(|e| format!("Failed to create file {}: {}", path.display(), e))?;

    file.write_all(template_str.as_bytes())
        .map_err(|e| format!("Failed to write template to {}: {}", path.display(), e))?;

    Ok(())
}

/// Reads a template from a file, detecting format automatically (JSON or YAML)
///
/// # Arguments
/// * `path` - Path to template file
///
/// # Returns
/// Template struct with content and detected format
fn read_template_from_file(path: &Path) -> Result<Template, Box<dyn Error>> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    // Try JSON first (faster and more common in automated deployments)
    match serde_json::from_str(&contents) {
        Ok(parsed) => Ok(Template::new(parsed, TemplateFormat::Json)),
        Err(_json_err) => {
            // Fallback to YAML parsing with CloudFormation tag support
            let has_cf_tags = contents.contains("!Ref")
                || contents.contains("!Sub")
                || contents.contains("!GetAtt")
                || contents.contains("!Join")
                || contents.contains("!Select")
                || contents.contains("!Split")
                || contents.contains("!FindInMap")
                || contents.contains("!Base64")
                || contents.contains("!GetAZs")
                || contents.contains("!ImportValue")
                || contents.contains("!If")
                || contents.contains("!Not")
                || contents.contains("!And")
                || contents.contains("!Or")
                || contents.contains("!Equals");

            // Try the CF-aware parser first
            match cfn_yaml::parse_yaml_to_json(&contents) {
                Ok(parsed) => {
                    if has_cf_tags {
                        eprintln!("\n⚠️  Warning: Template contains CloudFormation intrinsic function tags (!Ref, !Sub, etc.)");
                        eprintln!(
                            "   These will be converted to long-form when the template is updated:"
                        );
                        eprintln!("   - '!Ref MyParam' becomes 'Ref: MyParam'");
                        eprintln!("   - '!Sub ${{...}}' becomes 'Fn::Sub: ${{...}}'");
                        eprintln!("   Both forms are functionally equivalent and accepted by CloudFormation.\n");
                    }
                    Ok(Template::new(parsed, TemplateFormat::Yaml))
                }
                Err(_cf_yaml_err) => {
                    // If CF parser fails, try standard YAML parser as fallback
                    let parsed: serde_json::Value =
                        serde_yml::from_str(&contents).map_err(|yaml_err| {
                            format!(
                                "Failed to parse template file {} as JSON or YAML. YAML error: {}",
                                path.display(),
                                yaml_err
                            )
                        })?;
                    Ok(Template::new(parsed, TemplateFormat::Yaml))
                }
            }
        }
    }
}

/// Writes an error context file with debugging information
///
/// # Arguments
/// * `path` - Path where context file will be written (typically error-{operation}-{timestamp}.context.txt)
/// * `error_msg` - The error message
/// * `source_stack` - Source stack name
/// * `target_stack` - Target stack name (None for same-stack operations)
/// * `operation` - Operation type ("refactor" or "import")
/// * `resources` - List of resource IDs being migrated (old -> new mappings)
fn write_error_context(
    path: &Path,
    error_msg: &str,
    source_stack: &str,
    target_stack: Option<&str>,
    operation: &str,
    resources: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let timestamp = get_timestamp();
    let mut context = String::new();

    context.push_str(&format!("Error: {}\n", error_msg));
    context.push_str(&format!("Timestamp: {}\n", timestamp));
    context.push_str(&format!("Operation: {}\n", operation));
    context.push_str(&format!("Source Stack: {}\n", source_stack));

    if let Some(target) = target_stack {
        context.push_str(&format!("Target Stack: {}\n", target));
    }

    context.push_str("Resources:\n");
    for (old_id, new_id) in resources {
        if old_id == new_id {
            context.push_str(&format!("  - {} (no rename)\n", old_id));
        } else {
            context.push_str(&format!("  - {} -> {}\n", old_id, new_id));
        }
    }

    context.push_str("\nFull Error Details:\n");
    context.push_str(error_msg);
    context.push('\n');

    let mut file = fs::File::create(path).map_err(|e| {
        format!(
            "Failed to create error context file {}: {}",
            path.display(),
            e
        )
    })?;

    file.write_all(context.as_bytes())
        .map_err(|e| format!("Failed to write error context to {}: {}", path.display(), e))?;

    Ok(())
}

// End of Template File I/O Utilities
// ============================================================================

// ============================================================================
// Directory and Path Validation
// ============================================================================

/// Validates and resolves an output directory path.
///
/// This function ensures that the specified path:
/// - Exists or can be created
/// - Is a directory (not a file)
/// - Is writable
///
/// # Arguments
/// * `path` - The directory path to validate
///
/// # Returns
/// * `Ok(PathBuf)` - The validated absolute path
/// * `Err(Box<dyn Error>)` - If the path is invalid or cannot be used
///
/// # Example
/// ```ignore
/// let dir = validate_output_directory(Path::new("./templates"))?;
/// ```
fn validate_output_directory(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    // Convert to absolute path
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    // Check if path exists
    if abs_path.exists() {
        // Ensure it's a directory
        if !abs_path.is_dir() {
            return Err(
                format!("Path exists but is not a directory: {}", abs_path.display()).into(),
            );
        }

        // Check if directory is writable by attempting to create a temporary file
        let test_file = abs_path.join(".cfn-teleport-write-test");
        match fs::File::create(&test_file) {
            Ok(_) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file);
            }
            Err(e) => {
                return Err(
                    format!("Directory is not writable: {} ({})", abs_path.display(), e).into(),
                );
            }
        }
    } else {
        // Attempt to create the directory
        fs::create_dir_all(&abs_path).map_err(|e| {
            format!(
                "Failed to create output directory: {} ({})",
                abs_path.display(),
                e
            )
        })?;
    }

    Ok(abs_path)
}

// End of Directory and Path Validation
// ============================================================================

// ============================================================================
// Migration Specification File Parsing
// ============================================================================

/// Parses a migration specification file and returns resource ID mappings.
///
/// The migration spec file should be a JSON file with the following format:
/// ```json
/// {
///   "resources": {
///     "OldResourceId": "NewResourceId",
///     "AnotherResource": "RenamedResource"
///   }
/// }
/// ```
///
/// # Arguments
/// * `path` - Path to the migration specification file
///
/// # Returns
/// * `Ok(HashMap<String, String>)` - Map of old IDs to new IDs
/// * `Err(Box<dyn Error>)` - If the file cannot be read or parsed
///
/// # Example
/// ```ignore
/// let mappings = parse_migration_spec(Path::new("migration.json"))?;
/// ```
fn parse_migration_spec(path: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    // Read file contents
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "Failed to read migration spec file: {} ({})",
            path.display(),
            e
        )
    })?;

    // Parse JSON
    let parsed: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse migration spec as JSON: {} ({})",
            path.display(),
            e
        )
    })?;

    // Extract resources object
    let resources_obj = parsed
        .get("resources")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            format!(
                "Migration spec must contain a 'resources' object: {}",
                path.display()
            )
        })?;

    // Convert to HashMap
    let mut mappings = HashMap::new();
    for (old_id, new_id_value) in resources_obj {
        let new_id = new_id_value.as_str().ok_or_else(|| {
            format!(
                "Resource mapping value must be a string: {} -> {:?}",
                old_id, new_id_value
            )
        })?;

        mappings.insert(old_id.clone(), new_id.to_string());
    }

    if mappings.is_empty() {
        return Err(format!(
            "Migration spec contains no resource mappings: {}",
            path.display()
        )
        .into());
    }

    Ok(mappings)
}

/// Validates that all resource IDs in the migration spec exist in the source template.
///
/// # Arguments
/// * `mappings` - Resource ID mappings from migration spec
/// * `template` - Source CloudFormation template
///
/// # Returns
/// * `Ok(())` - If all resource IDs are valid
/// * `Err(Box<dyn Error>)` - If any resource ID is not found in the template
fn validate_migration_spec_resources(
    mappings: &HashMap<String, String>,
    template: &Template,
) -> Result<(), Box<dyn Error>> {
    let resources = template
        .content
        .get("Resources")
        .and_then(|v| v.as_object())
        .ok_or("Template does not contain a Resources section")?;

    let mut invalid_ids = Vec::new();
    for old_id in mappings.keys() {
        if !resources.contains_key(old_id) {
            invalid_ids.push(old_id.as_str());
        }
    }

    if !invalid_ids.is_empty() {
        return Err(format!(
            "Migration spec contains resource IDs that do not exist in the source template: {}",
            invalid_ids.join(", ")
        )
        .into());
    }

    Ok(())
}

// End of Migration Specification File Parsing
// ============================================================================

// ============================================================================
// Template Export Functions
// ============================================================================

/// Exports CloudFormation templates to disk with automatic collision handling.
///
/// This function writes one or more templates to the specified output directory,
/// automatically handling filename collisions by appending .1, .2, etc.
///
/// # Arguments
/// * `templates` - Vector of (template, suffix) tuples to export
/// * `output_dir` - Directory to write templates to (will be validated/created)
/// * `stack_name` - Stack name for filename generation
/// * `operation` - Operation type ("refactor", "import", etc.) for filename generation
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - Paths to all written template files
/// * `Err(Box<dyn Error>)` - If any file operation fails
///
/// # Example
/// ```ignore
/// let templates = vec![
///     (source_template, "source".to_string()),
///     (target_template, "target".to_string()),
/// ];
/// let paths = export_templates(&templates, Some(&output_dir), "MyStack", "refactor")?;
/// ```
fn export_templates(
    templates: &[(Template, String)],
    output_dir: Option<&PathBuf>,
    stack_name: &str,
    operation: &str,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    // Determine output directory (current dir if not specified)
    let dir = if let Some(d) = output_dir {
        validate_output_directory(d)?
    } else {
        std::env::current_dir()?
    };

    let timestamp = get_timestamp();
    let mut written_paths = Vec::new();

    for (template, suffix) in templates {
        // Generate filename with suffix and timestamp
        let extension = match template.format {
            TemplateFormat::Json => "json",
            TemplateFormat::Yaml => "yaml",
        };

        let filename_with_timestamp = format!(
            "{}-{}-{}-{}.{}",
            stack_name, operation, suffix, timestamp, extension
        );

        // Resolve any filename collisions
        let final_path = resolve_file_collision(&dir, &filename_with_timestamp)?;

        // Write template to file
        write_template_to_file(template, &final_path)?;

        // Notify user if collision occurred
        if final_path.file_name() != Some(std::ffi::OsStr::new(&filename_with_timestamp)) {
            println!(
                "  ⚠️  File exists, writing to: {}",
                final_path.file_name().unwrap().to_string_lossy()
            );
        }

        written_paths.push(final_path);
    }

    Ok(written_paths)
}

// End of Template Export Functions
// ============================================================================

async fn get_stacks(
    client: &cloudformation::Client,
) -> Result<Vec<cloudformation::types::StackSummary>, cloudformation::Error> {
    let mut stacks = Vec::new();
    let mut token = None;

    let stack_filter = vec![
        cloudformation::types::StackStatus::CreateInProgress,
        cloudformation::types::StackStatus::CreateFailed,
        cloudformation::types::StackStatus::CreateComplete,
        cloudformation::types::StackStatus::RollbackInProgress,
        cloudformation::types::StackStatus::RollbackFailed,
        cloudformation::types::StackStatus::RollbackComplete,
        cloudformation::types::StackStatus::DeleteFailed,
        cloudformation::types::StackStatus::UpdateInProgress,
        cloudformation::types::StackStatus::UpdateCompleteCleanupInProgress,
        cloudformation::types::StackStatus::UpdateComplete,
        cloudformation::types::StackStatus::UpdateFailed,
        cloudformation::types::StackStatus::UpdateRollbackInProgress,
        cloudformation::types::StackStatus::UpdateRollbackFailed,
        cloudformation::types::StackStatus::UpdateRollbackCompleteCleanupInProgress,
        cloudformation::types::StackStatus::UpdateRollbackComplete,
        cloudformation::types::StackStatus::ReviewInProgress,
        cloudformation::types::StackStatus::ImportInProgress,
        cloudformation::types::StackStatus::ImportComplete,
        cloudformation::types::StackStatus::ImportRollbackInProgress,
        cloudformation::types::StackStatus::ImportRollbackFailed,
        cloudformation::types::StackStatus::ImportRollbackComplete,
    ];

    loop {
        let query = match token {
            Some(token) => client.list_stacks().next_token(token),
            None => client.list_stacks(),
        };

        let resp = query
            .set_stack_status_filter(Some(stack_filter.clone()))
            .send()
            .await?;

        let new_stacks = resp.stack_summaries().to_vec();
        stacks.append(&mut new_stacks.clone());

        if let Some(next_token) = resp.next_token() {
            token = Some(next_token.to_owned());
        } else {
            break;
        }
    }

    let mut stacks = stacks
        .into_iter()
        .filter(|stack| !stack.stack_status().unwrap().as_str().starts_with("DELETE"))
        .collect::<Vec<_>>();

    if DEMO {
        // filter by name, for demo purposes
        stacks = stacks
            .into_iter()
            .filter(|stack| {
                stack
                    .stack_name()
                    .unwrap_or_default()
                    .contains("CfnTeleportTest")
            })
            .collect::<Vec<_>>();
    }

    // Sort the stacks by name
    let mut sorted_stacks = stacks;
    sorted_stacks.sort_by_key(|stack| stack.stack_name().unwrap_or_default().to_string());

    Ok(sorted_stacks)
}

fn select_stack<'a>(prompt: &str, items: &'a [&str]) -> Result<&'a str, Box<dyn Error>> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .report(false)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => Ok(items[index]),
        None => Err("User did not select anything".into()),
    }
}

async fn get_resources(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Vec<cloudformation::types::StackResourceSummary>, cloudformation::Error> {
    let resp = client
        .list_stack_resources()
        .stack_name(stack_name)
        .send()
        .await?;

    let resources = resp.stack_resource_summaries().to_vec();

    // Filter resources based on supported types
    let filtered_resources = resources
        .into_iter()
        .filter(|resource| {
            let resource_type = resource.resource_type().unwrap_or_default();
            supported_resource_types::SUPPORTED_RESOURCE_TYPES.contains(&resource_type)
        })
        .collect::<Vec<_>>();

    // Sort the resources by type, logical ID, and name
    let mut sorted_resources = filtered_resources;
    sorted_resources.sort_by_key(|resource| {
        (
            resource.resource_type().unwrap_or_default().to_string(),
            resource
                .logical_resource_id()
                .unwrap_or_default()
                .to_string(),
            resource
                .physical_resource_id()
                .unwrap_or_default()
                .to_string(),
        )
    });

    Ok(sorted_resources)
}

async fn filter_resources<'a>(
    resources: &'a [&aws_sdk_cloudformation::types::StackResourceSummary],
    filter: &[String],
) -> Result<Vec<&'a aws_sdk_cloudformation::types::StackResourceSummary>, Box<dyn Error>> {
    let mut filtered_resources = Vec::new();

    for resource in resources {
        let logical_id = resource.logical_resource_id().unwrap_or_default();

        if filter.contains(&logical_id.to_owned()) {
            filtered_resources.push(resource.to_owned());
        }
    }

    Ok(filtered_resources)
}

async fn select_resources<'a>(
    prompt: &str,
    resources: &'a [&aws_sdk_cloudformation::types::StackResourceSummary],
    template: &serde_json::Value,
    target_template: Option<&serde_json::Value>,
    is_cross_stack: bool,
) -> Result<Vec<&'a aws_sdk_cloudformation::types::StackResourceSummary>, Box<dyn Error>> {
    // Compute dependency markers for all resources
    let dependency_info = compute_dependency_markers(resources, template);

    // Generate and display legend if markers are present
    if let Some(legend) = generate_legend(&dependency_info) {
        println!("{}\n", legend);
    }

    // Format resources with dependency markers
    let items = format_resources(resources, None, Some(&dependency_info)).await?;

    // Create custom theme with grey checkmark for unchecked items
    let theme = ColorfulTheme {
        unchecked_item_prefix: style("✔".to_string()).for_stderr().dim(),
        ..ColorfulTheme::default()
    };

    let selection = MultiSelect::with_theme(&theme)
        .with_prompt(prompt)
        .report(false)
        .items(&items)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(indices) => {
            // Validate selection for cross-stack moves
            if is_cross_stack {
                let mut blocked_outputs = Vec::new();
                let mut blocked_parameters = Vec::new();

                // Get target template parameters if available
                let target_parameters: std::collections::HashSet<String> =
                    if let Some(target_tmpl) = target_template {
                        target_tmpl
                            .get("Parameters")
                            .and_then(|params| params.as_object())
                            .map(|params_obj| {
                                params_obj
                                    .keys()
                                    .map(|k| k.to_string())
                                    .collect::<std::collections::HashSet<String>>()
                            })
                            .unwrap_or_default()
                    } else {
                        std::collections::HashSet::new()
                    };

                // Get source template parameters
                let source_parameters: std::collections::HashSet<String> = template
                    .get("Parameters")
                    .and_then(|params| params.as_object())
                    .map(|params_obj| {
                        params_obj
                            .keys()
                            .map(|k| k.to_string())
                            .collect::<std::collections::HashSet<String>>()
                    })
                    .unwrap_or_default();

                for &index in &indices {
                    let resource = resources[index];
                    let logical_id = resource.logical_resource_id().unwrap_or_default();

                    if let Some(info) = dependency_info.get(logical_id) {
                        // Block resources referenced by outputs
                        if info.referenced_by_outputs {
                            blocked_outputs.push((
                                logical_id.to_string(),
                                resource.resource_type().unwrap_or_default().to_string(),
                            ));
                        }

                        // Block resources that depend on parameters not present in target
                        if info.depends_on_parameters {
                            // Find which parameters this resource depends on
                            let all_references = reference_updater::find_all_references(template);
                            let resource_references =
                                all_references.get(logical_id).cloned().unwrap_or_default();

                            // Check which parameters are missing in target
                            let missing_params: Vec<String> = resource_references
                                .iter()
                                .filter(|ref_name| source_parameters.contains(*ref_name))
                                .filter(|param_name| !target_parameters.contains(*param_name))
                                .map(|s| s.to_string())
                                .collect();

                            if !missing_params.is_empty() {
                                blocked_parameters.push((
                                    logical_id.to_string(),
                                    resource.resource_type().unwrap_or_default().to_string(),
                                    missing_params,
                                ));
                            }
                        }
                    }
                }

                // Build error messages
                let mut error_messages = Vec::new();

                if !blocked_outputs.is_empty() {
                    let resource_list = blocked_outputs
                        .iter()
                        .map(|(id, typ)| format!("  - {} ({})", id, typ))
                        .collect::<Vec<_>>()
                        .join("\n");

                    error_messages.push(format!(
                        "Cannot select the following resources because they are referenced by stack outputs:\n{}\n\
                         Outputs cannot be moved between stacks. Consider:\n\
                         - Removing or updating the outputs before moving\n\
                         - Using same-stack rename instead (references will be updated automatically)",
                        resource_list
                    ));
                }

                if !blocked_parameters.is_empty() {
                    let resource_list = blocked_parameters
                        .iter()
                        .map(|(id, typ, params)| {
                            format!(
                                "  - {} ({}) - missing parameters: {}",
                                id,
                                typ,
                                params.join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    error_messages.push(format!(
                        "Cannot select the following resources because they depend on parameters not present in target stack:\n{}\n\
                         Consider:\n\
                         - Adding the required parameters to the target stack\n\
                         - Removing parameter dependencies from these resources",
                        resource_list
                    ));
                }

                // If any resources are blocked, return error
                if !error_messages.is_empty() {
                    return Err(error_messages.join("\n\n").into());
                }
            }

            Ok(indices
                .into_iter()
                .map(|index| resources[index])
                .collect::<Vec<_>>())
        }
        None => Err("User did not select anything".into()),
    }
}

/// Validates that resources being moved don't have dangling references.
///
/// When moving resources between stacks, ensures that:
/// 1. Resources staying in source stack don't reference moving resources
/// 2. If a resource references another, both must be moved together
/// 3. Outputs in source stack don't reference moving resources (outputs can't be moved)
///
/// # Arguments
/// * `source_template` - The source stack template
/// * `new_logical_ids_map` - Map of resource IDs being moved (old ID -> new ID)
///
/// # Returns
/// Ok if validation passes, Err with detailed message if validation fails
fn validate_move_references(
    source_template: &serde_json::Value,
    new_logical_ids_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    use std::collections::HashSet;

    // Get all resources being moved
    let moving_resources: HashSet<String> = new_logical_ids_map.keys().cloned().collect();

    // Get all parameter names from the template (parameters are not resources)
    let parameter_names: HashSet<String> = source_template
        .get("Parameters")
        .and_then(|params| params.as_object())
        .map(|params_obj| {
            params_obj
                .keys()
                .map(|k| k.to_string())
                .collect::<HashSet<String>>()
        })
        .unwrap_or_default();

    // Find all references in the source template
    let all_references = reference_updater::find_all_references(source_template);

    let mut errors = Vec::new();

    // Check each resource that has references
    for (referencing_resource, referenced_resources) in &all_references {
        // Special case: Outputs section
        if referencing_resource == "Outputs" {
            // Check if any output references a moving resource
            for referenced in referenced_resources {
                if moving_resources.contains(referenced) {
                    errors.push(format!(
                        "  - Output section references resource '{}' which is being moved. \
                         Outputs cannot be moved between stacks. Please remove or update the output before moving the resource.",
                        referenced
                    ));
                }
            }
            continue;
        }

        // For regular resources: check if referencing resource is moving
        let is_referencing_resource_moving = moving_resources.contains(referencing_resource);

        // Check each referenced resource
        for referenced in referenced_resources {
            // Skip parameter references - parameters are stack-level config, not resources
            if parameter_names.contains(referenced) {
                continue;
            }

            let is_referenced_resource_moving = moving_resources.contains(referenced);

            // Problem: referencing resource stays, but referenced resource moves
            if !is_referencing_resource_moving && is_referenced_resource_moving {
                errors.push(format!(
                    "  - Resource '{}' references '{}', but only '{}' is being moved. \
                     Either move both resources together, or remove the reference before moving.",
                    referencing_resource, referenced, referenced
                ));
            }

            // NEW: Problem: referencing resource moves, but referenced resource stays
            // This catches resources being moved that depend on resources staying behind
            if is_referencing_resource_moving && !is_referenced_resource_moving {
                errors.push(format!(
                    "  - Resource '{}' depends on '{}' which is not being moved. \
                     Either move both resources together, or remove the dependency before moving.",
                    referencing_resource, referenced
                ));
            }
        }
    }

    // If we found any errors, return them all
    if !errors.is_empty() {
        let error_message = format!(
            "Cannot move resources due to dangling references:\n\n{}\n\n\
             Tip: You can move multiple resources together if they reference each other.\n\
             Tip: Same-stack renaming doesn't have this restriction.",
            errors.join("\n")
        );
        return Err(error_message.into());
    }

    Ok(())
}

fn user_confirm() -> Result<(), Box<dyn Error>> {
    let confirmed = Confirm::new()
        .with_prompt("Please confirm your selection:")
        .default(false)
        .interact_on_opt(&Term::stderr())?;

    println!();

    match confirmed {
        Some(true) => Ok(()),
        _ => Err("Selection has not been cofirmed".into()),
    }
}

async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
    template_file: Option<&PathBuf>,
) -> Result<Template, Box<dyn Error>> {
    // If template file is provided, read from file instead of AWS
    if let Some(file_path) = template_file {
        return read_template_from_file(file_path);
    }

    // Otherwise, fetch from AWS CloudFormation
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template_str = resp.template_body().ok_or("No template found")?;

    // Try JSON first (faster and more common in automated deployments)
    match serde_json::from_str(template_str) {
        Ok(parsed) => Ok(Template::new(parsed, TemplateFormat::Json)),
        Err(_json_err) => {
            // Fallback to YAML parsing with CloudFormation tag support
            // Check if template contains CloudFormation tags (!Ref, !Sub, etc.)
            let has_cf_tags = template_str.contains("!Ref")
                || template_str.contains("!Sub")
                || template_str.contains("!GetAtt")
                || template_str.contains("!Join")
                || template_str.contains("!Select")
                || template_str.contains("!Split")
                || template_str.contains("!FindInMap")
                || template_str.contains("!Base64")
                || template_str.contains("!GetAZs")
                || template_str.contains("!ImportValue")
                || template_str.contains("!If")
                || template_str.contains("!Not")
                || template_str.contains("!And")
                || template_str.contains("!Or")
                || template_str.contains("!Equals");

            // Try the CF-aware parser first (handles !Ref, !Sub, etc.)
            match cfn_yaml::parse_yaml_to_json(template_str) {
                Ok(parsed) => {
                    if has_cf_tags {
                        eprintln!("\n⚠️  Warning: Template contains CloudFormation intrinsic function tags (!Ref, !Sub, etc.)");
                        eprintln!(
                            "   These will be converted to long-form when the template is updated:"
                        );
                        eprintln!("   - '!Ref MyParam' becomes 'Ref: MyParam'");
                        eprintln!("   - '!Sub ${{...}}' becomes 'Fn::Sub: ${{...}}'");
                        eprintln!("   Both forms are functionally equivalent and accepted by CloudFormation.\n");
                    }
                    Ok(Template::new(parsed, TemplateFormat::Yaml))
                }
                Err(_cf_yaml_err) => {
                    // If CF parser fails, try standard YAML parser as fallback
                    let parsed: serde_json::Value =
                        serde_yml::from_str(template_str).map_err(|yaml_err| {
                            format!(
                                "Failed to parse template as JSON or YAML. YAML error: {}",
                                yaml_err
                            )
                        })?;
                    Ok(Template::new(parsed, TemplateFormat::Yaml))
                }
            }
        }
    }
}

/// Computes dependency markers for resources based on template analysis.
///
/// Analyzes the template to identify which resources:
/// - Have incoming dependencies (other resources reference them)
/// - Have outgoing dependencies (they reference other resources)
/// - Are referenced by stack outputs
/// - Depend on stack parameters
///
/// # Arguments
/// * `resources` - Slice of resource summaries to analyze
/// * `template` - The CloudFormation template as JSON
///
/// # Returns
/// HashMap mapping logical resource IDs to their dependency information
fn compute_dependency_markers(
    resources: &[&cloudformation::types::StackResourceSummary],
    template: &serde_json::Value,
) -> HashMap<String, ResourceDependencyInfo> {
    let mut dependency_map: HashMap<String, ResourceDependencyInfo> = HashMap::new();

    // Get all references in the template
    let all_references = reference_updater::find_all_references(template);

    // Check which resources are referenced by outputs
    let output_references = all_references.get("Outputs").cloned().unwrap_or_default();

    // Get list of parameter names from the template (compute once, reuse for all resources)
    let parameter_names: std::collections::HashSet<String> = template
        .get("Parameters")
        .and_then(|params| params.as_object())
        .map(|params_obj| {
            params_obj
                .keys()
                .map(|k| k.to_string())
                .collect::<std::collections::HashSet<String>>()
        })
        .unwrap_or_default();

    // Get the set of resources in the template (compute once, reuse for all resources)
    let resource_names: std::collections::HashSet<String> = template
        .get("Resources")
        .and_then(|res| res.as_object())
        .map(|res_obj| {
            res_obj
                .keys()
                .map(|k| k.to_string())
                .collect::<std::collections::HashSet<String>>()
        })
        .unwrap_or_default();

    // Initialize dependency info for all resources
    for resource in resources {
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let referenced_by_outputs = output_references.contains(logical_id);

        // Check if this resource is referenced by any other resource (incoming)
        let mut has_incoming_deps = false;
        for (referencing_resource, referenced_resources) in &all_references {
            // Skip the Outputs section - we already handled it
            if referencing_resource == "Outputs" {
                continue;
            }
            // Check if this resource is in the referenced set
            if referenced_resources.contains(logical_id) {
                has_incoming_deps = true;
                break;
            }
        }

        // Check if this resource references other resources (outgoing)
        // First, get what this resource references
        let resource_references = all_references.get(logical_id).cloned().unwrap_or_default();

        // A resource has outgoing dependencies if it references OTHER RESOURCES
        // (not parameters, not pseudo-parameters)
        let has_outgoing_deps = resource_references
            .iter()
            .any(|ref_name| resource_names.contains(ref_name));

        // Check if this resource depends on parameters
        // Parameters are things referenced by this resource but not in the Resources section
        let depends_on_parameters = resource_references
            .iter()
            .any(|ref_name| parameter_names.contains(ref_name));

        dependency_map.insert(
            logical_id.to_string(),
            ResourceDependencyInfo {
                has_incoming_deps,
                has_outgoing_deps,
                referenced_by_outputs,
                depends_on_parameters,
            },
        );
    }

    dependency_map
}

/// Generates a legend for dependency markers based on what markers are present.
///
/// Only returns a legend if there are actually markers to display.
/// Only includes entries for markers that are present in the dependency info.
///
/// # Arguments
/// * `dependency_info` - Map of resource IDs to their dependency information
///
/// # Returns
/// Some(legend string) if markers are present, None if no markers
fn generate_legend(dependency_info: &HashMap<String, ResourceDependencyInfo>) -> Option<String> {
    let mut has_incoming_deps = false;
    let mut has_outgoing_deps = false;
    let mut has_output_refs = false;
    let mut has_parameter_deps = false;

    // Scan to see which markers are present
    for info in dependency_info.values() {
        if info.has_incoming_deps {
            has_incoming_deps = true;
        }
        if info.has_outgoing_deps {
            has_outgoing_deps = true;
        }
        if info.referenced_by_outputs {
            has_output_refs = true;
        }
        if info.depends_on_parameters {
            has_parameter_deps = true;
        }
        // Early exit if we found all four
        if has_incoming_deps && has_outgoing_deps && has_output_refs && has_parameter_deps {
            break;
        }
    }

    // If no markers, return None
    if !has_incoming_deps && !has_outgoing_deps && !has_output_refs && !has_parameter_deps {
        return None;
    }

    // Build legend with only present markers
    let mut legend = String::from("Legend:");

    // Show resource dependency line if any resource deps exist
    if has_incoming_deps || has_outgoing_deps {
        legend.push_str(&format!(
            "\n  Resource dependencies: {}  incoming  {}  outgoing    {}  bidirectional",
            EMOJI_INCOMING, EMOJI_OUTGOING, EMOJI_BIDIRECTIONAL
        ));
    }

    // Show stack interface line if any outputs or parameters exist
    if has_output_refs || has_parameter_deps {
        legend.push_str(&format!(
            "\n  Stack interface:       {}  outputs   {}  parameters  {}  both",
            EMOJI_OUTPUTS, EMOJI_PARAMETERS, EMOJI_BOTH_STACK_INTERFACE
        ));
    }

    Some(legend)
}

async fn format_resources(
    resources: &[&cloudformation::types::StackResourceSummary],
    resource_id_map: Option<HashMap<String, String>>,
    dependency_info: Option<&HashMap<String, ResourceDependencyInfo>>,
) -> Result<Vec<String>, io::Error> {
    let mut max_lengths = [0; 3];
    let mut formatted_resources = Vec::new();

    let mut renamed = false;

    // Calculate max emoji count across all resources
    let max_emoji_count = if let Some(dep_info) = dependency_info {
        resources
            .iter()
            .map(|resource| {
                let logical_id = resource.logical_resource_id().unwrap_or_default();
                if let Some(info) = dep_info.get(logical_id) {
                    let has_lr = info.has_incoming_deps || info.has_outgoing_deps;
                    let has_ud = info.referenced_by_outputs || info.depends_on_parameters;
                    match (has_lr, has_ud) {
                        (true, true) => 2,
                        (true, false) | (false, true) => 1,
                        (false, false) => 0,
                    }
                } else {
                    0
                }
            })
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    for resource in resources.iter() {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();

        let new_logical_id = match resource_id_map {
            Some(ref map) => match map.get(logical_id) {
                Some(new_id) => new_id.to_string(),
                None => logical_id.to_string(),
            },
            None => logical_id.to_string(),
        };

        max_lengths[0] = max_lengths[0].max(resource_type.len());
        max_lengths[1] = max_lengths[1].max(logical_id.len());
        if logical_id != new_logical_id {
            max_lengths[2] = max_lengths[2].max(new_logical_id.len());
            renamed = true;
        }
    }

    for resource in resources.iter() {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let physical_id = resource.physical_resource_id().unwrap_or_default();
        let new_logical_id = match resource_id_map {
            Some(ref map) => match map.get(logical_id) {
                Some(new_id) => new_id.to_string(),
                None => logical_id.to_string(),
            },
            None => logical_id.to_string(),
        };

        // Generate marker string based on dependency info
        // Dynamically adjust spacing based on max emoji count
        let marker = if max_emoji_count == 0 {
            // No dependencies at all - no marker column needed
            String::new()
        } else if let Some(dep_info) = dependency_info {
            if let Some(info) = dep_info.get(logical_id) {
                let mut marker_str = String::new();

                // Left/Right arrows for resource dependencies
                if info.has_incoming_deps && info.has_outgoing_deps {
                    marker_str.push_str(EMOJI_BIDIRECTIONAL);
                } else if info.has_incoming_deps {
                    marker_str.push_str(EMOJI_INCOMING);
                } else if info.has_outgoing_deps {
                    marker_str.push_str(EMOJI_OUTGOING);
                }

                // Add space between if we have both types
                let has_lr = !marker_str.is_empty();
                let has_ud = info.referenced_by_outputs || info.depends_on_parameters;
                if has_lr && has_ud {
                    marker_str.push(' ');
                } else if has_lr && !has_ud && max_emoji_count == 2 {
                    // Pad single emoji to match 2-emoji width if max is 2
                    marker_str.push_str("  ");
                }

                // Up/Down arrows for outputs/parameters
                if info.referenced_by_outputs && info.depends_on_parameters {
                    marker_str.push_str(EMOJI_BOTH_STACK_INTERFACE);
                } else if info.referenced_by_outputs {
                    marker_str.push_str(EMOJI_OUTPUTS);
                    if !has_lr && max_emoji_count == 2 {
                        // Pad single emoji to match 2-emoji width if max is 2
                        marker_str.insert_str(0, "  ");
                    }
                } else if info.depends_on_parameters {
                    marker_str.push_str(EMOJI_PARAMETERS);
                    if !has_lr && max_emoji_count == 2 {
                        // Pad single emoji to match 2-emoji width if max is 2
                        marker_str.insert_str(0, "  ");
                    }
                }

                // If no markers for this resource, pad based on max_emoji_count
                if marker_str.is_empty() {
                    match max_emoji_count {
                        1 => "  ".to_string(),  // Match ~1 emoji width
                        2 => "   ".to_string(), // Match ~2 emoji width
                        _ => String::new(),
                    }
                } else {
                    marker_str
                }
            } else {
                // No info for this resource, pad based on max_emoji_count
                match max_emoji_count {
                    1 => "  ".to_string(),
                    2 => "   ".to_string(),
                    _ => String::new(),
                }
            }
        } else {
            String::new()
        };

        let output = if renamed {
            let renamed_indicator = if logical_id != new_logical_id {
                format!(" ► {}", new_logical_id)
            } else {
                "".to_string()
            };
            if max_emoji_count > 0 {
                format!(
                    "{:<width1$}  {}  {:<width2$}{:<width3$}  {}",
                    resource_type,
                    marker,
                    logical_id,
                    renamed_indicator,
                    physical_id,
                    width1 = max_lengths[0],
                    width2 = max_lengths[1],
                    width3 = max_lengths[2] + 4
                )
            } else {
                format!(
                    "{:<width1$} {:<width2$}{:<width3$}  {}",
                    resource_type,
                    logical_id,
                    renamed_indicator,
                    physical_id,
                    width1 = max_lengths[0],
                    width2 = max_lengths[1],
                    width3 = max_lengths[2] + 4
                )
            }
        } else if max_emoji_count > 0 {
            format!(
                "{:<width1$}  {}  {:<width2$}  {}",
                resource_type,
                marker,
                logical_id,
                physical_id,
                width1 = max_lengths[0],
                width2 = max_lengths[1]
            )
        } else {
            format!(
                "{:<width1$} {:<width2$}  {}",
                resource_type,
                logical_id,
                physical_id,
                width1 = max_lengths[0],
                width2 = max_lengths[1]
            )
        };

        formatted_resources.push(output);
    }

    Ok(formatted_resources)
}

fn retain_resources(
    mut template: serde_json::Value,
    resource_ids: Vec<String>,
) -> serde_json::Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        if let Some(resource) = resources.get_mut(&resource_id) {
            resource["DeletionPolicy"] = serde_json::Value::String("Retain".to_string());
        }
    }

    template
}

// for reasons unknown, importing resource requires a DeletionPolicy to be set. Se we add the documented defaults
// https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-attribute-deletionpolicy.html
fn set_default_deletion_policy(
    mut template: serde_json::Value,
    resource_ids: Vec<String>,
) -> serde_json::Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        if let Some(resource) = resources.get_mut(&resource_id) {
            if resource.is_object() {
                let resource_object = resource.as_object_mut().unwrap();
                if !resource_object.contains_key("DeletionPolicy") {
                    let resource_type = resource_object["Type"].as_str().unwrap();
                    let deletion_policy = match resource_type {
                        "AWS::RDS::DBCluster" => "Snapshot",
                        "AWS::RDS::DBInstance" => {
                            if resource_object.contains_key("DBClusterIdentifier") {
                                "Delete"
                            } else {
                                "Snapshot"
                            }
                        }
                        _ => "Delete",
                    };
                    resource["DeletionPolicy"] =
                        serde_json::Value::String(deletion_policy.to_string());
                }
            }
        }
    }

    template
}

fn remove_resources(
    mut template: serde_json::Value,
    resource_ids: Vec<String>,
) -> serde_json::Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        resources.remove(&resource_id);
    }

    template
}

fn add_resources(
    mut target_template: serde_json::Value,
    source_template: serde_json::Value,
    resource_id_map: HashMap<String, String>,
) -> (serde_json::Value, serde_json::Value) {
    let target_resources = target_template["Resources"].as_object_mut().unwrap();
    let source_resources = source_template["Resources"].as_object().unwrap();

    for (resource_id, new_resource_id) in resource_id_map.clone() {
        if let Some(resource) = source_resources.get(&resource_id) {
            target_resources.insert(new_resource_id, resource.clone());
        }
    }

    let target_template_with_deletion_policy = set_default_deletion_policy(
        target_template.clone(),
        resource_id_map.values().map(|x| x.to_string()).collect(),
    );

    (target_template_with_deletion_policy, target_template)
}

async fn validate_template(
    client: &cloudformation::Client,
    template: serde_json::Value,
) -> Result<(), cloudformation::Error> {
    match client
        .validate_template()
        .template_body(serde_json::to_string(&template).unwrap())
        .send()
        .await
    {
        Ok(_output) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn refactor_stack_resources(
    client: &cloudformation::Client,
    stack_name: &str,
    template: Template,
    id_mapping: HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};

    // Step 1: Create updated template with renamed resources and updated references
    let mut updated_template = template.content.clone();

    // Rename resources in the Resources section
    if let Some(resources) = updated_template
        .get_mut("Resources")
        .and_then(|r| r.as_object_mut())
    {
        let keys_to_rename: Vec<(String, String)> = id_mapping
            .iter()
            .map(|(old, new)| (old.clone(), new.clone()))
            .collect();

        for (old_id, new_id) in keys_to_rename {
            if let Some(resource) = resources.remove(&old_id) {
                resources.insert(new_id, resource);
            }
        }
    }

    // Update all references using our reference updater
    // This is needed because we're sending an updated template to CloudFormation,
    // and the template must have valid references for validation to pass
    let updated_template =
        reference_updater::update_template_references(updated_template, &id_mapping);

    // Validate the updated template
    if let Err(e) = validate_template(client, updated_template.clone()).await {
        eprintln!("\n⚠️  Template validation failed. Saving template for debugging...");

        // Save template
        let template_to_save = Template::new(updated_template.clone(), template.format);
        let templates = vec![(template_to_save, "refactored".to_string())];

        if let Ok(paths) =
            export_templates(&templates, None, stack_name, "error-refactor-same-stack")
        {
            eprintln!("📄 Template saved to:");
            for path in &paths {
                eprintln!("   {}", path.display());
            }
        }

        // Save error context
        let timestamp = get_timestamp();
        let context_filename = format!("{}-error-refactor-context-{}.txt", stack_name, timestamp);
        let context_path = std::env::current_dir()
            .ok()
            .map(|d| d.join(context_filename));

        if let Some(path) = context_path {
            if write_error_context(
                &path,
                &e.to_string(),
                stack_name,
                None,
                "refactor-same-stack",
                &id_mapping,
            )
            .is_ok()
            {
                eprintln!("📝 Error context saved to: {}", path.display());
            }
        }

        return Err(format!(
            "Updated template validation failed: {}\n\
             Template and error context have been saved for debugging.",
            e
        )
        .into());
    }

    // Step 2: Build resource mappings for CloudFormation
    let resource_mappings: Vec<ResourceMapping> = id_mapping
        .iter()
        .map(|(old_id, new_id)| {
            ResourceMapping::builder()
                .source(
                    ResourceLocation::builder()
                        .stack_name(stack_name)
                        .logical_resource_id(old_id)
                        .build(),
                )
                .destination(
                    ResourceLocation::builder()
                        .stack_name(stack_name)
                        .logical_resource_id(new_id)
                        .build(),
                )
                .build()
        })
        .collect();

    // Step 3-6: Create, validate, and execute stack refactor (with simplified output)
    let mut spinner = spinner::Spin::new(&format!(
        "Renaming {} resource{} in stack {}",
        id_mapping.len(),
        if id_mapping.len() == 1 { "" } else { "s" },
        stack_name,
    ));

    let stack_definition = StackDefinition::builder()
        .stack_name(stack_name)
        .template_body(Template::new(updated_template.clone(), template.format).to_string()?)
        .build();

    // Create refactor
    let create_result = client
        .create_stack_refactor()
        .stack_definitions(stack_definition)
        .set_resource_mappings(Some(resource_mappings))
        .send()
        .await
        .map_err(|e| format!("Failed to create stack refactor: {}", e))?;

    let refactor_id = create_result
        .stack_refactor_id()
        .ok_or("No stack refactor ID returned")?;

    // Wait for validation to complete
    loop {
        let status = client
            .describe_stack_refactor()
            .stack_refactor_id(refactor_id)
            .send()
            .await
            .map_err(|e| format!("Failed to describe stack refactor: {}", e))?;

        match status.status().map(|s| s.as_str()) {
            Some("CREATE_COMPLETE") => {
                break;
            }
            Some("CREATE_FAILED") | Some("FAILED") => {
                spinner.fail();
                return Err(format!(
                    "Stack refactor validation failed: {}",
                    status.status_reason().unwrap_or("Unknown error")
                )
                .into());
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    // Execute the refactor
    client
        .execute_stack_refactor()
        .stack_refactor_id(refactor_id)
        .send()
        .await
        .map_err(|e| format!("Failed to execute stack refactor: {}", e))?;

    // Wait for execution to complete
    loop {
        let status = client
            .describe_stack_refactor()
            .stack_refactor_id(refactor_id)
            .send()
            .await
            .map_err(|e| format!("Failed to describe stack refactor: {}", e))?;

        match status.execution_status().map(|s| s.as_str()) {
            Some("EXECUTE_COMPLETE") => {
                spinner.complete();
                println!(
                    "Renamed {} resource{} in stack {}",
                    id_mapping.len(),
                    if id_mapping.len() == 1 { "" } else { "s" },
                    stack_name
                );
                for (old_id, new_id) in id_mapping {
                    println!("  {} → {}", old_id, new_id);
                }
                return Ok(());
            }
            Some("EXECUTE_FAILED") | Some("FAILED") => {
                spinner.fail();
                return Err(format!(
                    "Stack refactor execution failed: {}",
                    status.status_reason().unwrap_or("Unknown error")
                )
                .into());
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Refactor CloudFormation stack resources across stacks using the Stack Refactoring API.
/// This function handles cross-stack moves only (use refactor_stack_resources for same-stack).
///
/// # Arguments
/// * `client` - CloudFormation client
/// * `source_stack_name` - Source stack name
/// * `target_stack_name` - Target stack name
/// * `source_template` - Source stack template
/// * `target_template` - Target stack template
/// * `id_mapping` - Map of old logical IDs to new logical IDs
///
/// # Returns
/// * `Ok(())` if refactoring succeeds
/// * `Err` if validation or execution fails
async fn refactor_stack_resources_cross_stack(
    client: &cloudformation::Client,
    source_stack_name: &str,
    target_stack_name: &str,
    source_template: Template,
    target_template: Template,
    id_mapping: HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};

    let resource_ids: Vec<String> = id_mapping.keys().cloned().collect();

    // Validate that resources being moved don't have dangling references
    // Check both directions:
    // 1. Resources staying in source that reference moving resources
    // 2. Resources being moved that depend on staying resources
    validate_move_references(&source_template.content, &id_mapping)?;

    // Step 1: Remove resources from source template
    let source_without_resources =
        remove_resources(source_template.content.clone(), resource_ids.clone());

    // Step 2: Add resources to target template
    // Note: add_resources returns (template_with_deletion_policy, template_without)
    // For refactor mode, we use the original template without DeletionPolicy modifications
    let (_, target_with_resources) = add_resources(
        target_template.content.clone(),
        source_template.content.clone(),
        id_mapping.clone(),
    );

    // Step 3: Update references in both templates
    let source_final =
        reference_updater::update_template_references(source_without_resources, &id_mapping);
    let target_final =
        reference_updater::update_template_references(target_with_resources, &id_mapping);

    // Step 4: Validate both templates
    let source_validation = validate_template(client, source_final.clone()).await;
    let target_validation = validate_template(client, target_final.clone()).await;

    if source_validation.is_err() || target_validation.is_err() {
        eprintln!("\n⚠️  Template validation failed. Saving templates for debugging...");

        // Save both templates
        let templates = vec![
            (
                Template::new(source_final.clone(), source_template.format),
                format!("source-{}", source_stack_name),
            ),
            (
                Template::new(target_final.clone(), target_template.format),
                format!("target-{}", target_stack_name),
            ),
        ];

        if let Ok(paths) = export_templates(
            &templates,
            None,
            source_stack_name,
            "error-refactor-cross-stack",
        ) {
            eprintln!("📄 Templates saved to:");
            for path in &paths {
                eprintln!("   {}", path.display());
            }
        }

        // Save error context
        let timestamp = get_timestamp();
        let context_filename = format!(
            "{}-error-refactor-cross-stack-context-{}.txt",
            source_stack_name, timestamp
        );
        let context_path = std::env::current_dir()
            .ok()
            .map(|d| d.join(context_filename));

        let error_msg = match (&source_validation, &target_validation) {
            (Err(e), _) => format!("Source template validation failed: {}", e),
            (_, Err(e)) => format!("Target template validation failed: {}", e),
            _ => "Unknown validation error".to_string(),
        };

        if let Some(path) = context_path {
            if write_error_context(
                &path,
                &error_msg,
                source_stack_name,
                Some(target_stack_name),
                "refactor-cross-stack",
                &id_mapping,
            )
            .is_ok()
            {
                eprintln!("📝 Error context saved to: {}", path.display());
            }
        }

        // Return appropriate error
        return Err(format!(
            "{}\nTemplates and error context have been saved for debugging.",
            error_msg
        )
        .into());
    }

    // Step 5: Build resource mappings for CloudFormation
    let resource_mappings: Vec<ResourceMapping> = id_mapping
        .iter()
        .map(|(old_id, new_id)| {
            ResourceMapping::builder()
                .source(
                    ResourceLocation::builder()
                        .stack_name(source_stack_name)
                        .logical_resource_id(old_id)
                        .build(),
                )
                .destination(
                    ResourceLocation::builder()
                        .stack_name(target_stack_name)
                        .logical_resource_id(new_id)
                        .build(),
                )
                .build()
        })
        .collect();

    // Step 6: Build stack definitions for both stacks
    let mut spinner = spinner::Spin::new(&format!(
        "Moving {} resource{} from {} to {}",
        id_mapping.len(),
        if id_mapping.len() == 1 { "" } else { "s" },
        source_stack_name,
        target_stack_name
    ));

    let source_stack_definition = StackDefinition::builder()
        .stack_name(source_stack_name)
        .template_body(Template::new(source_final.clone(), source_template.format).to_string()?)
        .build();

    let target_stack_definition = StackDefinition::builder()
        .stack_name(target_stack_name)
        .template_body(Template::new(target_final.clone(), target_template.format).to_string()?)
        .build();

    // Step 7: Create refactor
    let create_result = client
        .create_stack_refactor()
        .stack_definitions(source_stack_definition)
        .stack_definitions(target_stack_definition)
        .set_resource_mappings(Some(resource_mappings))
        .send()
        .await
        .map_err(|e| format!("Failed to create stack refactor: {}", e))?;

    let refactor_id = create_result
        .stack_refactor_id()
        .ok_or("No stack refactor ID returned")?;

    // Step 8: Wait for validation to complete
    loop {
        let status = client
            .describe_stack_refactor()
            .stack_refactor_id(refactor_id)
            .send()
            .await
            .map_err(|e| format!("Failed to describe stack refactor: {}", e))?;

        match status.status().map(|s| s.as_str()) {
            Some("CREATE_COMPLETE") => {
                break;
            }
            Some("CREATE_FAILED") | Some("FAILED") => {
                spinner.fail();
                return Err(format!(
                    "Stack refactor validation failed: {}",
                    status.status_reason().unwrap_or("Unknown error")
                )
                .into());
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    // Step 9: Execute the refactor
    client
        .execute_stack_refactor()
        .stack_refactor_id(refactor_id)
        .send()
        .await
        .map_err(|e| format!("Failed to execute stack refactor: {}", e))?;

    // Step 10: Wait for execution to complete
    loop {
        let status = client
            .describe_stack_refactor()
            .stack_refactor_id(refactor_id)
            .send()
            .await
            .map_err(|e| format!("Failed to describe stack refactor: {}", e))?;

        match status.execution_status().map(|s| s.as_str()) {
            Some("EXECUTE_COMPLETE") => {
                spinner.complete();
                println!(
                    "Moved {} resource{} from {} to {}",
                    id_mapping.len(),
                    if id_mapping.len() == 1 { "" } else { "s" },
                    source_stack_name,
                    target_stack_name
                );
                for (old_id, new_id) in id_mapping {
                    println!("  {} → {}", old_id, new_id);
                }
                return Ok(());
            }
            Some("EXECUTE_FAILED") | Some("FAILED") => {
                spinner.fail();
                return Err(format!(
                    "Stack refactor execution failed: {}",
                    status.status_reason().unwrap_or("Unknown error")
                )
                .into());
            }
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn update_stack(
    client: &cloudformation::Client,
    stack_name: &str,
    template: serde_json::Value,
    format: TemplateFormat,
) -> Result<(), Box<dyn Error>> {
    let template_body = Template::new(template, format).to_string()?;

    match client
        .update_stack()
        .stack_name(stack_name)
        .template_body(template_body)
        // @TODO: we can detect the required capabilities from the output of validate_template()
        .capabilities(cloudformation::types::Capability::CapabilityIam)
        .capabilities(cloudformation::types::Capability::CapabilityNamedIam)
        .capabilities(cloudformation::types::Capability::CapabilityAutoExpand)
        .send()
        .await
    {
        Ok(_output) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn get_stack_status(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Option<cloudformation::types::StackStatus>, Box<dyn std::error::Error>> {
    let describe_stacks_output = match client.describe_stacks().stack_name(stack_name).send().await
    {
        Ok(output) => output,
        Err(err) => return Err(Box::new(err)),
    };

    let stacks = describe_stacks_output.stacks();
    let stack = stacks.first();

    if let Some(stack) = stack {
        Ok(stack.stack_status.clone())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to determine stack status",
        )))
    }
}

async fn wait_for_stack_update_completion(
    client: &cloudformation::Client,
    stack_name: &str,
    mut spinner: Option<spinner::Spin>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stack_status = get_stack_status(client, stack_name).await?;

    while let Some(status) = stack_status.clone() {
        if status == cloudformation::types::StackStatus::UpdateInProgress
            || status == cloudformation::types::StackStatus::UpdateCompleteCleanupInProgress
            || status == cloudformation::types::StackStatus::ImportInProgress
        {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            stack_status = get_stack_status(client, stack_name).await?;
        } else {
            if status != cloudformation::types::StackStatus::UpdateComplete
                && status != cloudformation::types::StackStatus::ImportComplete
            {
                return Err(
                    format!("Stack update failed {}", stack_status.unwrap().as_str()).into(),
                );
            }
            break;
        }
    }

    if let Some(spinner) = spinner.as_mut() {
        spinner.complete();
    }

    Ok(())
}

async fn get_resource_identifier_mapping(
    client: &cloudformation::Client,
    template_body: &str,
) -> Result<HashMap<String, String>, cloudformation::Error> {
    match client
        .get_template_summary()
        .template_body(template_body)
        .send()
        .await
    {
        Ok(output) => {
            let mut map = HashMap::new();
            for item in output.resource_identifier_summaries().iter() {
                item.logical_resource_ids().iter().for_each(|logical_id| {
                    let resource_identifier = item.resource_identifiers().first().unwrap();
                    map.insert(logical_id.to_string(), resource_identifier.to_string());
                });
            }
            Ok(map)
        }
        Err(err) => Err(err.into()),
    }
}

async fn create_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    template: serde_json::Value,
    format: TemplateFormat,
    resources_to_import: Vec<&cloudformation::types::StackResourceSummary>,
    new_logical_ids_map: HashMap<String, String>,
) -> Result<std::string::String, Box<dyn Error>> {
    let template_string = Template::new(template.clone(), format).to_string()?;
    let resource_identifiers = get_resource_identifier_mapping(client, &template_string).await?;
    let resources = resources_to_import
        .iter()
        .map(|resource| {
            let resource_type = resource.resource_type().unwrap_or_default();
            let logical_id = resource.logical_resource_id().unwrap_or_default();
            let logical_id_new = match new_logical_ids_map.get(logical_id) {
                Some(key) => key,
                None => logical_id,
            };

            let physical_id = resource.physical_resource_id().unwrap_or_default();

            let resouce_identifier = resource_identifiers.get(logical_id_new).unwrap();

            cloudformation::types::ResourceToImport::builder()
                .resource_type(resource_type.to_string())
                .logical_resource_id(logical_id_new.to_string())
                .set_resource_identifier(Some(
                    vec![(resouce_identifier.to_string(), physical_id.to_string())]
                        .into_iter()
                        .collect(),
                ))
                .build()
        })
        .collect::<Vec<_>>();

    let change_set_name = format!("{}-{}", stack_name, Uuid::new_v4());

    match client
        .create_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name.clone())
        .template_body(template_string)
        .change_set_type(cloudformation::types::ChangeSetType::Import)
        .set_resources_to_import(resources.into())
        // @TODO: we can detect the required capabilities from the output of validate_template()
        .capabilities(cloudformation::types::Capability::CapabilityIam)
        .capabilities(cloudformation::types::Capability::CapabilityNamedIam)
        .capabilities(cloudformation::types::Capability::CapabilityAutoExpand)
        .send()
        .await
    {
        Ok(_) => Ok(change_set_name),
        Err(err) => Err(err.into()),
    }
}

async fn execute_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    change_set_name: &str,
) -> Result<(), cloudformation::Error> {
    match client
        .execute_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn get_changeset_status(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<Option<cloudformation::types::ChangeSetStatus>, Box<dyn std::error::Error>> {
    let change_set = match client
        .describe_change_set()
        .stack_name(stack_name)
        .change_set_name(changeset_name)
        .send()
        .await
    {
        Ok(output) => output,
        Err(err) => return Err(Box::new(err)),
    };

    if change_set.status == Some(cloudformation::types::ChangeSetStatus::Failed) {
        println!("{:?}", change_set);
        return Err(change_set.status_reason().unwrap().to_string().into());
    }

    Ok(change_set.status)
}

async fn wait_for_changeset_created(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut changeset_status = get_changeset_status(client, stack_name, changeset_name).await?;

    while let Some(status) = changeset_status.clone() {
        if status == cloudformation::types::ChangeSetStatus::CreateInProgress
            || status == cloudformation::types::ChangeSetStatus::CreatePending
        {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            changeset_status = get_changeset_status(client, stack_name, changeset_name).await?;
        } else {
            if status != cloudformation::types::ChangeSetStatus::CreateComplete {
                return Err(format!(
                    "Changeset creation failed {}",
                    changeset_status.unwrap().as_str()
                )
                .into());
            }
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_move_references_outgoing_dependency() {
        // Test: Moving resource depends on staying resource - should error
        let template = json!({
            "Resources": {
                "Instance": {
                    "Type": "AWS::EC2::Instance",
                    "Properties": {
                        "SecurityGroupIds": [
                            {"Ref": "SecurityGroup"}
                        ]
                    }
                },
                "SecurityGroup": {
                    "Type": "AWS::EC2::SecurityGroup",
                    "Properties": {}
                }
            }
        });

        let mut id_mapping = HashMap::new();
        id_mapping.insert("Instance".to_string(), "Instance".to_string());
        // Not moving SecurityGroup

        let result = validate_move_references(&template, &id_mapping);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Instance"));
        assert!(err_msg.contains("SecurityGroup"));
        assert!(err_msg.contains("depends on"));
    }

    #[test]
    fn test_validate_move_references_circular_dependencies() {
        // Test: Resources reference each other, both being moved - should succeed
        let template = json!({
            "Resources": {
                "ResourceA": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "Tags": [
                            {"Key": "RefB", "Value": {"Ref": "ResourceB"}}
                        ]
                    }
                },
                "ResourceB": {
                    "Type": "AWS::DynamoDB::Table",
                    "Properties": {
                        "Tags": [
                            {"Key": "RefA", "Value": {"Ref": "ResourceA"}}
                        ]
                    }
                }
            }
        });

        let mut id_mapping = HashMap::new();
        id_mapping.insert("ResourceA".to_string(), "ResourceA".to_string());
        id_mapping.insert("ResourceB".to_string(), "ResourceB".to_string());

        let result = validate_move_references(&template, &id_mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_move_references_standalone_resource() {
        // Test: Resource has no dependencies - should succeed
        let template = json!({
            "Resources": {
                "StandaloneBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": "test-bucket"
                    }
                },
                "OtherBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {}
                }
            }
        });

        let mut id_mapping = HashMap::new();
        id_mapping.insert(
            "StandaloneBucket".to_string(),
            "StandaloneBucket".to_string(),
        );

        let result = validate_move_references(&template, &id_mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_move_references_move_all_dependencies_together() {
        // Test: Moving Instance + SecurityGroup + Role together - should succeed
        let template = json!({
            "Resources": {
                "Instance": {
                    "Type": "AWS::EC2::Instance",
                    "Properties": {
                        "SecurityGroupIds": [{"Ref": "SecurityGroup"}],
                        "IamInstanceProfile": {"Ref": "Role"}
                    }
                },
                "SecurityGroup": {
                    "Type": "AWS::EC2::SecurityGroup",
                    "Properties": {}
                },
                "Role": {
                    "Type": "AWS::IAM::Role",
                    "Properties": {}
                }
            }
        });

        let mut id_mapping = HashMap::new();
        id_mapping.insert("Instance".to_string(), "Instance".to_string());
        id_mapping.insert("SecurityGroup".to_string(), "SecurityGroup".to_string());
        id_mapping.insert("Role".to_string(), "Role".to_string());

        let result = validate_move_references(&template, &id_mapping);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_move_references_parameter_reference() {
        // Test: Resource references a parameter - should succeed (parameters are stack config, not resources)
        let template = json!({
            "Parameters": {
                "TableName": {
                    "Type": "String",
                    "Default": "my-table"
                }
            },
            "Resources": {
                "DynamoTable": {
                    "Type": "AWS::DynamoDB::Table",
                    "Properties": {
                        "TableName": {"Ref": "TableName"}
                    }
                },
                "OtherResource": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {}
                }
            }
        });

        let mut id_mapping = HashMap::new();
        id_mapping.insert("DynamoTable".to_string(), "DynamoTable".to_string());
        // Not moving OtherResource

        let result = validate_move_references(&template, &id_mapping);
        assert!(result.is_ok()); // Should succeed because TableName is a parameter, not a resource
    }

    #[test]
    fn test_parse_template_json() {
        // Test: Parse valid JSON template
        let json_template = r#"{
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": "test-bucket"
                    }
                }
            }
        }"#;

        let result = serde_json::from_str::<serde_json::Value>(json_template);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(
            parsed["AWSTemplateFormatVersion"].as_str(),
            Some("2010-09-09")
        );
        assert!(parsed["Resources"]["MyBucket"].is_object());
    }

    #[test]
    fn test_parse_template_yaml() {
        // Test: Parse valid YAML template without CloudFormation intrinsic function tags
        let yaml_template = r#"AWSTemplateFormatVersion: 2010-09-09
Description: "Creates an S3 bucket to store logs."

Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: test-bucket
"#;

        // JSON parsing should fail for YAML
        let json_result = serde_json::from_str::<serde_json::Value>(yaml_template);
        assert!(json_result.is_err());

        // YAML parsing should work
        let yaml_result = serde_yml::from_str::<serde_json::Value>(yaml_template);
        assert!(yaml_result.is_ok());
        let parsed = yaml_result.unwrap();
        assert_eq!(
            parsed["AWSTemplateFormatVersion"].as_str(),
            Some("2010-09-09")
        );
        assert!(parsed["Resources"]["MyBucket"].is_object());
    }

    #[test]
    fn test_parse_template_auto_detect_json() {
        // Test: Auto-detection of JSON format
        let json_template = r#"{"AWSTemplateFormatVersion": "2010-09-09", "Resources": {}}"#;

        // Try JSON first (should succeed)
        let result = serde_json::from_str::<serde_json::Value>(json_template);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_template_auto_detect_yaml() {
        // Test: Auto-detection of YAML format
        let yaml_template = "AWSTemplateFormatVersion: 2010-09-09\nResources: {}";

        // Try JSON first (should fail)
        let json_result = serde_json::from_str::<serde_json::Value>(yaml_template);
        assert!(json_result.is_err());

        // Fallback to YAML (should succeed)
        let yaml_result = serde_yml::from_str::<serde_json::Value>(yaml_template);
        assert!(yaml_result.is_ok());
    }

    #[test]
    fn test_parse_template_malformed() {
        // Test: Malformed template (neither valid JSON nor valid structured YAML)
        let malformed_template = "{[}]invalid";

        // JSON parser should fail
        let json_result = serde_json::from_str::<serde_json::Value>(malformed_template);
        assert!(json_result.is_err());
    }

    #[test]
    fn test_parse_template_yaml_with_cloudformation_structure() {
        // Test: YAML template with CloudFormation structure
        let yaml_template = r#"AWSTemplateFormatVersion: 2010-09-09
Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: bucket-name
"#;

        // YAML parser should handle basic YAML structure
        let result = serde_yml::from_str::<serde_json::Value>(yaml_template);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed["Resources"]["MyBucket"].is_object());
        assert_eq!(
            parsed["Resources"]["MyBucket"]["Type"].as_str(),
            Some("AWS::S3::Bucket")
        );
    }

    #[test]
    fn test_parse_template_yaml_with_cloudformation_intrinsic_functions() {
        // Test: YAML template with CloudFormation intrinsic function tags (!Ref, !Sub, !GetAtt)
        let yaml_template = r#"AWSTemplateFormatVersion: 2010-09-09
Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: !Sub '${AWS::StackName}-bucket'
  
  MyQueue:
    Type: AWS::SQS::Queue
    Properties:
      QueueName: !Ref MyBucket
  
  MyRole:
    Type: AWS::IAM::Role
    Properties:
      RoleName: !GetAtt MyBucket.Arn
"#;

        // Standard YAML parser (serde_yml) should fail with intrinsic function tags
        let serde_result = serde_yml::from_str::<serde_json::Value>(yaml_template);
        assert!(serde_result.is_err());

        // Our cfn_yaml parser should handle CloudFormation tags
        let cf_result = cfn_yaml::parse_yaml_to_json(yaml_template);
        assert!(
            cf_result.is_ok(),
            "Failed to parse CF YAML: {:?}",
            cf_result
        );
        let parsed = cf_result.unwrap();

        // Verify intrinsic functions are converted to long-form JSON
        assert!(parsed["Resources"]["MyBucket"]["Properties"]["BucketName"].is_object());
        assert!(parsed["Resources"]["MyBucket"]["Properties"]["BucketName"]["Fn::Sub"].is_string());

        assert!(parsed["Resources"]["MyQueue"]["Properties"]["QueueName"].is_object());
        assert!(parsed["Resources"]["MyQueue"]["Properties"]["QueueName"]["Ref"].is_string());

        assert!(parsed["Resources"]["MyRole"]["Properties"]["RoleName"].is_object());
        // !GetAtt with dot notation can be either string or array depending on parser
        let getatt_value = &parsed["Resources"]["MyRole"]["Properties"]["RoleName"]["Fn::GetAtt"];
        assert!(getatt_value.is_array() || getatt_value.is_string());
    }

    #[test]
    fn test_template_format_preservation_json() {
        // Test: Template preserves JSON format when serialized
        use serde_json::json;

        let json_content = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket"
                }
            }
        });

        let template = Template::new(json_content, TemplateFormat::Json);
        let result = template.to_string();

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should be JSON (compact, no newlines between properties)
        assert!(output.starts_with('{'));
        assert!(output.ends_with('}'));

        // Should be valid JSON
        let reparsed: Result<serde_json::Value, _> = serde_json::from_str(&output);
        assert!(reparsed.is_ok());
    }

    #[test]
    fn test_template_format_preservation_yaml() {
        // Test: Template preserves YAML format when serialized
        use serde_json::json;

        let yaml_content = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket"
                }
            }
        });

        let template = Template::new(yaml_content, TemplateFormat::Yaml);
        let result = template.to_string();

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should be YAML (contains newlines and indentation)
        assert!(output.contains('\n'));
        assert!(output.contains("AWSTemplateFormatVersion:"));
        assert!(output.contains("Resources:"));

        // Should be valid YAML
        let reparsed: Result<serde_json::Value, _> = serde_yml::from_str(&output);
        assert!(reparsed.is_ok());
    }

    #[test]
    fn test_template_serialization_error_handling() {
        // Test: Template::to_string() returns Result, not panicking
        use serde_json::json;

        // Normal case should succeed
        let valid_template = Template::new(json!({"key": "value"}), TemplateFormat::Json);
        assert!(valid_template.to_string().is_ok());

        // Note: It's hard to trigger serialization errors with serde_json/serde_yml
        // as they can serialize any valid JSON value, but we've verified the
        // error path exists by returning Result instead of unwrapping
    }

    // ========================================================================
    // Template File I/O Utility Tests
    // ========================================================================

    #[test]
    fn test_get_timestamp_format() {
        // Test: Timestamp is in YYYYMMDD-HHMMSS format
        let timestamp = get_timestamp();

        // Should be 15 characters: YYYYMMDD-HHMMSS
        assert_eq!(timestamp.len(), 15);

        // Should have dash in position 8
        assert_eq!(&timestamp[8..9], "-");

        // Should be all digits except the dash
        let without_dash: String = timestamp.chars().filter(|&c| c != '-').collect();
        assert!(without_dash.chars().all(|c| c.is_ascii_digit()));

        // Year should be reasonable (2020-2099)
        let year: u32 = timestamp[0..4].parse().unwrap();
        assert!(year >= 2020 && year <= 2099);
    }

    #[test]
    fn test_is_leap_year() {
        // Test known leap years
        assert!(is_leap_year(2020)); // Divisible by 4
        assert!(is_leap_year(2024)); // Divisible by 4
        assert!(is_leap_year(2000)); // Divisible by 400

        // Test non-leap years
        assert!(!is_leap_year(2021)); // Not divisible by 4
        assert!(!is_leap_year(2022)); // Not divisible by 4
        assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
        assert!(!is_leap_year(2100)); // Divisible by 100 but not 400
    }

    #[test]
    fn test_generate_filename() {
        // Test JSON format
        let filename = generate_filename("MyStack", "refactor", TemplateFormat::Json);
        assert!(filename.starts_with("MyStack-refactor-"));
        assert!(filename.ends_with(".json"));
        assert!(filename.contains('-')); // Contains timestamp

        // Test YAML format
        let filename = generate_filename("TestStack", "retain", TemplateFormat::Yaml);
        assert!(filename.starts_with("TestStack-retain-"));
        assert!(filename.ends_with(".yaml"));

        // Test different operations
        let filename = generate_filename("Stack1", "import", TemplateFormat::Json);
        assert!(filename.contains("Stack1-import-"));

        let filename = generate_filename("Stack2", "final", TemplateFormat::Yaml);
        assert!(filename.contains("Stack2-final-"));
    }

    #[test]
    fn test_resolve_file_collision_no_collision() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Test: File doesn't exist, should return base path
        let result = resolve_file_collision(dir_path, "test.yaml");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.file_name().unwrap(), "test.yaml");
    }

    #[test]
    fn test_resolve_file_collision_with_collision() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create existing file
        let existing_file = dir_path.join("test.yaml");
        fs::write(&existing_file, "existing content").unwrap();

        // Test: File exists, should return .1 variant
        let result = resolve_file_collision(dir_path, "test.yaml");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.file_name().unwrap(), "test.1.yaml");
    }

    #[test]
    fn test_resolve_file_collision_multiple_collisions() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create existing files: test.yaml, test.1.yaml, test.2.yaml
        fs::write(dir_path.join("test.yaml"), "v0").unwrap();
        fs::write(dir_path.join("test.1.yaml"), "v1").unwrap();
        fs::write(dir_path.join("test.2.yaml"), "v2").unwrap();

        // Test: Should return .3 variant
        let result = resolve_file_collision(dir_path, "test.yaml");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.file_name().unwrap(), "test.3.yaml");
    }

    #[test]
    fn test_resolve_file_collision_json_format() {
        use std::fs;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create existing JSON file
        fs::write(dir_path.join("stack.json"), "{}").unwrap();

        // Test: Should handle .json extension correctly
        let result = resolve_file_collision(dir_path, "stack.json");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path.file_name().unwrap(), "stack.1.json");
    }

    #[test]
    fn test_write_and_read_template_json() {
        use serde_json::json;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-template.json");

        // Create a test template
        let template_content = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket"
                }
            }
        });
        let template = Template::new(template_content.clone(), TemplateFormat::Json);

        // Write template to file
        let write_result = write_template_to_file(&template, &file_path);
        assert!(write_result.is_ok());

        // Verify file exists
        assert!(file_path.exists());

        // Read template back from file
        let read_result = read_template_from_file(&file_path);
        assert!(read_result.is_ok());
        let read_template = read_result.unwrap();

        // Verify format is preserved
        assert_eq!(read_template.format, TemplateFormat::Json);

        // Verify content is identical
        assert_eq!(read_template.content, template_content);
    }

    #[test]
    fn test_write_and_read_template_yaml() {
        use serde_json::json;
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-template.yaml");

        // Create a test template
        let template_content = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyTable": {
                    "Type": "AWS::DynamoDB::Table",
                    "Properties": {
                        "TableName": "TestTable"
                    }
                }
            }
        });
        let template = Template::new(template_content.clone(), TemplateFormat::Yaml);

        // Write template to file
        let write_result = write_template_to_file(&template, &file_path);
        assert!(write_result.is_ok());

        // Verify file exists
        assert!(file_path.exists());

        // Read template back from file
        let read_result = read_template_from_file(&file_path);
        assert!(read_result.is_ok());
        let read_template = read_result.unwrap();

        // Verify format is detected as YAML (file parsing detects from content, not extension)
        // Since we write as YAML, it should be read back as YAML
        assert_eq!(read_template.format, TemplateFormat::Yaml);

        // Verify content is identical
        assert_eq!(read_template.content, template_content);
    }

    #[test]
    fn test_read_template_from_file_not_found() {
        use std::path::PathBuf;

        let non_existent = PathBuf::from("/tmp/does-not-exist-12345.yaml");
        let result = read_template_from_file(&non_existent);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to open file"));
    }

    #[test]
    fn test_write_error_context() {
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let context_path = temp_dir.path().join("error-context.txt");

        // Create resource mappings
        let mut resources = HashMap::new();
        resources.insert("Bucket1".to_string(), "Bucket1New".to_string());
        resources.insert("Table1".to_string(), "Table1".to_string());

        // Write error context
        let result = write_error_context(
            &context_path,
            "Template validation failed",
            "SourceStack",
            Some("TargetStack"),
            "refactor",
            &resources,
        );

        assert!(result.is_ok());
        assert!(context_path.exists());

        // Read and verify content
        let content = std::fs::read_to_string(&context_path).unwrap();
        assert!(content.contains("Error: Template validation failed"));
        assert!(content.contains("Operation: refactor"));
        assert!(content.contains("Source Stack: SourceStack"));
        assert!(content.contains("Target Stack: TargetStack"));
        assert!(content.contains("Bucket1 -> Bucket1New"));
        assert!(content.contains("Table1 (no rename)"));
        assert!(content.contains("Full Error Details:"));
    }

    #[test]
    fn test_write_error_context_same_stack() {
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let context_path = temp_dir.path().join("error-context-same-stack.txt");

        // Create resource mappings
        let mut resources = HashMap::new();
        resources.insert("OldId".to_string(), "NewId".to_string());

        // Write error context (no target stack for same-stack operation)
        let result = write_error_context(
            &context_path,
            "AWS API error",
            "MyStack",
            None,
            "refactor",
            &resources,
        );

        assert!(result.is_ok());
        assert!(context_path.exists());

        // Read and verify content
        let content = std::fs::read_to_string(&context_path).unwrap();
        assert!(content.contains("Source Stack: MyStack"));
        assert!(!content.contains("Target Stack:")); // Should not have target stack line
        assert!(content.contains("OldId -> NewId"));
    }

    #[test]
    fn test_validate_output_directory_existing() {
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Validate existing directory
        let result = validate_output_directory(dir_path);
        assert!(result.is_ok());

        let validated_path = result.unwrap();
        assert!(validated_path.is_absolute());
        assert!(validated_path.is_dir());
    }

    #[test]
    fn test_validate_output_directory_creates_new() {
        use tempfile::TempDir;

        // Create temporary directory
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new_subdir").join("nested");

        // Directory should not exist yet
        assert!(!new_dir.exists());

        // Validate should create it
        let result = validate_output_directory(&new_dir);
        assert!(result.is_ok());

        // Directory should now exist
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_validate_output_directory_file_not_directory() {
        use tempfile::NamedTempFile;

        // Create a temporary file (not a directory)
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path();

        // Attempting to validate a file as a directory should fail
        let result = validate_output_directory(file_path);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not a directory"));
    }

    #[test]
    fn test_validate_output_directory_relative_path() {
        use std::env;
        use tempfile::TempDir;

        // Create temporary directory and change to it
        let temp_dir = TempDir::new().unwrap();
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        // Validate relative path
        let rel_path = Path::new("./test_output");
        let result = validate_output_directory(rel_path);

        // Restore original directory
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
        let validated_path = result.unwrap();
        assert!(validated_path.is_absolute());
    }

    #[test]
    fn test_parse_migration_spec_valid() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary migration spec file
        let mut temp_file = NamedTempFile::new().unwrap();
        let spec_content = r#"{
            "resources": {
                "OldBucket": "NewBucket",
                "OldTable": "NewTable",
                "SameNameResource": "SameNameResource"
            }
        }"#;
        temp_file.write_all(spec_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Parse migration spec
        let result = parse_migration_spec(temp_file.path());
        assert!(result.is_ok());

        let mappings = result.unwrap();
        assert_eq!(mappings.len(), 3);
        assert_eq!(mappings.get("OldBucket"), Some(&"NewBucket".to_string()));
        assert_eq!(mappings.get("OldTable"), Some(&"NewTable".to_string()));
        assert_eq!(
            mappings.get("SameNameResource"),
            Some(&"SameNameResource".to_string())
        );
    }

    #[test]
    fn test_parse_migration_spec_file_not_found() {
        use std::path::PathBuf;

        let non_existent = PathBuf::from("/tmp/migration-spec-does-not-exist.json");
        let result = parse_migration_spec(&non_existent);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read migration spec file"));
    }

    #[test]
    fn test_parse_migration_spec_invalid_json() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with invalid JSON
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"{ invalid json }").unwrap();
        temp_file.flush().unwrap();

        let result = parse_migration_spec(temp_file.path());
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse migration spec as JSON"));
    }

    #[test]
    fn test_parse_migration_spec_missing_resources_field() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file without "resources" field
        let mut temp_file = NamedTempFile::new().unwrap();
        let spec_content = r#"{ "other_field": {} }"#;
        temp_file.write_all(spec_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = parse_migration_spec(temp_file.path());
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must contain a 'resources' object"));
    }

    #[test]
    fn test_parse_migration_spec_non_string_value() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with non-string value
        let mut temp_file = NamedTempFile::new().unwrap();
        let spec_content = r#"{
            "resources": {
                "Resource1": "ValidString",
                "Resource2": 123
            }
        }"#;
        temp_file.write_all(spec_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = parse_migration_spec(temp_file.path());
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("mapping value must be a string"));
    }

    #[test]
    fn test_parse_migration_spec_empty_resources() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create temporary file with empty resources object
        let mut temp_file = NamedTempFile::new().unwrap();
        let spec_content = r#"{ "resources": {} }"#;
        temp_file.write_all(spec_content.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let result = parse_migration_spec(temp_file.path());
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("contains no resource mappings"));
    }

    #[test]
    fn test_validate_migration_spec_resources_valid() {
        // Create template with resources
        let template_json = serde_json::json!({
            "Resources": {
                "Bucket1": { "Type": "AWS::S3::Bucket" },
                "Table1": { "Type": "AWS::DynamoDB::Table" },
                "Instance1": { "Type": "AWS::EC2::Instance" }
            }
        });
        let template = Template::new(template_json, TemplateFormat::Json);

        // Create mappings with valid resource IDs
        let mut mappings = HashMap::new();
        mappings.insert("Bucket1".to_string(), "NewBucket".to_string());
        mappings.insert("Table1".to_string(), "Table1".to_string());

        // Validation should succeed
        let result = validate_migration_spec_resources(&mappings, &template);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_migration_spec_resources_invalid_id() {
        // Create template with resources
        let template_json = serde_json::json!({
            "Resources": {
                "Bucket1": { "Type": "AWS::S3::Bucket" },
                "Table1": { "Type": "AWS::DynamoDB::Table" }
            }
        });
        let template = Template::new(template_json, TemplateFormat::Json);

        // Create mappings with invalid resource ID
        let mut mappings = HashMap::new();
        mappings.insert("NonExistent".to_string(), "NewName".to_string());
        mappings.insert("Table1".to_string(), "Table1".to_string());

        // Validation should fail
        let result = validate_migration_spec_resources(&mappings, &template);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("do not exist in the source template"));
        assert!(err_msg.contains("NonExistent"));
    }

    #[test]
    fn test_validate_migration_spec_resources_template_without_resources() {
        // Create template without Resources section
        let template_json = serde_json::json!({
            "AWSTemplateFormatVersion": "2010-09-09"
        });
        let template = Template::new(template_json, TemplateFormat::Json);

        // Create mappings
        let mut mappings = HashMap::new();
        mappings.insert("Bucket1".to_string(), "NewBucket".to_string());

        // Validation should fail
        let result = validate_migration_spec_resources(&mappings, &template);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("does not contain a Resources section"));
    }

    // ============================================================================
    // Integration/Workflow Tests
    // ============================================================================

    #[test]
    fn test_workflow_template_file_collision_handling() {
        // T041: Test file collision handling with actual files
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create a template
        let template_json = serde_json::json!({
            "Resources": {
                "Bucket1": { "Type": "AWS::S3::Bucket" }
            }
        });
        let template = Template::new(template_json, TemplateFormat::Json);

        // Write the first template (no collision)
        let path1 = export_templates(
            &[(template.clone(), "source".to_string())],
            Some(&dir_path.to_path_buf()),
            "TestStack",
            "refactor",
        )
        .unwrap()[0]
            .clone();

        assert!(path1.exists());
        assert_eq!(
            path1
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("TestStack-refactor-source-"),
            true
        );
        assert_eq!(path1.extension().unwrap(), "json");

        // Write the second template (should create .1 variant)
        let path2 = export_templates(
            &[(template.clone(), "source".to_string())],
            Some(&dir_path.to_path_buf()),
            "TestStack",
            "refactor",
        )
        .unwrap()[0]
            .clone();

        assert!(path2.exists());
        assert_ne!(path1, path2);
        assert!(path2
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(".1.json"));

        // Write the third template (should create .2 variant)
        let path3 = export_templates(
            &[(template.clone(), "source".to_string())],
            Some(&dir_path.to_path_buf()),
            "TestStack",
            "refactor",
        )
        .unwrap()[0]
            .clone();

        assert!(path3.exists());
        assert_ne!(path1, path3);
        assert_ne!(path2, path3);
        assert!(path3
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(".2.json"));
    }

    #[test]
    fn test_workflow_export_multiple_templates() {
        // T037/T038: Test exporting multiple templates in one operation
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create different templates
        let template1_json = serde_json::json!({
            "Resources": {
                "Bucket1": { "Type": "AWS::S3::Bucket" }
            }
        });
        let template1 = Template::new(template1_json, TemplateFormat::Json);

        let template2_json = serde_json::json!({
            "Resources": {
                "Table1": { "Type": "AWS::DynamoDB::Table" }
            }
        });
        let template2 = Template::new(template2_json, TemplateFormat::Yaml);

        // Export multiple templates (simulating import mode with 4 templates)
        let paths = export_templates(
            &[
                (template1.clone(), "source-retained".to_string()),
                (template1.clone(), "source-removed".to_string()),
                (template2.clone(), "target-with-retention".to_string()),
                (template2.clone(), "target-final".to_string()),
            ],
            Some(&dir_path.to_path_buf()),
            "ImportStack",
            "import",
        )
        .unwrap();

        // Verify 4 files were created
        assert_eq!(paths.len(), 4);

        // Verify all files exist and have correct extensions
        assert!(paths[0].exists());
        assert!(paths[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("source-retained"));
        assert_eq!(paths[0].extension().unwrap(), "json");

        assert!(paths[1].exists());
        assert!(paths[1]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("source-removed"));
        assert_eq!(paths[1].extension().unwrap(), "json");

        assert!(paths[2].exists());
        assert!(paths[2]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("target-with-retention"));
        assert_eq!(paths[2].extension().unwrap(), "yaml");

        assert!(paths[3].exists());
        assert!(paths[3]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("target-final"));
        assert_eq!(paths[3].extension().unwrap(), "yaml");

        // Verify file contents are readable
        let read_back1 = read_template_from_file(&paths[0]).unwrap();
        assert_eq!(read_back1.format, TemplateFormat::Json);

        let read_back2 = read_template_from_file(&paths[2]).unwrap();
        assert_eq!(read_back2.format, TemplateFormat::Yaml);
    }

    #[test]
    fn test_workflow_roundtrip_template_json() {
        // T027: Test template input workflow - write and read back JSON template
        use tempfile::NamedTempFile;

        let template_json = serde_json::json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": "my-test-bucket"
                    }
                },
                "MyTable": {
                    "Type": "AWS::DynamoDB::Table",
                    "Properties": {
                        "TableName": "my-test-table",
                        "AttributeDefinitions": [
                            { "AttributeName": "id", "AttributeType": "S" }
                        ],
                        "KeySchema": [
                            { "AttributeName": "id", "KeyType": "HASH" }
                        ]
                    }
                }
            }
        });

        let original = Template::new(template_json, TemplateFormat::Json);

        // Write template to file
        let temp_file = NamedTempFile::new().unwrap();
        write_template_to_file(&original, temp_file.path()).unwrap();

        // Read template back
        let read_back = read_template_from_file(temp_file.path()).unwrap();

        // Verify format preserved
        assert_eq!(read_back.format, TemplateFormat::Json);

        // Verify content matches
        assert_eq!(
            read_back
                .content
                .get("AWSTemplateFormatVersion")
                .unwrap()
                .as_str()
                .unwrap(),
            "2010-09-09"
        );

        let resources = read_back
            .content
            .get("Resources")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(resources.contains_key("MyBucket"));
        assert!(resources.contains_key("MyTable"));

        let bucket = resources.get("MyBucket").unwrap().as_object().unwrap();
        assert_eq!(
            bucket.get("Type").unwrap().as_str().unwrap(),
            "AWS::S3::Bucket"
        );
    }

    #[test]
    fn test_workflow_roundtrip_template_yaml() {
        // T027: Test template input workflow - write and read back YAML template
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let yaml_file = temp_dir.path().join("test-template.yaml");

        let template_json = serde_json::json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Description": "Test YAML template",
            "Resources": {
                "SecurityGroup": {
                    "Type": "AWS::EC2::SecurityGroup",
                    "Properties": {
                        "GroupDescription": "Test security group",
                        "SecurityGroupIngress": [
                            {
                                "IpProtocol": "tcp",
                                "FromPort": 80,
                                "ToPort": 80,
                                "CidrIp": "0.0.0.0/0"
                            }
                        ]
                    }
                }
            }
        });

        let original = Template::new(template_json, TemplateFormat::Yaml);

        // Write template to file
        write_template_to_file(&original, &yaml_file).unwrap();

        // Verify file exists and has .yaml extension
        assert!(yaml_file.exists());
        assert!(yaml_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".yaml"));

        // Read template back
        let read_back = read_template_from_file(&yaml_file).unwrap();

        // Verify format preserved as YAML
        assert_eq!(read_back.format, TemplateFormat::Yaml);

        // Verify content matches
        assert_eq!(
            read_back
                .content
                .get("Description")
                .unwrap()
                .as_str()
                .unwrap(),
            "Test YAML template"
        );

        let resources = read_back
            .content
            .get("Resources")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(resources.contains_key("SecurityGroup"));
    }

    #[test]
    fn test_workflow_migration_spec_end_to_end() {
        // T027: Test migration spec workflow - write spec file, read it back, validate against template
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create migration spec file
        let spec_content = r#"{
  "resources": {
    "OldBucket": "NewBucket",
    "OldTable": "NewTable",
    "OldInstance": "Instance1"
  }
}"#;

        let mut spec_file = NamedTempFile::new().unwrap();
        spec_file.write_all(spec_content.as_bytes()).unwrap();
        spec_file.flush().unwrap();

        // Parse migration spec
        let mappings = parse_migration_spec(spec_file.path()).unwrap();

        assert_eq!(mappings.len(), 3);
        assert_eq!(mappings.get("OldBucket").unwrap(), "NewBucket");
        assert_eq!(mappings.get("OldTable").unwrap(), "NewTable");
        assert_eq!(mappings.get("OldInstance").unwrap(), "Instance1");

        // Create matching template
        let template_json = serde_json::json!({
            "Resources": {
                "OldBucket": { "Type": "AWS::S3::Bucket" },
                "OldTable": { "Type": "AWS::DynamoDB::Table" },
                "OldInstance": { "Type": "AWS::EC2::Instance" }
            }
        });
        let template = Template::new(template_json, TemplateFormat::Json);

        // Validate spec against template
        let result = validate_migration_spec_resources(&mappings, &template);
        assert!(result.is_ok());
    }

    #[test]
    fn test_workflow_format_preservation_across_operations() {
        // T052: Verify format preservation through export and import
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Test JSON preservation
        let json_template = Template::new(
            serde_json::json!({"Resources": {"Bucket1": {"Type": "AWS::S3::Bucket"}}}),
            TemplateFormat::Json,
        );

        let json_paths = export_templates(
            &[(json_template, "source".to_string())],
            Some(&temp_dir.path().to_path_buf()),
            "JsonStack",
            "refactor",
        )
        .unwrap();

        let json_read_back = read_template_from_file(&json_paths[0]).unwrap();
        assert_eq!(json_read_back.format, TemplateFormat::Json);
        assert_eq!(json_paths[0].extension().unwrap(), "json");

        // Test YAML preservation
        let yaml_template = Template::new(
            serde_json::json!({"Resources": {"Table1": {"Type": "AWS::DynamoDB::Table"}}}),
            TemplateFormat::Yaml,
        );

        let yaml_paths = export_templates(
            &[(yaml_template, "target".to_string())],
            Some(&temp_dir.path().to_path_buf()),
            "YamlStack",
            "import",
        )
        .unwrap();

        let yaml_read_back = read_template_from_file(&yaml_paths[0]).unwrap();
        assert_eq!(yaml_read_back.format, TemplateFormat::Yaml);
        assert_eq!(yaml_paths[0].extension().unwrap(), "yaml");
    }

    #[test]
    fn test_workflow_error_context_generation() {
        // T049: Test error context file generation workflow
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let error_file = temp_dir.path().join("migration-error.txt");

        let error_msg =
            "Template validation failed: Missing required property 'Type' in resource 'MyBucket'";
        let source_stack = "FailedStack";
        let target_stack = Some("TargetStack");
        let operation = "refactor";

        // Create resource mappings
        let mut resources = HashMap::new();
        resources.insert("MyBucket".to_string(), "NewBucket".to_string());
        resources.insert("MyTable".to_string(), "MyTable".to_string());

        // Write error context
        let result = write_error_context(
            &error_file,
            error_msg,
            source_stack,
            target_stack,
            operation,
            &resources,
        );

        assert!(result.is_ok());

        // Verify file exists
        assert!(error_file.exists());

        // Verify file content
        let content = std::fs::read_to_string(&error_file).unwrap();
        assert!(content.contains("Error: Template validation failed"));
        assert!(content.contains("Operation: refactor"));
        assert!(content.contains("Source Stack: FailedStack"));
        assert!(content.contains("Target Stack: TargetStack"));
        assert!(content.contains("MyBucket -> NewBucket"));
        assert!(content.contains("MyTable (no rename)"));
        assert!(content.contains("Full Error Details:"));
    }

    // ============================================================================
    // CLI Argument Validation Tests (T059)
    // ============================================================================
    // Note: These test the validation logic extracted from run() function.
    // The actual validations occur at lines 132-157 in run().

    /// Helper function to test CLI argument validation logic
    /// Mirrors the validation checks in run() function (lines 132-157)
    fn validate_cli_args(
        export: bool,
        source_template: bool,
        target_template: bool,
        migration_spec: bool,
        resource: bool,
    ) -> Result<(), String> {
        // Validation 1: Cannot use --export with --source-template
        if export && source_template {
            return Err("Cannot use --export with --source-template.\n\
                 Export mode fetches templates from AWS and writes them to disk.\n\
                 If you already have template files, you don't need export mode."
                .to_string());
        }

        // Validation 2: Cannot use --export with --target-template
        if export && target_template {
            return Err("Cannot use --export with --target-template.\n\
                 Export mode fetches templates from AWS and writes them to disk.\n\
                 If you already have template files, you don't need export mode."
                .to_string());
        }

        // Validation 3: Cannot use --migration-spec with --resource
        if migration_spec && resource {
            return Err(
                "Cannot use --migration-spec with --resource.\n\
                 The migration spec file defines resource mappings, so the --resource flag is not needed."
                    .to_string(),
            );
        }

        Ok(())
    }

    #[test]
    fn test_cli_validation_export_with_source_template() {
        // T059: Test --export + --source-template conflict
        let result = validate_cli_args(
            true,  // export
            true,  // source_template
            false, // target_template
            false, // migration_spec
            false, // resource
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Cannot use --export with --source-template"));
    }

    #[test]
    fn test_cli_validation_export_with_target_template() {
        // T059: Test --export + --target-template conflict
        let result = validate_cli_args(
            true,  // export
            false, // source_template
            true,  // target_template
            false, // migration_spec
            false, // resource
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Cannot use --export with --target-template"));
    }

    #[test]
    fn test_cli_validation_migration_spec_with_resource() {
        // T059: Test --migration-spec + --resource conflict
        let result = validate_cli_args(
            false, // export
            false, // source_template
            false, // target_template
            true,  // migration_spec
            true,  // resource
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Cannot use --migration-spec with --resource"));
    }

    #[test]
    fn test_cli_validation_export_only() {
        // T059: Test --export alone is valid
        let result = validate_cli_args(
            true,  // export
            false, // source_template
            false, // target_template
            false, // migration_spec
            false, // resource
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_validation_source_template_only() {
        // T059: Test --source-template alone is valid
        let result = validate_cli_args(
            false, // export
            true,  // source_template
            false, // target_template
            false, // migration_spec
            false, // resource
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_validation_migration_spec_only() {
        // T059: Test --migration-spec alone is valid
        let result = validate_cli_args(
            false, // export
            false, // source_template
            false, // target_template
            true,  // migration_spec
            false, // resource
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_validation_export_with_migration_spec() {
        // T059: Test --export + --migration-spec is valid (both allowed together)
        let result = validate_cli_args(
            true,  // export
            false, // source_template
            false, // target_template
            true,  // migration_spec
            false, // resource
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_validation_templates_with_migration_spec() {
        // T059: Test --source-template + --migration-spec is valid
        let result = validate_cli_args(
            false, // export
            true,  // source_template
            true,  // target_template
            true,  // migration_spec
            false, // resource
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_validation_all_conflicts() {
        // T059: Test multiple conflicts at once (export takes precedence)
        let result = validate_cli_args(
            true, // export
            true, // source_template
            true, // target_template
            true, // migration_spec
            true, // resource
        );

        assert!(result.is_err());
        // Should catch the first validation (export + source_template)
        let err = result.unwrap_err();
        assert!(err.contains("Cannot use --export with --source-template"));
    }
}
