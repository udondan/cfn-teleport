use aws_config::BehaviorVersion;
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use clap::{Parser, ValueEnum};
use console::style;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
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
    #[arg(long, value_enum, default_value = "refactor")]
    mode: Mode,

    /// Write generated templates to disk instead of executing (--dry-run)
    #[arg(long)]
    dry_run: bool,

    /// Directory to write templates to (used with --dry-run and on errors; defaults to current directory)
    #[arg(long, value_name = "PATH")]
    template_dir: Option<String>,

    /// Path to a file containing the source stack template (instead of fetching from AWS)
    #[arg(long = "source-template", value_name = "FILE")]
    source_template_file: Option<String>,

    /// Path to a file containing the target stack template (instead of fetching from AWS)
    #[arg(long = "target-template", value_name = "FILE")]
    target_template_file: Option<String>,
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
    let source_template = match &args.source_template_file {
        Some(path) => load_template_from_file(path)?,
        None => get_template(&client, &source_stack).await?,
    };

    // Determine if this is a cross-stack move or same-stack rename
    let is_cross_stack = source_stack != target_stack;

    // Fetch target template early for cross-stack parameter validation
    let target_template = if is_cross_stack {
        Some(match &args.target_template_file {
            Some(path) => load_template_from_file(path)?,
            None => get_template(&client, &target_stack).await?,
        })
    } else {
        None
    };

    let selected_resources = match args.resource.clone() {
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
    };

    if selected_resources.is_empty() {
        return Err("No resources have been selected".into());
    }

    let mut new_logical_ids_map = HashMap::new();
    //let mut resource_has_been_renamed = false;

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

    // Resolve the directory for writing templates (defaults to current directory).
    let template_dir_path = PathBuf::from(args.template_dir.as_deref().unwrap_or("."));

    let template_source_str = template_source.to_string()?;

    // Validate that resources being moved don't have dangling references
    // (i.e., resources staying in source stack that reference moving resources)
    // Only validate for cross-stack moves, not same-stack renames
    if source_stack != target_stack {
        validate_move_references(&template_source.content, &new_logical_ids_map)?;
    }

    // Same-stack rename: Use CloudFormation Stack Refactoring API
    if source_stack == target_stack {
        return refactor_stack_resources(
            &client,
            &source_stack,
            template_source,
            new_logical_ids_map,
            args.dry_run,
            &template_dir_path,
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
            get_template(&client, &target_stack).await?
        };
        return refactor_stack_resources_cross_stack(
            &client,
            &source_stack,
            &target_stack,
            template_source,
            template_target,
            new_logical_ids_map,
            args.dry_run,
            &template_dir_path,
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
        get_template(&client, &target_stack).await?
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
            return Err(format!(
                "Unable to proceed, because the template is invalid: {}",
                result.err().unwrap()
            )
            .into());
        }
    }

    // Build the list of templates to save (used for --dry-run and on error).
    let templates_to_save: Vec<(String, Template)> = vec![
        (
            format!("{}-retained", source_stack),
            Template::new(template_retained.clone(), template_source.format),
        ),
        (
            format!("{}-removed", source_stack),
            Template::new(template_removed.clone(), template_source.format),
        ),
        (
            format!("{}-import", target_stack),
            Template::new(
                template_target_with_deletion_policy.clone(),
                target_template_actual.format,
            ),
        ),
        (
            format!("{}-final", target_stack),
            Template::new(template_target.clone(), target_template_actual.format),
        ),
    ];

    if args.dry_run {
        println!("Dry run – writing templates to disk:");
        write_templates(&template_dir_path, &templates_to_save);
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

    let exec_result: Result<(), Box<dyn Error>> = async {
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
    .await;

    if exec_result.is_err() {
        eprintln!("\nAn error occurred. Writing templates to disk for recovery:");
        write_templates(&template_dir_path, &templates_to_save);
    }

    exec_result
}

fn split_ids(id: String) -> (String, String) {
    if id.contains(&":".to_string()) {
        let parts: Vec<String> = id.split(':').map(String::from).collect();
        (parts[0].clone(), parts[1].clone())
    } else {
        (id.clone(), id)
    }
}

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
) -> Result<Template, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template_str = resp.template_body().ok_or("No template found")?;
    parse_template_str(template_str)
}

/// Parse a CloudFormation template string (JSON or YAML with CF intrinsic function support).
fn parse_template_str(template_str: &str) -> Result<Template, Box<dyn Error>> {
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

/// Load and parse a CloudFormation template from a file on disk.
fn load_template_from_file(path: &str) -> Result<Template, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read template file '{}': {}", path, e))?;
    parse_template_str(&content)
}

