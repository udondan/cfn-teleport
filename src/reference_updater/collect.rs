use super::is_pseudo_parameter;
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Recursively collects all resource references from a JSON value.
///
/// # Arguments
/// * `value` - The JSON value to scan
/// * `references` - Set to collect found resource IDs into
pub(super) fn collect_references(value: &Value, references: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            collect_ref(map, references);
            collect_getatt(map, references);
            collect_sub(map, references);
            collect_depends_on(map, references);

            for value in map.values() {
                collect_references(value, references);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_references(item, references);
            }
        }
        _ => {}
    }
}

/// `{"Ref": "Name"}`, ignoring pseudo parameters like `AWS::Region`
fn collect_ref(map: &Map<String, Value>, references: &mut HashSet<String>) {
    let Some(name) = map.get("Ref").and_then(Value::as_str) else {
        return;
    };
    if !is_pseudo_parameter(name) {
        references.insert(name.to_string());
    }
}

/// `{"Fn::GetAtt": ["Name", "Attr"]}` or `{"Fn::GetAtt": "Name.Attr"}`
fn collect_getatt(map: &Map<String, Value>, references: &mut HashSet<String>) {
    let name = match map.get("Fn::GetAtt") {
        Some(Value::Array(array)) => array.first().and_then(Value::as_str),
        Some(Value::String(value)) => value.split('.').next(),
        _ => None,
    };
    if let Some(name) = name {
        references.insert(name.to_string());
    }
}

/// `{"Fn::Sub": "..."}` or `{"Fn::Sub": ["...", {variables}]}`
fn collect_sub(map: &Map<String, Value>, references: &mut HashSet<String>) {
    match map.get("Fn::Sub") {
        Some(Value::String(template)) => {
            extract_sub_references(template, references, &HashSet::new());
        }
        Some(Value::Array(array)) => {
            let Some(template) = array.first().and_then(Value::as_str) else {
                return;
            };
            // Variables defined in the variable map (second element) are not references
            let local_vars: HashSet<String> = array
                .get(1)
                .and_then(Value::as_object)
                .map(|vars| vars.keys().cloned().collect())
                .unwrap_or_default();
            extract_sub_references(template, references, &local_vars);
        }
        _ => {}
    }
}

/// `DependsOn` of a resource definition (an object with a `Type`)
fn collect_depends_on(map: &Map<String, Value>, references: &mut HashSet<String>) {
    if !map.contains_key("Type") {
        return;
    }
    match map.get("DependsOn") {
        Some(Value::String(dependency)) => {
            references.insert(dependency.clone());
        }
        Some(Value::Array(dependencies)) => {
            references.extend(
                dependencies
                    .iter()
                    .filter_map(Value::as_str)
                    .map(String::from),
            );
        }
        _ => {}
    }
}

/// Extracts resource references from Fn::Sub template strings.
///
/// Looks for ${ResourceName} or ${ResourceName.Attribute} patterns.
/// Excludes variables defined in the Fn::Sub variable map.
fn extract_sub_references(
    template: &str,
    references: &mut HashSet<String>,
    local_vars: &HashSet<String>,
) {
    for name in sub_variables(template) {
        if !name.is_empty() && !is_pseudo_parameter(name) && !local_vars.contains(name) {
            references.insert(name.to_string());
        }
    }
}

/// Names used in the `${...}` placeholders of a Fn::Sub string, without attribute.
/// Escaped literals (`${!Literal}`) are skipped.
fn sub_variables(template: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("${") {
        let body = &rest[start + 2..];
        if !body.starts_with('!') {
            let end = body.find(['}', '.']).unwrap_or(body.len());
            names.push(&body[..end]);
        }
        // Continue after the closing brace
        let next = body.find('}').map_or(body.len(), |i| i + 1);
        rest = &body[next..];
    }

    names
}
