//! Unit tests of the helpers used by the CLI flow

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
    let yaml_result = cfn_yaml::parse_yaml_to_json(yaml_template);
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
    let yaml_result = cfn_yaml::parse_yaml_to_json(yaml_template);
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
    let result = cfn_yaml::parse_yaml_to_json(yaml_template);
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
    let reparsed = cfn_yaml::parse_yaml_to_json(&output);
    assert!(reparsed.is_ok());
}

#[test]
fn test_template_serialization_error_handling() {
    // Test: Template::to_string() returns Result, not panicking
    use serde_json::json;

    // Normal case should succeed
    let valid_template = Template::new(json!({"key": "value"}), TemplateFormat::Json);
    assert!(valid_template.to_string().is_ok());

    // Note: It's hard to trigger serialization errors with serde_json and the YAML emitter
    // as they can serialize any valid JSON value, but we've verified the
    // error path exists by returning Result instead of unwrapping
}

// ========================================================================
// Template File I/O Utility Tests
// ========================================================================

#[test]
fn test_get_timestamp_format() {
    // Test: Timestamp is in YYYYMMDD-HHMMSS format
    let timestamp = get_timestamp();

    // Should be 15 characters: YYYYMMDD-HHMMSS
    assert_eq!(timestamp.len(), 15);

    // Should have dash in position 8
    assert_eq!(&timestamp[8..9], "-");

    // Should be all digits except the dash
    let without_dash: String = timestamp.chars().filter(|&c| c != '-').collect();
    assert!(without_dash.chars().all(|c| c.is_ascii_digit()));

    // Year should be reasonable (>= 2020)
    let year: u32 = timestamp[0..4].parse().unwrap();
    assert!(year >= 2020);
}

#[test]
fn test_is_leap_year() {
    // Test known leap years
    assert!(is_leap_year(2020)); // Divisible by 4
    assert!(is_leap_year(2024)); // Divisible by 4
    assert!(is_leap_year(2000)); // Divisible by 400

    // Test non-leap years
    assert!(!is_leap_year(2021)); // Not divisible by 4
    assert!(!is_leap_year(2022)); // Not divisible by 4
    assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
    assert!(!is_leap_year(2100)); // Divisible by 100 but not 400
}

#[test]
fn test_generate_filename() {
    // Test JSON format
    let filename = generate_filename("MyStack", "refactor", TemplateFormat::Json);
    assert!(filename.starts_with("MyStack-refactor-"));
    assert!(filename.ends_with(".json"));
    assert!(filename.contains('-')); // Contains timestamp

    // Test YAML format
    let filename = generate_filename("TestStack", "retain", TemplateFormat::Yaml);
    assert!(filename.starts_with("TestStack-retain-"));
    assert!(filename.ends_with(".yaml"));

    // Test different operations
    let filename = generate_filename("Stack1", "import", TemplateFormat::Json);
    assert!(filename.contains("Stack1-import-"));

    let filename = generate_filename("Stack2", "final", TemplateFormat::Yaml);
    assert!(filename.contains("Stack2-final-"));
}

#[test]
fn test_resolve_file_collision_no_collision() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Test: File doesn't exist, should return base path
    let result = resolve_file_collision(dir_path, "test.yaml");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.file_name().unwrap(), "test.yaml");
}

#[test]
fn test_resolve_file_collision_with_collision() {
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create existing file
    let existing_file = dir_path.join("test.yaml");
    fs::write(&existing_file, "existing content").unwrap();

    // Test: File exists, should return .1 variant
    let result = resolve_file_collision(dir_path, "test.yaml");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.file_name().unwrap(), "test.1.yaml");
}

#[test]
fn test_resolve_file_collision_multiple_collisions() {
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create existing files: test.yaml, test.1.yaml, test.2.yaml
    fs::write(dir_path.join("test.yaml"), "v0").unwrap();
    fs::write(dir_path.join("test.1.yaml"), "v1").unwrap();
    fs::write(dir_path.join("test.2.yaml"), "v2").unwrap();

    // Test: Should return .3 variant
    let result = resolve_file_collision(dir_path, "test.yaml");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.file_name().unwrap(), "test.3.yaml");
}