/// Return an available file path, appending `.1`, `.2`, … suffixes to avoid collisions.
fn safe_file_path(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let base = dir.join(format!("{}.{}", name, ext));
    if !base.exists() {
        return base;
    }
    let mut suffix = 1u32;
    loop {
        let candidate = dir.join(format!("{}.{}.{}", name, ext, suffix));
        if !candidate.exists() {
            return candidate;
        }
        suffix += 1;
    }
}

/// Write a template to disk with collision detection and print the resulting path.
fn write_template(dir: &Path, name: &str, template: &Template) -> Result<PathBuf, Box<dyn Error>> {
    let ext = match template.format {
        TemplateFormat::Json => "json",
        TemplateFormat::Yaml => "yaml",
    };
    let path = safe_file_path(dir, name, ext);
    let content = template.to_string()?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Failed to write template to '{}': {}", path.display(), e))?;
    println!("  Written: {}", path.display());
    Ok(path)
}

/// Write a collection of (name, template) pairs to disk, printing warnings for failures.
fn write_templates(dir: &Path, templates: &[(String, Template)]) {
    for (name, tmpl) in templates {
        if let Err(e) = write_template(dir, name, tmpl) {
            eprintln!("  Warning: Failed to write '{}': {}", name, e);
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
    dry_run: bool,
    template_dir: &Path,
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

    let updated_template_obj = Template::new(updated_template.clone(), template.format);

    if dry_run {
        println!("Dry run – writing templates to disk:");
        write_template(
            template_dir,
            &format!("{}-final", stack_name),
            &updated_template_obj,
        )?;
        return Ok(());
    }

    // Validate the updated template
    if let Err(e) = validate_template(client, updated_template.clone())
        .await
        .map_err(|e| format!("Updated template validation failed: {}", e))
    {
        eprintln!("\nAn error occurred. Writing templates to disk for recovery:");
        let _ = write_template(
            template_dir,
            &format!("{}-final", stack_name),
            &updated_template_obj,
        );
        return Err(e.into());
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

    let exec_result: Result<(), Box<dyn Error>> = async {
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
    .await;

    if exec_result.is_err() {
        eprintln!("\nAn error occurred. Writing templates to disk for recovery:");
        let _ = write_template(
            template_dir,
            &format!("{}-final", stack_name),
            &updated_template_obj,
        );
    }

    exec_result
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
/// * `dry_run` - If true, write templates to disk and return without executing
/// * `template_dir` - Directory to write templates to on dry-run or error
///
/// # Returns
/// * `Ok(())` if refactoring succeeds
/// * `Err` if validation or execution fails
#[allow(clippy::too_many_arguments)]
async fn refactor_stack_resources_cross_stack(
    client: &cloudformation::Client,
    source_stack_name: &str,
    target_stack_name: &str,
    source_template: Template,
    target_template: Template,
    id_mapping: HashMap<String, String>,
    dry_run: bool,
    template_dir: &Path,
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

    let source_template_obj = Template::new(source_final.clone(), source_template.format);
    let target_template_obj = Template::new(target_final.clone(), target_template.format);

    let templates_to_save: Vec<(String, Template)> = vec![
        (
            format!("{}-final", source_stack_name),
            source_template_obj.clone(),
        ),
        (
            format!("{}-final", target_stack_name),
            target_template_obj.clone(),
        ),
    ];

    if dry_run {
        println!("Dry run – writing templates to disk:");
        write_templates(template_dir, &templates_to_save);
        return Ok(());
    }

    // Step 4: Validate both templates
    let validate_result = async {
        validate_template(client, source_final.clone())
            .await
            .map_err(|e| format!("Source template validation failed: {}", e))?;
        validate_template(client, target_final.clone())
            .await
            .map_err(|e| format!("Target template validation failed: {}", e))?;
        Ok::<(), Box<dyn Error>>(())
    }
    .await;
    if validate_result.is_err() {
        eprintln!("\nAn error occurred. Writing templates to disk for recovery:");
        write_templates(template_dir, &templates_to_save);
        return validate_result;
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
        .template_body(source_template_obj.to_string()?)
        .build();

    let target_stack_definition = StackDefinition::builder()
        .stack_name(target_stack_name)
        .template_body(target_template_obj.to_string()?)
        .build();

    let exec_result: Result<(), Box<dyn Error>> = async {
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
    .await;

    if exec_result.is_err() {
        eprintln!("\nAn error occurred. Writing templates to disk for recovery:");
        write_templates(template_dir, &templates_to_save);
    }

    exec_result
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

    #[test]
    fn test_safe_file_path_no_collision() {
        let dir = std::env::temp_dir();
        let name = format!("cfn_teleport_test_{}", uuid::Uuid::new_v4());
        let path = safe_file_path(&dir, &name, "json");
        assert_eq!(path, dir.join(format!("{}.json", name)));
    }

    #[test]
    fn test_safe_file_path_collision() {
        let dir = std::env::temp_dir();
        let name = format!("cfn_teleport_test_{}", uuid::Uuid::new_v4());
        // Create the base file to force a collision
        let base = dir.join(format!("{}.json", name));
        std::fs::write(&base, "{}").unwrap();

        let path = safe_file_path(&dir, &name, "json");
        assert_eq!(path, dir.join(format!("{}.json.1", name)));

        // Cleanup
        std::fs::remove_file(base).unwrap();
    }

    #[test]
    fn test_safe_file_path_multiple_collisions() {
        let dir = std::env::temp_dir();
        let name = format!("cfn_teleport_test_{}", uuid::Uuid::new_v4());
        let base = dir.join(format!("{}.json", name));
        let v1 = dir.join(format!("{}.json.1", name));
        std::fs::write(&base, "{}").unwrap();
        std::fs::write(&v1, "{}").unwrap();

        let path = safe_file_path(&dir, &name, "json");
        assert_eq!(path, dir.join(format!("{}.json.2", name)));

        // Cleanup
        std::fs::remove_file(base).unwrap();
        std::fs::remove_file(v1).unwrap();
    }

    #[test]
    fn test_write_template_creates_file() {
        let dir = std::env::temp_dir();
        let name = format!("cfn_teleport_test_{}", uuid::Uuid::new_v4());
        let content = json!({"AWSTemplateFormatVersion": "2010-09-09", "Resources": {}});
        let template = Template::new(content.clone(), TemplateFormat::Json);

        let path = write_template(&dir, &name, &template).unwrap();
        assert!(path.exists());

        let written = std::fs::read_to_string(&path).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(reparsed, content);

        // Cleanup
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_write_template_collision_detection() {
        let dir = std::env::temp_dir();
        let name = format!("cfn_teleport_test_{}", uuid::Uuid::new_v4());
        let template = Template::new(json!({"Resources": {}}), TemplateFormat::Json);

        let path1 = write_template(&dir, &name, &template).unwrap();
        let path2 = write_template(&dir, &name, &template).unwrap();

        assert_ne!(path1, path2);
        assert!(path1.ends_with(format!("{}.json", name)));
        assert!(path2.ends_with(format!("{}.json.1", name)));

        // Cleanup
        std::fs::remove_file(path1).unwrap();
        std::fs::remove_file(path2).unwrap();
    }

    #[test]
    fn test_load_template_from_file_json() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cfn_teleport_test_{}.json", uuid::Uuid::new_v4()));
        let content = r#"{"AWSTemplateFormatVersion":"2010-09-09","Resources":{}}"#;
        std::fs::write(&path, content).unwrap();

        let template = load_template_from_file(path.to_str().unwrap()).unwrap();
        assert!(matches!(template.format, TemplateFormat::Json));
        assert_eq!(
            template.content["AWSTemplateFormatVersion"].as_str(),
            Some("2010-09-09")
        );

        // Cleanup
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_template_from_file_yaml() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("cfn_teleport_test_{}.yaml", uuid::Uuid::new_v4()));
        let content = "AWSTemplateFormatVersion: '2010-09-09'\nResources: {}\n";
        std::fs::write(&path, content).unwrap();

        let template = load_template_from_file(path.to_str().unwrap()).unwrap();
        assert!(matches!(template.format, TemplateFormat::Yaml));
        assert_eq!(
            template.content["AWSTemplateFormatVersion"].as_str(),
            Some("2010-09-09")
        );

        // Cleanup
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_load_template_from_file_not_found() {
        let result = load_template_from_file("/nonexistent/path/template.json");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read template file"));
    }

    #[test]
    fn test_parse_template_str_json() {
        let json_str = r#"{"AWSTemplateFormatVersion":"2010-09-09","Resources":{}}"#;
        let template = parse_template_str(json_str).unwrap();
        assert!(matches!(template.format, TemplateFormat::Json));
    }

    #[test]
    fn test_parse_template_str_yaml() {
        let yaml_str = "AWSTemplateFormatVersion: '2010-09-09'\nResources: {}\n";
        let template = parse_template_str(yaml_str).unwrap();
        assert!(matches!(template.format, TemplateFormat::Yaml));
    }
}
