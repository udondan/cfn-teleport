//! Checks that resources can be moved to another stack

use crate::dependencies::ResourceDependencyInfo;
use crate::reference_updater;
use crate::template_edit::parameter_names;
use aws_sdk_cloudformation::types::StackResourceSummary;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::error::Error;

/// Validates that resources being moved don't have dangling references.
///
/// When moving resources between stacks, ensures that:
/// 1. Resources staying in source stack don't reference moving resources
/// 2. If a resource references another, both must be moved together
/// 3. Outputs in source stack don't reference moving resources (outputs can't be moved)
pub(crate) fn validate_move_references(
    source_template: &Value,
    new_logical_ids_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let moving: HashSet<&str> = new_logical_ids_map.keys().map(String::as_str).collect();
    let parameters = parameter_names(source_template);
    let all_references = reference_updater::find_all_references(source_template);

    let mut errors = Vec::new();
    for (referencing, referenced_resources) in &all_references {
        errors.extend(referenced_resources.iter().filter_map(|referenced| {
            reference_problem(referencing, referenced, &moving, &parameters)
        }));
    }

    if errors.is_empty() {
        return Ok(());
    }

    Err(format!(
        "Cannot move resources due to dangling references:\n\n{}\n\n\
         Tip: You can move multiple resources together if they reference each other.\n\
         Tip: Same-stack renaming doesn't have this restriction.",
        errors.join("\n")
    )
    .into())
}

/// Describes why `referencing` -> `referenced` blocks the move, if it does
fn reference_problem(
    referencing: &str,
    referenced: &str,
    moving: &HashSet<&str>,
    parameters: &HashSet<String>,
) -> Option<String> {
    let referenced_moves = moving.contains(referenced);

    if referencing == "Outputs" {
        return referenced_moves.then(|| {
            format!(
                "  - Output section references resource '{}' which is being moved. \
                 Outputs cannot be moved between stacks. Please remove or update the output before moving the resource.",
                referenced
            )
        });
    }

    // Parameters are stack-level config, not resources
    if parameters.contains(referenced) {
        return None;
    }

    match (moving.contains(referencing), referenced_moves) {
        // Referencing resource stays, but referenced resource moves
        (false, true) => Some(format!(
            "  - Resource '{}' references '{}', but only '{}' is being moved. \
             Either move both resources together, or remove the reference before moving.",
            referencing, referenced, referenced
        )),
        // Referencing resource moves, but referenced resource stays
        (true, false) => Some(format!(
            "  - Resource '{}' depends on '{}' which is not being moved. \
             Either move both resources together, or remove the dependency before moving.",
            referencing, referenced
        )),
        _ => None,
    }
}

/// Checks a selection for a cross-stack move: resources referenced by outputs
/// and resources using parameters the target stack doesn't have can't be moved.
pub(crate) fn check_cross_stack_selection(
    selected: &[&StackResourceSummary],
    dependency_info: &HashMap<String, ResourceDependencyInfo>,
    template: &Value,
    target_template: Option<&Value>,
) -> Result<(), Box<dyn Error>> {
    let info = |resource: &StackResourceSummary| {
        &dependency_info[resource.logical_resource_id().unwrap_or_default()]
    };

    let blocked_by_outputs: Vec<String> = selected
        .iter()
        .filter(|resource| info(resource).referenced_by_outputs)
        .map(|resource| format!("  - {}", describe(resource)))
        .collect();

    let source_parameters = parameter_names(template);
    let target_parameters = target_template.map(parameter_names).unwrap_or_default();
    let all_references = reference_updater::find_all_references(template);

    let blocked_by_parameters: Vec<String> = selected
        .iter()
        .filter(|resource| info(resource).depends_on_parameters)
        .filter_map(|resource| {
            let missing = referenced_parameters(resource, &all_references, &source_parameters)
                .into_iter()
                .filter(|name| !target_parameters.contains(*name))
                .collect::<Vec<_>>();
            (!missing.is_empty()).then(|| {
                format!(
                    "  - {} - missing parameters: {}",
                    describe(resource),
                    missing.join(", ")
                )
            })
        })
        .collect();

    let mut error_messages = Vec::new();

    if !blocked_by_outputs.is_empty() {
        error_messages.push(format!(
            "Cannot select the following resources because they are referenced by stack outputs:\n{}\n\
             Outputs cannot be moved between stacks. Consider:\n\
             - Removing or updating the outputs before moving\n\
             - Using same-stack rename instead (references will be updated automatically)",
            blocked_by_outputs.join("\n")
        ));
    }

    if !blocked_by_parameters.is_empty() {
        error_messages.push(format!(
            "Cannot select the following resources because they depend on parameters not present in target stack:\n{}\n\
             Consider:\n\
             - Adding the required parameters to the target stack\n\
             - Removing parameter dependencies from these resources",
            blocked_by_parameters.join("\n")
        ));
    }

    if error_messages.is_empty() {
        Ok(())
    } else {
        Err(error_messages.join("\n\n").into())
    }
}

/// Import mode does NOT copy parameters from source to target stack.
/// Resources that depend on parameters would fail to import because the parameter
/// reference is invalid in the target stack, causing "No updates" errors
/// and leaving the resource in a broken state (deleted from source, not in target).
pub(crate) fn check_import_parameter_dependencies(
    selected: &[&StackResourceSummary],
    new_logical_ids_map: &HashMap<String, String>,
    template: &Value,
) -> Result<(), Box<dyn Error>> {
    let parameters = parameter_names(template);
    let all_references = reference_updater::find_all_references(template);

    let blocked: Vec<String> = selected
        .iter()
        .filter(|resource| {
            new_logical_ids_map.contains_key(resource.logical_resource_id().unwrap_or_default())
        })
        .filter_map(|resource| {
            let used = referenced_parameters(resource, &all_references, &parameters);
            (!used.is_empty()).then(|| {
                format!(
                    "  - {} - depends on parameters: {}\n",
                    describe(resource),
                    used.join(", ")
                )
            })
        })
        .collect();

    if blocked.is_empty() {
        return Ok(());
    }

    Err(format!(
        "Cannot use import mode for resources that depend on stack parameters:\n{}\n\
         Import mode does NOT copy parameters between stacks, which causes resources to be\n\
         deleted from source stack but fail to import to target stack, leaving them orphaned.\n\n\
         Use --mode refactor instead, or remove parameter dependencies from these resources.",
        blocked.concat()
    )
    .into())
}

/// "LogicalId (Type)"
fn describe(resource: &StackResourceSummary) -> String {
    format!(
        "{} ({})",
        resource.logical_resource_id().unwrap_or_default(),
        resource.resource_type().unwrap_or_default()
    )
}

/// The parameters a resource references
fn referenced_parameters<'a>(
    resource: &StackResourceSummary,
    all_references: &'a HashMap<String, HashSet<String>>,
    parameters: &HashSet<String>,
) -> Vec<&'a str> {
    all_references
        .get(resource.logical_resource_id().unwrap_or_default())
        .into_iter()
        .flatten()
        .filter(|name| parameters.contains(*name))
        .map(String::as_str)
        .collect()
}
