//! Migration specification files with resource ID mappings

use crate::template::Template;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

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
pub(crate) fn parse_migration_spec(path: &Path) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "Failed to read migration spec file: {} ({})",
            path.display(),
            e
        )
    })?;

    let parsed: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse migration spec as JSON: {} ({})",
            path.display(),
            e
        )
    })?;

    let resources_obj = parsed
        .get("resources")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            format!(
                "Migration spec must contain a 'resources' object: {}",
                path.display()
            )
        })?;

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
pub(crate) fn validate_migration_spec_resources(
    mappings: &HashMap<String, String>,
    template: &Template,
) -> Result<(), Box<dyn Error>> {
    let resources = template
        .content
        .get("Resources")
        .and_then(|v| v.as_object())
        .ok_or("Template does not contain a Resources section")?;

    let invalid_ids: Vec<&str> = mappings
        .keys()
        .filter(|old_id| !resources.contains_key(*old_id))
        .map(String::as_str)
        .collect();

    if !invalid_ids.is_empty() {
        return Err(format!(
            "Migration spec contains resource IDs that do not exist in the source template: {}",
            invalid_ids.join(", ")
        )
        .into());
    }

    Ok(())
}
