//! Choosing the resources to move or rename and their new logical IDs

use crate::cli::Args;
use crate::dependencies::{compute_dependency_markers, generate_legend};
use crate::display::format_resources;
use crate::migration_spec::{parse_migration_spec, validate_migration_spec_resources};
use crate::move_checks::check_cross_stack_selection;
use crate::prompt::Prompter;
use crate::stacks::{filter_resources, split_ids};
use crate::template::Template;
use aws_sdk_cloudformation::types::StackResourceSummary;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Source and target of the operation. The target template is only loaded for cross-stack moves.
pub(crate) struct Stacks {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) source_template: Template,
    pub(crate) target_template: Option<Template>,
}

impl Stacks {
    pub(crate) fn is_cross_stack(&self) -> bool {
        self.source != self.target
    }
}

/// Selected resources and their new logical IDs (old -> new)
pub(crate) type Selection<'a> = (Vec<&'a StackResourceSummary>, HashMap<String, String>);

/// Picks the resources and new IDs from the migration spec, the --resource flags, or prompts
pub(crate) async fn choose_resources<'a>(
    args: &Args,
    prompter: &dyn Prompter,
    stacks: &Stacks,
    resources: &'a [&'a StackResourceSummary],
) -> Result<Selection<'a>, Box<dyn Error>> {
    match (&args.migration_spec, &args.resource) {
        (Some(spec_path), _) => from_migration_spec(spec_path, resources, stacks).await,
        (None, Some(resource_args)) => from_resource_args(resource_args, resources, stacks).await,
        (None, None) => from_prompts(prompter, resources, stacks).await,
    }
}

async fn from_migration_spec<'a>(
    spec_path: &Path,
    resources: &'a [&'a StackResourceSummary],
    stacks: &Stacks,
) -> Result<Selection<'a>, Box<dyn Error>> {
    let mappings = parse_migration_spec(spec_path)?;
    validate_migration_spec_resources(&mappings, &stacks.source_template)?;

    let source_ids: Vec<String> = mappings.keys().cloned().collect();
    let selected = ensure_selected(filter_resources(resources, &source_ids).await?)?;
    Ok((selected, mappings))
}

async fn from_resource_args<'a>(
    resource_args: &[String],
    resources: &'a [&'a StackResourceSummary],
    stacks: &Stacks,
) -> Result<Selection<'a>, Box<dyn Error>> {
    let source_ids: Vec<String> = resource_args
        .iter()
        .map(|r| split_ids(r.clone()).0)
        .collect();

    let non_existing_ids: Vec<&str> = source_ids
        .iter()
        .map(String::as_str)
        .filter(|id| {
            !resources
                .iter()
                .any(|r| r.logical_resource_id().unwrap_or_default() == *id)
        })
        .collect();

    if !non_existing_ids.is_empty() {
        return Err(format!(
            "ERROR: The following resources do not exist on stack '{}':\n - {}",
            stacks.source,
            non_existing_ids.join("\n - "),
        )
        .into());
    }

    let selected = ensure_selected(filter_resources(resources, &source_ids).await?)?;
    let id_map = resource_args.iter().map(|r| split_ids(r.clone())).collect();
    Ok((selected, id_map))
}

async fn from_prompts<'a>(
    prompter: &dyn Prompter,
    resources: &'a [&'a StackResourceSummary],
    stacks: &Stacks,
) -> Result<Selection<'a>, Box<dyn Error>> {
    let selected = select_resources(
        prompter,
        resources,
        &stacks.source_template.content,
        stacks.target_template.as_ref().map(|t| &t.content),
        stacks.is_cross_stack(),
    )
    .await?;
    let selected = ensure_selected(selected)?;

    let mut id_map = HashMap::new();
    for resource in &selected {
        let old_id = resource.logical_resource_id().unwrap_or_default();
        let new_id = prompter.input(
            &format!(
                "Optionally provide a new logical ID for resource '{}'",
                old_id
            ),
            old_id,
        )?;
        let new_id = if new_id.is_empty() {
            old_id.to_string()
        } else {
            new_id
        };
        id_map.insert(old_id.to_string(), new_id);
    }

    Ok((selected, id_map))
}

fn ensure_selected(
    selected: Vec<&StackResourceSummary>,
) -> Result<Vec<&StackResourceSummary>, Box<dyn Error>> {
    if selected.is_empty() {
        return Err("No resources have been selected".into());
    }
    Ok(selected)
}

/// Lets the user pick resources from a list with dependency markers.
/// For cross-stack moves, resources that can't be moved are rejected.
pub(crate) async fn select_resources<'a>(
    prompter: &dyn Prompter,
    resources: &'a [&StackResourceSummary],
    template: &serde_json::Value,
    target_template: Option<&serde_json::Value>,
    is_cross_stack: bool,
) -> Result<Vec<&'a StackResourceSummary>, Box<dyn Error>> {
    let dependency_info = compute_dependency_markers(resources, template);

    if let Some(legend) = generate_legend(&dependency_info) {
        println!("{}\n", legend);
    }

    let items = format_resources(resources, None, Some(&dependency_info)).await?;

    let indices = prompter
        .multi_select("Select resources to copy", &items)?
        .ok_or("User did not select anything")?;
    let selected: Vec<_> = indices.into_iter().map(|index| resources[index]).collect();

    if is_cross_stack {
        check_cross_stack_selection(&selected, &dependency_info, template, target_template)?;
    }

    Ok(selected)
}