#[test]
fn test_resolve_file_collision_json_format() {
    use std::fs;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create existing JSON file
    fs::write(dir_path.join("stack.json"), "{}").unwrap();

    // Test: Should handle .json extension correctly
    let result = resolve_file_collision(dir_path, "stack.json");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.file_name().unwrap(), "stack.1.json");
}

#[test]
fn test_write_and_read_template_json() {
    use serde_json::json;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test-template.json");

    // Create a test template
    let template_content = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "MyBucket": {
                "Type": "AWS::S3::Bucket"
            }
        }
    });
    let template = Template::new(template_content.clone(), TemplateFormat::Json);

    // Write template to file
    let write_result = write_template_to_file(&template, &file_path);
    assert!(write_result.is_ok());

    // Verify file exists
    assert!(file_path.exists());

    // Read template back from file
    let read_result = read_template_from_file(&file_path);
    assert!(read_result.is_ok());
    let read_template = read_result.unwrap();

    // Verify format is preserved
    assert_eq!(read_template.format, TemplateFormat::Json);

    // Verify content is identical
    assert_eq!(read_template.content, template_content);
}

#[test]
fn test_write_and_read_template_yaml() {
    use serde_json::json;
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test-template.yaml");

    // Create a test template
    let template_content = json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "MyTable": {
                "Type": "AWS::DynamoDB::Table",
                "Properties": {
                    "TableName": "TestTable"
                }
            }
        }
    });
    let template = Template::new(template_content.clone(), TemplateFormat::Yaml);

    // Write template to file
    let write_result = write_template_to_file(&template, &file_path);
    assert!(write_result.is_ok());

    // Verify file exists
    assert!(file_path.exists());

    // Read template back from file
    let read_result = read_template_from_file(&file_path);
    assert!(read_result.is_ok());
    let read_template = read_result.unwrap();

    // Verify format is detected as YAML (file parsing detects from content, not extension)
    // Since we write as YAML, it should be read back as YAML
    assert_eq!(read_template.format, TemplateFormat::Yaml);

    // Verify content is identical
    assert_eq!(read_template.content, template_content);
}

#[test]
fn test_read_template_from_file_not_found() {
    use std::path::PathBuf;

    let non_existent = PathBuf::from("/tmp/does-not-exist-12345.yaml");
    let result = read_template_from_file(&non_existent);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to open file"));
}

#[test]
fn test_write_error_context() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let context_path = temp_dir.path().join("error-context.txt");

    // Create resource mappings
    let mut resources = HashMap::new();
    resources.insert("Bucket1".to_string(), "Bucket1New".to_string());
    resources.insert("Table1".to_string(), "Table1".to_string());

    // Write error context
    let result = write_error_context(
        &context_path,
        "20260228-143022",
        &ErrorContext {
            error: "Template validation failed",
            source_stack: "SourceStack",
            target_stack: Some("TargetStack"),
            operation: "refactor",
            resources: &resources,
        },
    );

    assert!(result.is_ok());
    assert!(context_path.exists());

    // Read and verify content
    let content = std::fs::read_to_string(&context_path).unwrap();
    assert!(content.contains("Error: Template validation failed"));
    assert!(content.contains("Operation: refactor"));
    assert!(content.contains("Source Stack: SourceStack"));
    assert!(content.contains("Target Stack: TargetStack"));
    assert!(content.contains("Bucket1 -> Bucket1New"));
    assert!(content.contains("Table1 (no rename)"));
    assert!(content.contains("Full Error Details:"));
}

#[test]
fn test_write_error_context_same_stack() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let context_path = temp_dir.path().join("error-context-same-stack.txt");

    // Create resource mappings
    let mut resources = HashMap::new();
    resources.insert("OldId".to_string(), "NewId".to_string());

    // Write error context (no target stack for same-stack operation)
    let result = write_error_context(
        &context_path,
        "20260228-143022",
        &ErrorContext {
            error: "AWS API error",
            source_stack: "MyStack",
            target_stack: None,
            operation: "refactor",
            resources: &resources,
        },
    );

    assert!(result.is_ok());
    assert!(context_path.exists());

    // Read and verify content
    let content = std::fs::read_to_string(&context_path).unwrap();
    assert!(content.contains("Source Stack: MyStack"));
    assert!(!content.contains("Target Stack:")); // Should not have target stack line
    assert!(content.contains("OldId -> NewId"));
}

