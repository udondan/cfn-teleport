use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod collect;
mod update;

#[cfg(test)]
mod tests;

use collect::collect_references;
use update::traverse_and_update;

/// Main entry point for updating resource references in a CloudFormation template.
///
/// Takes a template and a mapping of old resource IDs to new resource IDs,
/// and updates all references (Ref, GetAtt, Sub, DependsOn) throughout the template.
///
/// # Arguments
/// * `template` - The CloudFormation template as a JSON Value
/// * `id_mapping` - HashMap mapping old resource IDs to new resource IDs
///
/// # Returns
/// The updated template with all references replaced
pub fn update_template_references(
    mut template: Value,
    id_mapping: &HashMap<String, String>,
) -> Value {
    for (old_id, new_id) in id_mapping {
        template = traverse_and_update(template, old_id, new_id);
    }
    template
}

/// Finds all resource references in a CloudFormation template.
///
/// Returns a map where:
/// - Key: The resource ID or "Outputs" that contains the reference
/// - Value: Set of resource IDs that are referenced
///
/// This is used for validating cross-stack moves - if a resource is being moved
/// but is still referenced by resources staying in the source stack, the move should fail.
///
/// # Arguments
/// * `template` - The CloudFormation template as a JSON Value
///
/// # Returns
/// HashMap mapping referencing resources to sets of referenced resources
pub fn find_all_references(template: &Value) -> HashMap<String, HashSet<String>> {
    let mut references: HashMap<String, HashSet<String>> = HashMap::new();

    // Scan Resources section
    if let Some(resources) = template.get("Resources").and_then(|r| r.as_object()) {
        for (resource_id, resource_def) in resources {
            let mut refs_in_resource = HashSet::new();
            collect_references(resource_def, &mut refs_in_resource);

            if !refs_in_resource.is_empty() {
                references.insert(resource_id.clone(), refs_in_resource);
            }
        }
    }

    // Scan Outputs section
    if let Some(outputs) = template.get("Outputs").and_then(|o| o.as_object()) {
        let mut refs_in_outputs = HashSet::new();
        for output_def in outputs.values() {
            collect_references(output_def, &mut refs_in_outputs);
        }

        if !refs_in_outputs.is_empty() {
            references.insert("Outputs".to_string(), refs_in_outputs);
        }
    }

    references
}

/// Checks if a resource name is a CloudFormation pseudo-parameter
fn is_pseudo_parameter(name: &str) -> bool {
    name.starts_with("AWS::")
}
