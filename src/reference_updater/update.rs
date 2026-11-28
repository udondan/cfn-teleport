use super::is_pseudo_parameter;
use serde_json::{Map, Value};

/// Recursively traverses a JSON structure and updates resource references.
///
/// # Arguments
/// * `value` - The current JSON value being processed
/// * `old_id` - The old resource ID to find
/// * `new_id` - The new resource ID to replace with
///
/// # Returns
/// The updated JSON value
pub(super) fn traverse_and_update(value: Value, old_id: &str, new_id: &str) -> Value {
    match value {
        Value::Object(map) => update_object(map, old_id, new_id),
        Value::Array(array) => Value::Array(
            array
                .into_iter()
                .map(|item| traverse_and_update(item, old_id, new_id))
                .collect(),
        ),
        // Strings, numbers, bools, and null pass through unchanged
        _ => value,
    }
}

fn update_object(mut map: Map<String, Value>, old_id: &str, new_id: &str) -> Value {
    // A matching Ref or GetAtt is the whole object, nothing left to update
    let renamed =
        renamed_ref(&map, old_id, new_id).or_else(|| renamed_getatt(&map, old_id, new_id));
    if let Some((key, value)) = renamed {
        map.insert(key.to_string(), value);
        return Value::Object(map);
    }

    if let Some(sub) = map.get_mut("Fn::Sub") {
        *sub = update_sub_value(sub.take(), old_id, new_id);
    }

    if let Some(depends_on) = map.get_mut("DependsOn") {
        *depends_on = update_dependson_value(depends_on.take(), old_id, new_id);
    }

    for value in map.values_mut() {
        *value = traverse_and_update(value.take(), old_id, new_id);
    }

    Value::Object(map)
}

/// New value for `{"Ref": old_id}`
fn renamed_ref(
    map: &Map<String, Value>,
    old_id: &str,
    new_id: &str,
) -> Option<(&'static str, Value)> {
    let name = map.get("Ref")?.as_str()?;
    (name == old_id && !is_pseudo_parameter(name))
        .then(|| ("Ref", Value::String(new_id.to_string())))
}

/// New value for `{"Fn::GetAtt": [old_id, "Attr"]}` or `{"Fn::GetAtt": "old_id.Attr"}`
fn renamed_getatt(
    map: &Map<String, Value>,
    old_id: &str,
    new_id: &str,
) -> Option<(&'static str, Value)> {
    let new_value = match map.get("Fn::GetAtt")? {
        Value::Array(array) if array.first()?.as_str()? == old_id => {
            let mut new_array = array.clone();
            new_array[0] = Value::String(new_id.to_string());
            Value::Array(new_array)
        }
        Value::String(getatt) => {
            let (name, attribute) = getatt.split_once('.')?;
            (name == old_id).then(|| Value::String(format!("{}.{}", new_id, attribute)))?
        }
        _ => return None,
    };
    Some(("Fn::GetAtt", new_value))
}

/// Updates a Fn::Sub value (string or array form)
fn update_sub_value(sub_value: Value, old_id: &str, new_id: &str) -> Value {
    match sub_value {
        Value::String(template) => Value::String(rename_sub_vars(&template, old_id, new_id)),
        Value::Array(mut array) => {
            // Array form: [template_string, variable_map]
            if let Some(Value::String(template)) = array.first_mut() {
                *template = rename_sub_vars(template, old_id, new_id);
            }
            if let Some(Value::Object(vars)) = array.get_mut(1) {
                rename_sub_var_map(vars, old_id, new_id);
            }
            Value::Array(array)
        }
        _ => sub_value,
    }
}

/// Replaces `${old_id}` and `${old_id.Attr}` in a Fn::Sub template string
fn rename_sub_vars(template: &str, old_id: &str, new_id: &str) -> String {
    template
        .replace(&format!("${{{}}}", old_id), &format!("${{{}}}", new_id))
        .replace(&format!("${{{}.", old_id), &format!("${{{}.", new_id))
}

/// Renames the `old_id` key of a Fn::Sub variable map and updates references in its values
fn rename_sub_var_map(vars: &mut Map<String, Value>, old_id: &str, new_id: &str) {
    if let Some(value) = vars.remove(old_id) {
        vars.insert(
            new_id.to_string(),
            traverse_and_update(value, old_id, new_id),
        );
    }

    for value in vars.values_mut() {
        *value = traverse_and_update(value.take(), old_id, new_id);
    }
}

/// Updates a DependsOn value (string or array form)
fn update_dependson_value(depends_value: Value, old_id: &str, new_id: &str) -> Value {
    let rename = |item: Value| match item {
        Value::String(name) if name == old_id => Value::String(new_id.to_string()),
        other => other,
    };
    match depends_value {
        Value::Array(array) => Value::Array(array.into_iter().map(rename).collect()),
        other => rename(other),
    }
}
