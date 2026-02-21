use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
        for (_output_name, output_def) in outputs {
            collect_references(output_def, &mut refs_in_outputs);
        }

        if !refs_in_outputs.is_empty() {
            references.insert("Outputs".to_string(), refs_in_outputs);
        }
    }

    references
}

/// Recursively collects all resource references from a JSON value.
///
/// # Arguments
/// * `value` - The JSON value to scan
/// * `references` - Set to collect found resource IDs into
fn collect_references(value: &Value, references: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            // Check for Ref
            if let Some(ref_value) = map.get("Ref") {
                if let Some(resource_name) = ref_value.as_str() {
                    if !is_pseudo_parameter(resource_name) {
                        references.insert(resource_name.to_string());
                    }
                }
            }

            // Check for GetAtt
            if let Some(getatt_value) = map.get("Fn::GetAtt") {
                if let Some(array) = getatt_value.as_array() {
                    if let Some(resource_name) = array.first().and_then(|v| v.as_str()) {
                        references.insert(resource_name.to_string());
                    }
                } else if let Some(string_val) = getatt_value.as_str() {
                    if let Some(resource_name) = string_val.split('.').next() {
                        references.insert(resource_name.to_string());
                    }
                }
            }

            // Check for Sub
            if let Some(sub_value) = map.get("Fn::Sub") {
                if let Some(template_str) = sub_value.as_str() {
                    extract_sub_references(template_str, references);
                } else if let Some(array) = sub_value.as_array() {
                    if let Some(template_str) = array.first().and_then(|v| v.as_str()) {
                        extract_sub_references(template_str, references);
                    }
                }
            }

            // Check for DependsOn
            if map.contains_key("Type") {
                // This is a resource definition
                if let Some(depends_on) = map.get("DependsOn") {
                    if let Some(dep_str) = depends_on.as_str() {
                        references.insert(dep_str.to_string());
                    } else if let Some(dep_array) = depends_on.as_array() {
                        for dep in dep_array {
                            if let Some(dep_str) = dep.as_str() {
                                references.insert(dep_str.to_string());
                            }
                        }
                    }
                }
            }

            // Recurse into all values
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

/// Extracts resource references from Fn::Sub template strings.
///
/// Looks for ${ResourceName} or ${ResourceName.Attribute} patterns.
fn extract_sub_references(template: &str, references: &mut HashSet<String>) {
    // Match ${...} patterns
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(&'{') = chars.peek() {
                chars.next(); // consume '{'
                let mut var_name = String::new();

                // Collect until '}' or '.'
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' || next_ch == '.' {
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }

                // Skip until '}'
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    if next_ch == '}' {
                        break;
                    }
                }

                if !var_name.is_empty() && !is_pseudo_parameter(&var_name) {
                    references.insert(var_name);
                }
            }
        }
    }
}

/// Recursively traverses a JSON structure and updates resource references.
///
/// # Arguments
/// * `value` - The current JSON value being processed
/// * `old_id` - The old resource ID to find
/// * `new_id` - The new resource ID to replace with
///
/// # Returns
/// The updated JSON value
fn traverse_and_update(value: Value, old_id: &str, new_id: &str) -> Value {
    match value {
        Value::Object(mut map) => {
            // Check for Ref pattern
            if let Some(ref_value) = map.get("Ref") {
                if let Some(resource_name) = ref_value.as_str() {
                    if resource_name == old_id && !is_pseudo_parameter(resource_name) {
                        map.insert("Ref".to_string(), Value::String(new_id.to_string()));
                        return Value::Object(map);
                    }
                }
            }

            // Check for GetAtt pattern
            if let Some(getatt_value) = map.get("Fn::GetAtt") {
                if let Some(array) = getatt_value.as_array() {
                    if !array.is_empty() {
                        if let Some(resource_name) = array[0].as_str() {
                            if resource_name == old_id {
                                let mut new_array = array.clone();
                                new_array[0] = Value::String(new_id.to_string());
                                map.insert("Fn::GetAtt".to_string(), Value::Array(new_array));
                                return Value::Object(map);
                            }
                        }
                    }
                }
            }

            // Check for Sub pattern
            if let Some(sub_value) = map.get("Fn::Sub").cloned() {
                let updated_sub = update_sub_value(sub_value, old_id, new_id);
                map.insert("Fn::Sub".to_string(), updated_sub);
            }

            // Check for DependsOn pattern
            if let Some(depends_value) = map.get("DependsOn").cloned() {
                let updated_depends = update_dependson_value(depends_value, old_id, new_id);
                map.insert("DependsOn".to_string(), updated_depends);
            }

            // Recursively process all object values
            for (key, val) in map.clone() {
                map.insert(key, traverse_and_update(val, old_id, new_id));
            }

            Value::Object(map)
        }
        Value::Array(array) => {
            // Recursively process all array elements
            let updated_array: Vec<Value> = array
                .into_iter()
                .map(|item| traverse_and_update(item, old_id, new_id))
                .collect();
            Value::Array(updated_array)
        }
        // Strings, numbers, bools, and null pass through unchanged
        _ => value,
    }
}