#[test]
fn test_validate_output_directory_existing() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Validate existing directory
    let result = validate_output_directory(dir_path, dir_path);
    assert!(result.is_ok());

    let validated_path = result.unwrap();
    assert!(validated_path.is_absolute());
    assert!(validated_path.is_dir());
}

#[test]
fn test_validate_output_directory_creates_new() {
    use tempfile::TempDir;

    // Create temporary directory
    let temp_dir = TempDir::new().unwrap();
    let new_dir = temp_dir.path().join("new_subdir").join("nested");

    // Directory should not exist yet
    assert!(!new_dir.exists());

    // Validate should create it
    let result = validate_output_directory(&new_dir, temp_dir.path());
    assert!(result.is_ok());

    // Directory should now exist
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

#[test]
fn test_validate_output_directory_file_not_directory() {
    use tempfile::NamedTempFile;

    // Create a temporary file (not a directory)
    let temp_file = NamedTempFile::new().unwrap();
    let file_path = temp_file.path();

    // Attempting to validate a file as a directory should fail
    let result = validate_output_directory(file_path, Path::new("/"));
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not a directory"));
}

#[test]
fn test_validate_output_directory_relative_path() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // A relative path is resolved against the base directory
    let rel_path = Path::new("test_output");
    let result = validate_output_directory(rel_path, temp_dir.path());

    assert!(result.is_ok());
    let validated_path = result.unwrap();
    assert!(validated_path.is_absolute());
    assert_eq!(validated_path, temp_dir.path().join("test_output"));
    assert!(validated_path.is_dir());
}

#[test]
fn test_parse_migration_spec_valid() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary migration spec file
    let mut temp_file = NamedTempFile::new().unwrap();
    let spec_content = r#"{
            "resources": {
                "OldBucket": "NewBucket",
                "OldTable": "NewTable",
                "SameNameResource": "SameNameResource"
            }
        }"#;
    temp_file.write_all(spec_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    // Parse migration spec
    let result = parse_migration_spec(temp_file.path());
    assert!(result.is_ok());

    let mappings = result.unwrap();
    assert_eq!(mappings.len(), 3);
    assert_eq!(mappings.get("OldBucket"), Some(&"NewBucket".to_string()));
    assert_eq!(mappings.get("OldTable"), Some(&"NewTable".to_string()));
    assert_eq!(
        mappings.get("SameNameResource"),
        Some(&"SameNameResource".to_string())
    );
}

#[test]
fn test_parse_migration_spec_file_not_found() {
    use std::path::PathBuf;

    let non_existent = PathBuf::from("/tmp/migration-spec-does-not-exist.json");
    let result = parse_migration_spec(&non_existent);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to read migration spec file"));
}

#[test]
fn test_parse_migration_spec_invalid_json() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary file with invalid JSON
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"{ invalid json }").unwrap();
    temp_file.flush().unwrap();

    let result = parse_migration_spec(temp_file.path());
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to parse migration spec as JSON"));
}

#[test]
fn test_parse_migration_spec_missing_resources_field() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary file without "resources" field
    let mut temp_file = NamedTempFile::new().unwrap();
    let spec_content = r#"{ "other_field": {} }"#;
    temp_file.write_all(spec_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = parse_migration_spec(temp_file.path());
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("must contain a 'resources' object"));
}

#[test]
fn test_parse_migration_spec_non_string_value() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary file with non-string value
    let mut temp_file = NamedTempFile::new().unwrap();
    let spec_content = r#"{
            "resources": {
                "Resource1": "ValidString",
                "Resource2": 123
            }
        }"#;
    temp_file.write_all(spec_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = parse_migration_spec(temp_file.path());
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("mapping value must be a string"));
}

#[test]
fn test_parse_migration_spec_empty_resources() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create temporary file with empty resources object
    let mut temp_file = NamedTempFile::new().unwrap();
    let spec_content = r#"{ "resources": {} }"#;
    temp_file.write_all(spec_content.as_bytes()).unwrap();
    temp_file.flush().unwrap();

    let result = parse_migration_spec(temp_file.path());
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("contains no resource mappings"));
}

