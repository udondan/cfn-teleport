//! Changes to template contents: moving, renaming and protecting resources

use crate::reference_updater;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Keys of a top-level template section like "Parameters" or "Resources"
pub(crate) fn section_keys(template: &Value, section: &str) -> HashSet<String> {
    template
        .get(section)
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

/// Names of the template's parameters
pub(crate) fn parameter_names(template: &Value) -> HashSet<String> {
    section_keys(template, "Parameters")
}

/// Renames resources and updates all references to them
pub(crate) fn rename_resources(mut template: Value, id_mapping: &HashMap<String, String>) -> Value {
    if let Some(resources) = template.get_mut("Resources").and_then(Value::as_object_mut) {
        for (old_id, new_id) in id_mapping {
            if let Some(resource) = resources.remove(old_id) {
                resources.insert(new_id.clone(), resource);
            }
        }
    }

    reference_updater::update_template_references(template, id_mapping)
}

pub(crate) fn retain_resources(mut template: Value, resource_ids: Vec<String>) -> Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        if let Some(resource) = resources.get_mut(&resource_id) {
            resource["DeletionPolicy"] = Value::String("Retain".to_string());
        }
    }

    template
}

// for reasons unknown, importing resource requires a DeletionPolicy to be set. Se we add the documented defaults
// https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-attribute-deletionpolicy.html
pub(crate) fn set_default_deletion_policy(mut template: Value, resource_ids: Vec<String>) -> Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        let Some(resource) = resources.get_mut(&resource_id) else {
            continue;
        };
        let Some(resource_object) = resource.as_object_mut() else {
            continue;
        };
        if !resource_object.contains_key("DeletionPolicy") {
            let policy = default_deletion_policy(resource_object);
            resource_object.insert("DeletionPolicy".to_string(), Value::String(policy.into()));
        }
    }

    template
}

/// The DeletionPolicy CloudFormation uses when none is set
fn default_deletion_policy(resource: &serde_json::Map<String, Value>) -> &'static str {
    match resource["Type"].as_str().unwrap() {
        "AWS::RDS::DBCluster" => "Snapshot",
        // Instances of a cluster are deleted, the cluster keeps the snapshot
        "AWS::RDS::DBInstance"
            if resource
                .get("Properties")
                .and_then(|p| p.get("DBClusterIdentifier"))
                .is_some() =>
        {
            "Delete"
        }
        "AWS::RDS::DBInstance" => "Snapshot",
        _ => "Delete",
    }
}

pub(crate) fn remove_resources(mut template: Value, resource_ids: Vec<String>) -> Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        resources.remove(&resource_id);
    }

    template
}

pub(crate) fn add_resources(
    mut target_template: Value,
    source_template: Value,
    resource_id_map: HashMap<String, String>,
) -> (Value, Value) {
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