/// Updates a Fn::Sub value (string or array form)
fn update_sub_value(sub_value: Value, old_id: &str, new_id: &str) -> Value {
    match sub_value {
        Value::String(template_str) => {
            // Simple string form: replace ${OldId} with ${NewId}
            let old_var = format!("${{{}}}", old_id);
            let new_var = format!("${{{}}}", new_id);
            let updated_str = template_str.replace(&old_var, &new_var);

            // Also handle ${OldId.Attribute} form
            let old_var_with_dot = format!("${{{}.", old_id);
            let new_var_with_dot = format!("${{{}.", new_id);
            let updated_str = updated_str.replace(&old_var_with_dot, &new_var_with_dot);

            Value::String(updated_str)
        }
        Value::Array(mut array) => {
            // Array form: [template_string, variable_map]
            if !array.is_empty() {
                // Update template string (first element)
                if let Some(template_str) = array[0].as_str() {
                    let old_var = format!("${{{}}}", old_id);
                    let new_var = format!("${{{}}}", new_id);
                    let updated_str = template_str.replace(&old_var, &new_var);

                    let old_var_with_dot = format!("${{{}.", old_id);
                    let new_var_with_dot = format!("${{{}.", new_id);
                    let updated_str = updated_str.replace(&old_var_with_dot, &new_var_with_dot);

                    array[0] = Value::String(updated_str);
                }

                // Update variable map (second element) if present
                if array.len() > 1 {
                    if let Some(var_map) = array[1].as_object() {
                        let mut new_map = var_map.clone();

                        // Check if old_id is a key in the map - rename the key
                        if let Some(value) = new_map.remove(old_id) {
                            // Recursively update the value in case it contains references
                            let updated_value = traverse_and_update(value, old_id, new_id);
                            new_map.insert(new_id.to_string(), updated_value);
                        }

                        // Update any Ref/GetAtt in the map values
                        for (key, value) in new_map.clone() {
                            new_map.insert(key, traverse_and_update(value, old_id, new_id));
                        }

                        array[1] = Value::Object(new_map);
                    }
                }
            }
            Value::Array(array)
        }
        _ => sub_value,
    }
}

/// Updates a DependsOn value (string or array form)
fn update_dependson_value(depends_value: Value, old_id: &str, new_id: &str) -> Value {
    match depends_value {
        Value::String(resource_name) => {
            if resource_name == old_id {
                Value::String(new_id.to_string())
            } else {
                Value::String(resource_name)
            }
        }
        Value::Array(array) => {
            let updated_array: Vec<Value> = array
                .into_iter()
                .map(|item| {
                    if let Some(resource_name) = item.as_str() {
                        if resource_name == old_id {
                            Value::String(new_id.to_string())
                        } else {
                            item
                        }
                    } else {
                        item
                    }
                })
                .collect();
            Value::Array(updated_array)
        }
        _ => depends_value,
    }
}