#[test]
fn test_validate_migration_spec_resources_valid() {
    // Create template with resources
    let template_json = serde_json::json!({
        "Resources": {
            "Bucket1": { "Type": "AWS::S3::Bucket" },
            "Table1": { "Type": "AWS::DynamoDB::Table" },
            "Instance1": { "Type": "AWS::EC2::Instance" }
        }
    });
    let template = Template::new(template_json, TemplateFormat::Json);

    // Create mappings with valid resource IDs
    let mut mappings = HashMap::new();
    mappings.insert("Bucket1".to_string(), "NewBucket".to_string());
    mappings.insert("Table1".to_string(), "Table1".to_string());

    // Validation should succeed
    let result = validate_migration_spec_resources(&mappings, &template);
    assert!(result.is_ok());
}

#[test]
fn test_validate_migration_spec_resources_invalid_id() {
    // Create template with resources
    let template_json = serde_json::json!({
        "Resources": {
            "Bucket1": { "Type": "AWS::S3::Bucket" },
            "Table1": { "Type": "AWS::DynamoDB::Table" }
        }
    });
    let template = Template::new(template_json, TemplateFormat::Json);

    // Create mappings with invalid resource ID
    let mut mappings = HashMap::new();
    mappings.insert("NonExistent".to_string(), "NewName".to_string());
    mappings.insert("Table1".to_string(), "Table1".to_string());

    // Validation should fail
    let result = validate_migration_spec_resources(&mappings, &template);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("do not exist in the source template"));
    assert!(err_msg.contains("NonExistent"));
}

#[test]
fn test_validate_migration_spec_resources_template_without_resources() {
    // Create template without Resources section
    let template_json = serde_json::json!({
        "AWSTemplateFormatVersion": "2010-09-09"
    });
    let template = Template::new(template_json, TemplateFormat::Json);

    // Create mappings
    let mut mappings = HashMap::new();
    mappings.insert("Bucket1".to_string(), "NewBucket".to_string());

    // Validation should fail
    let result = validate_migration_spec_resources(&mappings, &template);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("does not contain a Resources section"));
}

// ============================================================================
// Integration/Workflow Tests
// ============================================================================

#[test]
fn test_workflow_template_file_collision_handling() {
    // T041: Test file collision handling with actual files
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create a template
    let template_json = serde_json::json!({
        "Resources": {
            "Bucket1": { "Type": "AWS::S3::Bucket" }
        }
    });
    let template = Template::new(template_json, TemplateFormat::Json);

    // Write the first template (no collision)
    let path1 = export_templates(
        &[(template.clone(), "source".to_string())],
        Some(&dir_path.to_path_buf()),
        dir_path,
        "TestStack",
        "refactor",
    )
    .unwrap()[0]
        .clone();

    assert!(path1.exists());
    assert!(path1
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("TestStack-refactor-source-"));
    assert_eq!(path1.extension().unwrap(), "json");

    // Write the second template (should create .1 variant)
    let path2 = export_templates(
        &[(template.clone(), "source".to_string())],
        Some(&dir_path.to_path_buf()),
        dir_path,
        "TestStack",
        "refactor",
    )
    .unwrap()[0]
        .clone();

    assert!(path2.exists());
    assert_ne!(path1, path2);
    assert!(path2
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".1.json"));

    // Write the third template (should create .2 variant)
    let path3 = export_templates(
        &[(template.clone(), "source".to_string())],
        Some(&dir_path.to_path_buf()),
        dir_path,
        "TestStack",
        "refactor",
    )
    .unwrap()[0]
        .clone();

    assert!(path3.exists());
    assert_ne!(path1, path3);
    assert_ne!(path2, path3);
    assert!(path3
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains(".2.json"));
}

