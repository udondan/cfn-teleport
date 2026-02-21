use aws_config::BehaviorVersion;
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use clap::Parser;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::error::Error;
use uuid::Uuid;
mod spinner;
use std::collections::HashMap;
use std::io;
use std::process;
mod reference_updater;
mod supported_resource_types;

const DEMO: bool = false;

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

    /// Operation mode: 'refactor' (safe, atomic, fewer resource types) or 'import' (legacy, more resource types, can orphan resources)
    #[arg(long, value_name = "MODE", default_value = "refactor")]
    mode: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Validate mode parameter
    let mode = args.mode.to_lowercase();
    if mode != "refactor" && mode != "import" {
        return Err(format!(
            "Invalid mode '{}'. Must be 'refactor' or 'import'.",
            args.mode
        )
        .into());
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
                eprintln!("\nAWS credentials not found.\n");
                eprintln!("Please ensure you're authenticated with AWS using one of the following methods:");
                eprintln!("  • AWS CLI: Run 'aws configure'");
                eprintln!(
                    "  • Environment variables: Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY"
                );
                eprintln!("  • IAM role (if running on EC2/ECS/Lambda)");
                eprintln!("\nFor more information, visit:");
                eprintln!(
                    "  https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html\n"
                );
                process::exit(1);
            } else {
                // Handle other AWS errors cleanly
                let message = err.message().unwrap_or("An AWS error occurred");

                if let Some(code) = err.code() {
                    eprintln!("\nAWS Error ({}): {}\n", code, message);
                } else {
                    eprintln!("\n{}\n", message);
                }
                process::exit(1);
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
                eprintln!(
                    "ERROR: The following resources do not exist on stack '{}':\n - {}",
                    source_stack,
                    non_existing_ids.to_owned().join("\n - "),
                );
                process::exit(1);
            }
            filter_resources(resource_refs, &source_ids).await?
        }
        None => select_resources("Select resources to copy", resource_refs).await?,
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

    // Fetch template once and reuse
    let template_source = get_template(&client, &source_stack).await?;

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
        let existing_resources = if let Some(resources) = template_source.get("Resources") {
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

    for resource in format_resources(&selected_resources, Some(new_logical_ids_map.clone())).await?
    {
        println!("  {}", resource);
    }

    if !args.yes {
        user_confirm()?;
    }

    let template_source_str = serde_json::to_string(&template_source)?;

    // Validate that resources being moved don't have dangling references
    // (i.e., resources staying in source stack that reference moving resources)
    // Only validate for cross-stack moves, not same-stack renames
    if source_stack != target_stack {
        validate_move_references(&template_source, &new_logical_ids_map)?;
    }

    // Same-stack rename: Use CloudFormation Stack Refactoring API
    if source_stack == target_stack {
        return refactor_stack_resources(
            &client,
            &source_stack,
            template_source,
            new_logical_ids_map,
        )
        .await;
    }

    // Cross-stack move: Use refactor or import based on --mode
    if mode == "refactor" {
        // Use CloudFormation Stack Refactoring API (safer, atomic, but fewer supported resource types)
        let template_target = get_template(&client, &target_stack).await?;
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

    // Legacy import/export flow (mode == "import")
    let resource_ids_to_remove: Vec<_> = new_logical_ids_map.keys().cloned().collect();

    let template_retained =
        retain_resources(template_source.clone(), resource_ids_to_remove.clone());
    let template_retained_str = serde_json::to_string(&template_retained)?;

    let template_removed =
        remove_resources(template_source.clone(), resource_ids_to_remove.clone());

    let (template_target_with_deletion_policy, template_target) = add_resources(
        get_template(&client, &target_stack).await?,
        template_source.clone(),
        new_logical_ids_map.clone(),
    );

    // Update all resource references in the target templates
    let template_target_with_deletion_policy = reference_updater::update_template_references(
        template_target_with_deletion_policy,
        &new_logical_ids_map,
    );
    let template_target =
        reference_updater::update_template_references(template_target, &new_logical_ids_map);

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

    let spinner = spinner::Spin::new(
        format!(
            "Removing {} resources from stack {}",
            resource_ids_to_remove.len(),
            source_stack
        )
        .as_str(),
    );

    if template_source_str != template_retained_str {
        update_stack(&client, &source_stack, template_retained).await?;
        wait_for_stack_update_completion(&client, &source_stack, None).await?;
    }

    update_stack(&client, &source_stack, template_removed).await?;
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
        selected_resources,
        new_logical_ids_map,
    )
    .await?;

    wait_for_changeset_created(&client, &target_stack, &changeset_name).await?;
    execute_changeset(&client, &target_stack, &changeset_name).await?;
    wait_for_stack_update_completion(&client, &target_stack, None).await?;

    update_stack(&client, &target_stack, template_target).await?;
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
) -> Result<Vec<&'a aws_sdk_cloudformation::types::StackResourceSummary>, Box<dyn Error>> {
    let items = format_resources(resources, None).await?;
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .report(false)
        .items(&items)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(indices) => Ok(indices
            .into_iter()
            .map(|index| resources[index])
            .collect::<Vec<_>>()),
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
            let is_referenced_resource_moving = moving_resources.contains(referenced);

            // Problem: referencing resource stays, but referenced resource moves
            if !is_referencing_resource_moving && is_referenced_resource_moving {
                errors.push(format!(
                    "  - Resource '{}' references '{}', but only '{}' is being moved. \
                     Either move both resources together, or remove the reference before moving.",
                    referencing_resource, referenced, referenced
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
) -> Result<serde_json::Value, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    let parsed_template = serde_json::from_str(template)?;
    Ok(parsed_template)
}

async fn format_resources(
    resources: &[&cloudformation::types::StackResourceSummary],
    resource_id_map: Option<HashMap<String, String>>,
) -> Result<Vec<String>, io::Error> {
    let mut max_lengths = [0; 3];
    let mut formatted_resources = Vec::new();

    let mut renamed = false;

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

        let output = if renamed {
            let renamed = if logical_id != new_logical_id {
                format!(" ► {}", new_logical_id)
            } else {
                "".to_string()
            };
            format!(
                "{:<width1$}  {:<width2$}{:<width3$}   {}",
                resource_type,
                logical_id,
                renamed,
                physical_id,
                width1 = max_lengths[0] + 2,
                width2 = max_lengths[1],
                width3 = max_lengths[2] + 4
            )
        } else {
            format!(
                "{:<width1$}  {:<width2$}  {}",
                resource_type,
                logical_id,
                physical_id,
                width1 = max_lengths[0] + 2,
                width2 = max_lengths[1] + 2
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
    template: serde_json::Value,
    id_mapping: HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};

    // Step 1: Create updated template with renamed resources and updated references
    let mut updated_template = template.clone();

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
    validate_template(client, updated_template.clone())
        .await
        .map_err(|e| format!("Updated template validation failed: {}", e))?;

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
    let spinner = spinner::Spin::new(&format!(
        "Renaming {} resource{} in stack {}",
        id_mapping.len(),
        if id_mapping.len() == 1 { "" } else { "s" },
        stack_name,
    ));

    let stack_definition = StackDefinition::builder()
        .stack_name(stack_name)
        .template_body(serde_json::to_string(&updated_template)?)
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
                drop(spinner);
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
                drop(spinner);
                println!(
                    "✓ Renamed {} resource{} in stack {}",
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
                drop(spinner);
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
    source_template: serde_json::Value,
    target_template: serde_json::Value,
    id_mapping: HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};

    let resource_ids: Vec<String> = id_mapping.keys().cloned().collect();

    // Step 1: Remove resources from source template
    let source_without_resources = remove_resources(source_template.clone(), resource_ids.clone());

    // Step 2: Add resources to target template
    let (target_with_resources, _) = add_resources(
        target_template.clone(),
        source_template.clone(),
        id_mapping.clone(),
    );

    // Step 3: Update references in both templates
    let source_final =
        reference_updater::update_template_references(source_without_resources, &id_mapping);
    let target_final =
        reference_updater::update_template_references(target_with_resources, &id_mapping);

    // Step 4: Validate both templates
    validate_template(client, source_final.clone())
        .await
        .map_err(|e| format!("Source template validation failed: {}", e))?;

    validate_template(client, target_final.clone())
        .await
        .map_err(|e| format!("Target template validation failed: {}", e))?;

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
    let spinner = spinner::Spin::new(&format!(
        "Moving {} resource{} from {} to {}",
        id_mapping.len(),
        if id_mapping.len() == 1 { "" } else { "s" },
        source_stack_name,
        target_stack_name
    ));

    let source_stack_definition = StackDefinition::builder()
        .stack_name(source_stack_name)
        .template_body(serde_json::to_string(&source_final)?)
        .build();

    let target_stack_definition = StackDefinition::builder()
        .stack_name(target_stack_name)
        .template_body(serde_json::to_string(&target_final)?)
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
                drop(spinner);
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
                drop(spinner);
                println!(
                    "✓ Moved {} resource{} from {} to {}",
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
                drop(spinner);
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
) -> Result<(), cloudformation::Error> {
    match client
        .update_stack()
        .stack_name(stack_name)
        .template_body(serde_json::to_string(&template).unwrap())
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
    resources_to_import: Vec<&cloudformation::types::StackResourceSummary>,
    new_logical_ids_map: HashMap<String, String>,
) -> Result<std::string::String, cloudformation::Error> {
    let template_string = serde_json::to_string(&template).unwrap();
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