/// Checks if a resource name is a CloudFormation pseudo-parameter
fn is_pseudo_parameter(name: &str) -> bool {
    name.starts_with("AWS::")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_traverse_empty_object() {
        let template = json!({});
        let result = traverse_and_update(template.clone(), "OldId", "NewId");
        assert_eq!(result, template);
    }

    #[test]
    fn test_update_ref_basic() {
        let template = json!({ "Ref": "OldBucket" });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Ref": "NewBucket" }));
    }

    #[test]
    fn test_update_ref_pseudo_parameter() {
        let template = json!({ "Ref": "AWS::Region" });
        let result = traverse_and_update(template.clone(), "AWS::Region", "NewRegion");
        assert_eq!(result, json!({ "Ref": "AWS::Region" })); // Unchanged
    }

    #[test]
    fn test_update_ref_not_matching() {
        let template = json!({ "Ref": "OtherResource" });
        let result = traverse_and_update(template.clone(), "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Ref": "OtherResource" })); // Unchanged
    }

    #[test]
    fn test_update_getatt_basic() {
        let template = json!({ "Fn::GetAtt": ["OldBucket", "Arn"] });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Fn::GetAtt": ["NewBucket", "Arn"] }));
    }

    #[test]
    fn test_update_getatt_not_matching() {
        let template = json!({ "Fn::GetAtt": ["OtherResource", "Arn"] });
        let result = traverse_and_update(template.clone(), "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Fn::GetAtt": ["OtherResource", "Arn"] }));
    }

    #[test]
    fn test_update_dependson_string() {
        let template = json!({
            "Type": "AWS::EC2::Instance",
            "DependsOn": "OldSecurityGroup",
            "Properties": {}
        });
        let result = traverse_and_update(template, "OldSecurityGroup", "NewSecurityGroup");
        assert_eq!(result["DependsOn"], json!("NewSecurityGroup"));
    }

    #[test]
    fn test_update_dependson_array() {
        let template = json!({
            "Type": "AWS::EC2::Instance",
            "DependsOn": ["OldSG", "OtherResource", "OldBucket"],
            "Properties": {}
        });
        let mut mapping = HashMap::new();
        mapping.insert("OldSG".to_string(), "NewSG".to_string());
        mapping.insert("OldBucket".to_string(), "NewBucket".to_string());

        let mut result = template.clone();
        for (old_id, new_id) in &mapping {
            result = traverse_and_update(result, old_id, new_id);
        }

        assert_eq!(
            result["DependsOn"],
            json!(["NewSG", "OtherResource", "NewBucket"])
        );
    }

    #[test]
    fn test_update_sub_simple() {
        let template = json!({ "Fn::Sub": "arn:aws:s3:::${OldBucket}/*" });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Fn::Sub": "arn:aws:s3:::${NewBucket}/*" }));
    }

    #[test]
    fn test_update_sub_multiple_vars() {
        let template = json!({ "Fn::Sub": "${OldBucket}-${OldTable}" });
        let mut result = template.clone();
        result = traverse_and_update(result, "OldBucket", "NewBucket");
        result = traverse_and_update(result, "OldTable", "NewTable");
        assert_eq!(result, json!({ "Fn::Sub": "${NewBucket}-${NewTable}" }));
    }

    #[test]
    fn test_update_sub_no_partial_match() {
        let template = json!({ "Fn::Sub": "${OldBucket}-${OldBucket2}" });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        // Should only replace exact matches
        assert_eq!(result, json!({ "Fn::Sub": "${NewBucket}-${OldBucket2}" }));
    }

    #[test]
    fn test_update_sub_with_attribute() {
        let template = json!({ "Fn::Sub": "${OldBucket.Arn}" });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(result, json!({ "Fn::Sub": "${NewBucket.Arn}" }));
    }

    #[test]
    fn test_update_sub_array_form() {
        let template = json!({
            "Fn::Sub": [
                "${BucketName}-suffix",
                {"BucketName": {"Ref": "OldBucket"}}
            ]
        });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(
            result,
            json!({
                "Fn::Sub": [
                    "${BucketName}-suffix",
                    {"BucketName": {"Ref": "NewBucket"}}
                ]
            })
        );
    }

    #[test]
    fn test_update_sub_array_rename_key() {
        let template = json!({
            "Fn::Sub": [
                "${OldBucket}-suffix",
                {"OldBucket": "some-value"}
            ]
        });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");
        assert_eq!(
            result,
            json!({
                "Fn::Sub": [
                    "${NewBucket}-suffix",
                    {"NewBucket": "some-value"}
                ]
            })
        );
    }

    #[test]
    fn test_nested_structure() {
        let template = json!({
            "Resources": {
                "MyInstance": {
                    "Type": "AWS::EC2::Instance",
                    "Properties": {
                        "SecurityGroups": [
                            {"Ref": "OldSG"}
                        ]
                    }
                }
            }
        });
        let result = traverse_and_update(template, "OldSG", "NewSG");
        assert_eq!(
            result["Resources"]["MyInstance"]["Properties"]["SecurityGroups"][0],
            json!({"Ref": "NewSG"})
        );
    }

    #[test]
    fn test_multiple_references_same_resource() {
        let template = json!({
            "Resources": {
                "Resource1": {
                    "Properties": {
                        "Bucket": {"Ref": "OldBucket"},
                        "BucketArn": {"Fn::GetAtt": ["OldBucket", "Arn"]}
                    }
                }
            },
            "Outputs": {
                "BucketName": {
                    "Value": {"Ref": "OldBucket"}
                }
            }
        });
        let result = traverse_and_update(template, "OldBucket", "NewBucket");

        assert_eq!(
            result["Resources"]["Resource1"]["Properties"]["Bucket"],
            json!({"Ref": "NewBucket"})
        );
        assert_eq!(
            result["Resources"]["Resource1"]["Properties"]["BucketArn"],
            json!({"Fn::GetAtt": ["NewBucket", "Arn"]})
        );
        assert_eq!(
            result["Outputs"]["BucketName"]["Value"],
            json!({"Ref": "NewBucket"})
        );
    }

    #[test]
    fn test_update_template_references_main_api() {
        let template = json!({
            "Resources": {
                "MyLambda": {
                    "Type": "AWS::Lambda::Function",
                    "Properties": {
                        "Environment": {
                            "Variables": {
                                "BUCKET": {"Ref": "OldBucket"},
                                "TABLE": {"Ref": "OldTable"}
                            }
                        }
                    }
                }
            }
        });

        let mut mapping = HashMap::new();
        mapping.insert("OldBucket".to_string(), "NewBucket".to_string());
        mapping.insert("OldTable".to_string(), "NewTable".to_string());

        let result = update_template_references(template, &mapping);

        assert_eq!(
            result["Resources"]["MyLambda"]["Properties"]["Environment"]["Variables"]["BUCKET"],
            json!({"Ref": "NewBucket"})
        );
        assert_eq!(
            result["Resources"]["MyLambda"]["Properties"]["Environment"]["Variables"]["TABLE"],
            json!({"Ref": "NewTable"})
        );
    }

    #[test]
    fn test_find_all_references_ref() {
        let template = json!({
            "Resources": {
                "Lambda": {
                    "Type": "AWS::Lambda::Function",
                    "Properties": {
                        "Environment": {
                            "Variables": {
                                "BUCKET": {"Ref": "MyBucket"}
                            }
                        }
                    }
                },
                "MyBucket": {
                    "Type": "AWS::S3::Bucket"
                }
            }
        });

        let references = find_all_references(&template);
        assert!(references.contains_key("Lambda"));
        assert!(references["Lambda"].contains("MyBucket"));
    }

    #[test]
    fn test_find_all_references_getatt() {
        let template = json!({
            "Resources": {
                "Lambda": {
                    "Type": "AWS::Lambda::Function",
                    "Properties": {
                        "TableArn": {"Fn::GetAtt": ["MyTable", "Arn"]}
                    }
                }
            }
        });

        let references = find_all_references(&template);
        assert!(references["Lambda"].contains("MyTable"));
    }

    #[test]
    fn test_find_all_references_dependson() {
        let template = json!({
            "Resources": {
                "Instance": {
                    "Type": "AWS::EC2::Instance",
                    "DependsOn": "MySecurityGroup",
                    "Properties": {}
                }
            }
        });

        let references = find_all_references(&template);
        assert!(references["Instance"].contains("MySecurityGroup"));
    }

    #[test]
    fn test_find_all_references_sub() {
        let template = json!({
            "Resources": {
                "Policy": {
                    "Type": "AWS::IAM::Policy",
                    "Properties": {
                        "PolicyDocument": {
                            "Statement": [{
                                "Resource": {"Fn::Sub": "arn:aws:s3:::${MyBucket}/*"}
                            }]
                        }
                    }
                }
            }
        });

        let references = find_all_references(&template);
        assert!(references["Policy"].contains("MyBucket"));
    }

    #[test]
    fn test_find_all_references_outputs() {
        let template = json!({
            "Resources": {
                "MyBucket": {
                    "Type": "AWS::S3::Bucket"
                }
            },
            "Outputs": {
                "BucketName": {
                    "Value": {"Ref": "MyBucket"}
                }
            }
        });

        let references = find_all_references(&template);
        assert!(references.contains_key("Outputs"));
        assert!(references["Outputs"].contains("MyBucket"));
    }

    #[test]
    fn test_find_all_references_multiple() {
        let template = json!({
            "Resources": {
                "Lambda": {
                    "Type": "AWS::Lambda::Function",
                    "DependsOn": ["MyBucket", "MyTable"],
                    "Properties": {
                        "Environment": {
                            "Variables": {
                                "BUCKET": {"Ref": "MyBucket"},
                                "TABLE_ARN": {"Fn::GetAtt": ["MyTable", "Arn"]}
                            }
                        },
                        "Role": {"Fn::Sub": "arn:aws:iam::${AWS::AccountId}:role/${MyRole}"}
                    }
                }
            }
        });

        let references = find_all_references(&template);
        let lambda_refs = &references["Lambda"];

        assert!(lambda_refs.contains("MyBucket"));
        assert!(lambda_refs.contains("MyTable"));
        assert!(lambda_refs.contains("MyRole"));
        assert!(!lambda_refs.contains("AWS::AccountId")); // Pseudo-parameter should be ignored
    }

    #[test]
    fn test_find_all_references_ignores_pseudo_parameters() {
        let template = json!({
            "Resources": {
                "Resource": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": {"Fn::Sub": "${AWS::StackName}-bucket"},
                        "Tags": [{
                            "Key": "Region",
                            "Value": {"Ref": "AWS::Region"}
                        }]
                    }
                }
            }
        });

        let references = find_all_references(&template);

        // Should not contain any pseudo-parameters
        if let Some(refs) = references.get("Resource") {
            assert!(!refs.contains("AWS::StackName"));
            assert!(!refs.contains("AWS::Region"));
        }
    }
}