#[test]
fn test_workflow_export_multiple_templates() {
    // T037/T038: Test exporting multiple templates in one operation
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create different templates
    let template1_json = serde_json::json!({
        "Resources": {
            "Bucket1": { "Type": "AWS::S3::Bucket" }
        }
    });
    let template1 = Template::new(template1_json, TemplateFormat::Json);

    let template2_json = serde_json::json!({
        "Resources": {
            "Table1": { "Type": "AWS::DynamoDB::Table" }
        }
    });
    let template2 = Template::new(template2_json, TemplateFormat::Yaml);

    // Export multiple templates (simulating import mode with 4 templates)
    let paths = export_templates(
        &[
            (template1.clone(), "source-retained".to_string()),
            (template1.clone(), "source-removed".to_string()),
            (template2.clone(), "target-with-retention".to_string()),
            (template2.clone(), "target-final".to_string()),
        ],
        Some(&dir_path.to_path_buf()),
        dir_path,
        "ImportStack",
        "import",
    )
    .unwrap();

    // Verify 4 files were created
    assert_eq!(paths.len(), 4);

    // Verify all files exist and have correct extensions
    assert!(paths[0].exists());
    assert!(paths[0]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("source-retained"));
    assert_eq!(paths[0].extension().unwrap(), "json");

    assert!(paths[1].exists());
    assert!(paths[1]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("source-removed"));
    assert_eq!(paths[1].extension().unwrap(), "json");

    assert!(paths[2].exists());
    assert!(paths[2]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("target-with-retention"));
    assert_eq!(paths[2].extension().unwrap(), "yaml");

    assert!(paths[3].exists());
    assert!(paths[3]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .contains("target-final"));
    assert_eq!(paths[3].extension().unwrap(), "yaml");

    // Verify file contents are readable
    let read_back1 = read_template_from_file(&paths[0]).unwrap();
    assert_eq!(read_back1.format, TemplateFormat::Json);

    let read_back2 = read_template_from_file(&paths[2]).unwrap();
    assert_eq!(read_back2.format, TemplateFormat::Yaml);
}

#[test]
fn test_workflow_roundtrip_template_json() {
    // T027: Test template input workflow - write and read back JSON template
    use tempfile::NamedTempFile;

    let template_json = serde_json::json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Resources": {
            "MyBucket": {
                "Type": "AWS::S3::Bucket",
                "Properties": {
                    "BucketName": "my-test-bucket"
                }
            },
            "MyTable": {
                "Type": "AWS::DynamoDB::Table",
                "Properties": {
                    "TableName": "my-test-table",
                    "AttributeDefinitions": [
                        { "AttributeName": "id", "AttributeType": "S" }
                    ],
                    "KeySchema": [
                        { "AttributeName": "id", "KeyType": "HASH" }
                    ]
                }
            }
        }
    });

    let original = Template::new(template_json, TemplateFormat::Json);

    // Write template to file
    let temp_file = NamedTempFile::new().unwrap();
    write_template_to_file(&original, temp_file.path()).unwrap();

    // Read template back
    let read_back = read_template_from_file(temp_file.path()).unwrap();

    // Verify format preserved
    assert_eq!(read_back.format, TemplateFormat::Json);

    // Verify content matches
    assert_eq!(
        read_back
            .content
            .get("AWSTemplateFormatVersion")
            .unwrap()
            .as_str()
            .unwrap(),
        "2010-09-09"
    );

    let resources = read_back
        .content
        .get("Resources")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(resources.contains_key("MyBucket"));
    assert!(resources.contains_key("MyTable"));

    let bucket = resources.get("MyBucket").unwrap().as_object().unwrap();
    assert_eq!(
        bucket.get("Type").unwrap().as_str().unwrap(),
        "AWS::S3::Bucket"
    );
}

#[test]
fn test_workflow_roundtrip_template_yaml() {
    // T027: Test template input workflow - write and read back YAML template
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.path().join("test-template.yaml");

    let template_json = serde_json::json!({
        "AWSTemplateFormatVersion": "2010-09-09",
        "Description": "Test YAML template",
        "Resources": {
            "SecurityGroup": {
                "Type": "AWS::EC2::SecurityGroup",
                "Properties": {
                    "GroupDescription": "Test security group",
                    "SecurityGroupIngress": [
                        {
                            "IpProtocol": "tcp",
                            "FromPort": 80,
                            "ToPort": 80,
                            "CidrIp": "0.0.0.0/0"
                        }
                    ]
                }
            }
        }
    });

    let original = Template::new(template_json, TemplateFormat::Yaml);

    // Write template to file
    write_template_to_file(&original, &yaml_file).unwrap();

    // Verify file exists and has .yaml extension
    assert!(yaml_file.exists());
    assert!(yaml_file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .ends_with(".yaml"));

    // Read template back
    let read_back = read_template_from_file(&yaml_file).unwrap();

    // Verify format preserved as YAML
    assert_eq!(read_back.format, TemplateFormat::Yaml);

    // Verify content matches
    assert_eq!(
        read_back
            .content
            .get("Description")
            .unwrap()
            .as_str()
            .unwrap(),
        "Test YAML template"
    );

    let resources = read_back
        .content
        .get("Resources")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(resources.contains_key("SecurityGroup"));
}

#[test]
fn test_workflow_migration_spec_end_to_end() {
    // T027: Test migration spec workflow - write spec file, read it back, validate against template
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create migration spec file
    let spec_content = r#"{
  "resources": {
    "OldBucket": "NewBucket",
    "OldTable": "NewTable",
    "OldInstance": "Instance1"
  }
}"#;

    let mut spec_file = NamedTempFile::new().unwrap();
    spec_file.write_all(spec_content.as_bytes()).unwrap();
    spec_file.flush().unwrap();

    // Parse migration spec
    let mappings = parse_migration_spec(spec_file.path()).unwrap();

    assert_eq!(mappings.len(), 3);
    assert_eq!(mappings.get("OldBucket").unwrap(), "NewBucket");
    assert_eq!(mappings.get("OldTable").unwrap(), "NewTable");
    assert_eq!(mappings.get("OldInstance").unwrap(), "Instance1");

    // Create matching template
    let template_json = serde_json::json!({
        "Resources": {
            "OldBucket": { "Type": "AWS::S3::Bucket" },
            "OldTable": { "Type": "AWS::DynamoDB::Table" },
            "OldInstance": { "Type": "AWS::EC2::Instance" }
        }
    });
    let template = Template::new(template_json, TemplateFormat::Json);

    // Validate spec against template
    let result = validate_migration_spec_resources(&mappings, &template);
    assert!(result.is_ok());
}

#[test]
fn test_workflow_format_preservation_across_operations() {
    // T052: Verify format preservation through export and import
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Test JSON preservation
    let json_template = Template::new(
        serde_json::json!({"Resources": {"Bucket1": {"Type": "AWS::S3::Bucket"}}}),
        TemplateFormat::Json,
    );

    let json_paths = export_templates(
        &[(json_template, "source".to_string())],
        Some(&temp_dir.path().to_path_buf()),
        temp_dir.path(),
        "JsonStack",
        "refactor",
    )
    .unwrap();

    let json_read_back = read_template_from_file(&json_paths[0]).unwrap();
    assert_eq!(json_read_back.format, TemplateFormat::Json);
    assert_eq!(json_paths[0].extension().unwrap(), "json");

    // Test YAML preservation
    let yaml_template = Template::new(
        serde_json::json!({"Resources": {"Table1": {"Type": "AWS::DynamoDB::Table"}}}),
        TemplateFormat::Yaml,
    );

    let yaml_paths = export_templates(
        &[(yaml_template, "target".to_string())],
        Some(&temp_dir.path().to_path_buf()),
        temp_dir.path(),
        "YamlStack",
        "import",
    )
    .unwrap();

    let yaml_read_back = read_template_from_file(&yaml_paths[0]).unwrap();
    assert_eq!(yaml_read_back.format, TemplateFormat::Yaml);
    assert_eq!(yaml_paths[0].extension().unwrap(), "yaml");
}

#[test]
fn test_workflow_error_context_generation() {
    // T049: Test error context file generation workflow
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let error_file = temp_dir.path().join("migration-error.txt");

    let error_msg =
        "Template validation failed: Missing required property 'Type' in resource 'MyBucket'";
    let source_stack = "FailedStack";
    let target_stack = Some("TargetStack");
    let operation = "refactor";

    // Create resource mappings
    let mut resources = HashMap::new();
    resources.insert("MyBucket".to_string(), "NewBucket".to_string());
    resources.insert("MyTable".to_string(), "MyTable".to_string());

    // Write error context
    let result = write_error_context(
        &error_file,
        "20260228-143022",
        &ErrorContext {
            error: error_msg,
            source_stack,
            target_stack,
            operation,
            resources: &resources,
        },
    );

    assert!(result.is_ok());

    // Verify file exists
    assert!(error_file.exists());

    // Verify file content
    let content = std::fs::read_to_string(&error_file).unwrap();
    assert!(content.contains("Error: Template validation failed"));
    assert!(content.contains("Operation: refactor"));
    assert!(content.contains("Source Stack: FailedStack"));
    assert!(content.contains("Target Stack: TargetStack"));
    assert!(content.contains("MyBucket -> NewBucket"));
    assert!(content.contains("MyTable (no rename)"));
    assert!(content.contains("Full Error Details:"));
}

// ============================================================================
// CLI Argument Validation Tests
// ============================================================================

fn parse_args(extra: &[&str]) -> Args {
    let mut argv = vec!["cfn-teleport"];
    argv.extend_from_slice(extra);
    Args::try_parse_from(argv).unwrap()
}

fn validate_err(extra: &[&str]) -> String {
    validate_args(&parse_args(extra)).unwrap_err().to_string()
}

#[test]
fn test_validate_args_defaults_ok() {
    assert!(validate_args(&parse_args(&[])).is_ok());
    assert!(validate_args(&parse_args(&["--export"])).is_ok());
    assert!(validate_args(&parse_args(&["--export", "--out-dir", "x"])).is_ok());
}

#[test]
fn test_validate_args_flag_conflicts() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    let err = validate_err(&["--export", "--source-template", path]);
    assert!(err.contains("Cannot use --export with --source-template"));

    let err = validate_err(&["--export", "--target-template", path]);
    assert!(err.contains("Cannot use --export with --target-template"));

    let err = validate_err(&["--out-dir", "x"]);
    assert!(err.contains("Cannot use --out-dir without --export"));

    let err = validate_err(&["--migration-spec", path, "--resource", "A"]);
    assert!(err.contains("Cannot use --migration-spec with --resource"));
}

#[test]
fn test_validate_args_valid_files() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();
    let args = parse_args(&[
        "--source-template",
        path,
        "--target-template",
        path,
        "--migration-spec",
        path,
    ]);
    assert!(validate_args(&args).is_ok());
    assert!(validate_args(&parse_args(&["--export", "--migration-spec", path])).is_ok());
}

#[test]
fn test_validate_args_missing_files() {
    let dir = tempfile::TempDir::new().unwrap();
    let missing = dir.path().join("missing.json");
    let missing = missing.to_str().unwrap();

    let err = validate_err(&["--source-template", missing]);
    assert!(err.contains("Source template file does not exist"));
    let err = validate_err(&["--target-template", missing]);
    assert!(err.contains("Target template file does not exist"));
    let err = validate_err(&["--migration-spec", missing]);
    assert!(err.contains("Migration spec file does not exist"));
}

#[test]
fn test_validate_args_directories() {
    let dir = tempfile::TempDir::new().unwrap();
    let dir = dir.path().to_str().unwrap();

    let err = validate_err(&["--source-template", dir]);
    assert!(err.contains("Source template path is not a file"));
    let err = validate_err(&["--target-template", dir]);
    assert!(err.contains("Target template path is not a file"));
    let err = validate_err(&["--migration-spec", dir]);
    assert!(err.contains("Migration spec path is not a file"));
}

#[cfg(unix)]
#[test]
fn test_validate_args_unreadable_files() {
    use std::os::unix::fs::PermissionsExt;

    let file = tempfile::NamedTempFile::new().unwrap();
    fs::set_permissions(file.path(), fs::Permissions::from_mode(0o000)).unwrap();
    // Root can read anything, so validation passes then
    let readable = fs::File::open(file.path()).is_ok();
    let path = file.path().to_str().unwrap();

    for (flag, expected) in [
        ("--source-template", "Cannot read source template file"),
        ("--target-template", "Cannot read target template file"),
        ("--migration-spec", "Cannot read migration spec file"),
    ] {
        let result = validate_args(&parse_args(&[flag, path]));
        let message = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert_eq!(message.contains(expected), !readable);
    }
}
