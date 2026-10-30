//! Tests of the CloudFormation API helpers against a fake API

use super::*;
use crate::test_support::{form_value, FakeCfn, Reply};
use serde_json::json;

fn resource_summary(logical: &str, physical: &str, typ: &str) -> StackResourceSummary {
    StackResourceSummary::builder()
        .logical_resource_id(logical)
        .physical_resource_id(physical)
        .resource_type(typ)
        .build()
}

type StackResourceSummary = cloudformation::types::StackResourceSummary;

fn bucket_template() -> Template {
    Template::new(
        json!({"Resources": {"Bucket": {"Type": "AWS::S3::Bucket"}}}),
        TemplateFormat::Json,
    )
}

fn rename_bucket() -> HashMap<String, String> {
    HashMap::from([("Bucket".to_string(), "NewBucket".to_string())])
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------
// get_stacks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_stacks_paginates_filters_and_sorts() {
    let fake = FakeCfn::new(vec![
        Reply::list_stacks(
            &[("Zeta", "CREATE_COMPLETE"), ("Gone", "DELETE_COMPLETE")],
            Some("token-1"),
        ),
        Reply::list_stacks(&[("Alpha", "UPDATE_COMPLETE")], None),
    ]);

    let stacks = get_stacks(&fake.client()).await.unwrap();

    let names: Vec<&str> = stacks.iter().map(|s| s.stack_name().unwrap()).collect();
    assert_eq!(names, vec!["Alpha", "Zeta"]);
    let requests = fake.requests("ListStacks");
    assert_eq!(
        form_value(&requests[1], "NextToken").as_deref(),
        Some("token-1")
    );
    fake.assert_done();
}

#[tokio::test]
async fn test_get_stacks_error() {
    let fake = FakeCfn::new(vec![Reply::error(
        "ListStacks",
        "AccessDenied",
        "not allowed",
    )]);

    let err = get_stacks(&fake.client()).await.unwrap_err();

    assert_eq!(err.code(), Some("AccessDenied"));
    assert_eq!(err.message(), Some("not allowed"));
}

#[test]
fn test_filter_demo_stacks() {
    let stacks = vec![
        cloudformation::types::StackSummary::builder()
            .stack_name("CfnTeleportTest1")
            .build(),
        cloudformation::types::StackSummary::builder()
            .stack_name("Other")
            .build(),
    ];

    assert_eq!(filter_demo_stacks(stacks.clone(), false).len(), 2);
    let demo = filter_demo_stacks(stacks, true);
    assert_eq!(demo.len(), 1);
    assert_eq!(demo[0].stack_name(), Some("CfnTeleportTest1"));
}

// ---------------------------------------------------------------------------
// get_resources
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_resources_filters_unsupported_and_sorts() {
    let fake = FakeCfn::new(vec![Reply::list_stack_resources(&[
        ("Queue", "q-1", "AWS::SQS::Queue"),
        ("BucketB", "b-2", "AWS::S3::Bucket"),
        ("Custom", "c-1", "Custom::Thing"),
        ("BucketA", "b-1", "AWS::S3::Bucket"),
    ])]);

    let resources = get_resources(&fake.client(), "Stack").await.unwrap();

    let ids: Vec<&str> = resources
        .iter()
        .map(|r| r.logical_resource_id().unwrap())
        .collect();
    assert_eq!(ids, vec!["BucketA", "BucketB", "Queue"]);
    assert_eq!(
        form_value(&fake.requests("ListStackResources")[0], "StackName").as_deref(),
        Some("Stack")
    );
}

#[tokio::test]
async fn test_get_resources_error() {
    let fake = FakeCfn::new(vec![Reply::error(
        "ListStackResources",
        "ValidationError",
        "Stack does not exist",
    )]);

    let err = get_resources(&fake.client(), "Stack").await.unwrap_err();

    assert!(!err.to_string().is_empty());
}

// ---------------------------------------------------------------------------
// get_template
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_template_json() {
    let fake = FakeCfn::new(vec![Reply::get_template(
        r#"{"Resources":{"Bucket":{"Type":"AWS::S3::Bucket"}}}"#,
    )]);

    let template = get_template(&fake.client(), "Stack", None).await.unwrap();

    assert_eq!(template.format, TemplateFormat::Json);
    assert_eq!(
        template.content["Resources"]["Bucket"]["Type"],
        "AWS::S3::Bucket"
    );
}

#[tokio::test]
async fn test_get_template_yaml_with_tags() {
    let fake = FakeCfn::new(vec![Reply::get_template(
        "Resources:\n  Bucket:\n    Type: AWS::S3::Bucket\n    Properties:\n      BucketName: !Ref Name\n",
    )]);

    let template = get_template(&fake.client(), "Stack", None).await.unwrap();

    assert_eq!(template.format, TemplateFormat::Yaml);
    assert_eq!(
        template.content["Resources"]["Bucket"]["Properties"]["BucketName"],
        json!({"Ref": "Name"})
    );
}

#[tokio::test]
async fn test_get_template_yaml_without_tags() {
    let fake = FakeCfn::new(vec![Reply::get_template(
        "Resources:\n  Bucket:\n    Type: AWS::S3::Bucket\n",
    )]);

    let template = get_template(&fake.client(), "Stack", None).await.unwrap();

    assert_eq!(template.format, TemplateFormat::Yaml);
}

#[tokio::test]
async fn test_get_template_invalid() {
    let fake = FakeCfn::new(vec![Reply::get_template("a: [b\n")]);

    let err = get_template(&fake.client(), "Stack", None)
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Failed to parse template as JSON or YAML"));
}

#[tokio::test]
async fn test_get_template_without_body() {
    let fake = FakeCfn::new(vec![Reply::ok("GetTemplate", "")]);

    let err = get_template(&fake.client(), "Stack", None)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "No template found");
}

#[tokio::test]
async fn test_get_template_error() {
    let fake = FakeCfn::new(vec![Reply::error("GetTemplate", "Throttling", "slow down")]);

    assert!(get_template(&fake.client(), "Stack", None).await.is_err());
}

#[tokio::test]
async fn test_get_template_from_file_skips_api() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("template.json");
    fs::write(&path, r#"{"Resources":{}}"#).unwrap();
    let fake = FakeCfn::new(vec![]);

    let template = get_template(&fake.client(), "Stack", Some(&path))
        .await
        .unwrap();

    assert_eq!(template.content, json!({"Resources": {}}));
    assert!(fake.requests("GetTemplate").is_empty());
}

// ---------------------------------------------------------------------------
// validate_template / update_stack / stack status
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_validate_template() {
    let fake = FakeCfn::new(vec![
        Reply::validate_template(),
        Reply::error("ValidateTemplate", "ValidationError", "bad template"),
    ]);
    let client = fake.client();

    validate_template(&client, json!({"Resources": {}}))
        .await
        .unwrap();
    assert!(validate_template(&client, json!({})).await.is_err());

    let body = form_value(&fake.requests("ValidateTemplate")[0], "TemplateBody").unwrap();
    assert_eq!(body, r#"{"Resources":{}}"#);
}

#[tokio::test]
async fn test_update_stack() {
    let fake = FakeCfn::new(vec![
        Reply::update_stack(),
        Reply::error("UpdateStack", "ValidationError", "No updates"),
    ]);
    let client = fake.client();

    update_stack(
        &client,
        "Stack",
        json!({"Resources": {}}),
        TemplateFormat::Yaml,
    )
    .await
    .unwrap();
    assert!(
        update_stack(&client, "Stack", json!({}), TemplateFormat::Json)
            .await
            .is_err()
    );

    let request = &fake.requests("UpdateStack")[0];
    assert_eq!(form_value(request, "StackName").as_deref(), Some("Stack"));
    assert_eq!(
        form_value(request, "Capabilities.member.3").as_deref(),
        Some("CAPABILITY_AUTO_EXPAND")
    );
}

#[tokio::test]
async fn test_get_stack_status() {
    let fake = FakeCfn::new(vec![
        Reply::describe_stacks("UPDATE_COMPLETE"),
        Reply::ok("DescribeStacks", "<Stacks></Stacks>"),
        Reply::error("DescribeStacks", "ValidationError", "does not exist"),
    ]);
    let client = fake.client();

    assert_eq!(
        get_stack_status(&client, "Stack").await.unwrap(),
        Some(cloudformation::types::StackStatus::UpdateComplete)
    );
    assert_eq!(
        get_stack_status(&client, "Stack")
            .await
            .unwrap_err()
            .to_string(),
        "Failed to determine stack status"
    );
    assert!(get_stack_status(&client, "Stack").await.is_err());
}

#[tokio::test(start_paused = true)]
async fn test_wait_for_stack_update_completion_polls_until_complete() {
    let fake = FakeCfn::new(vec![
        Reply::describe_stacks("UPDATE_IN_PROGRESS"),
        Reply::describe_stacks("UPDATE_COMPLETE_CLEANUP_IN_PROGRESS"),
        Reply::describe_stacks("UPDATE_COMPLETE"),
    ]);

    wait_for_stack_update_completion(&fake.client(), "Stack", Some(spinner::Spin::new("wait")))
        .await
        .unwrap();

    fake.assert_done();
}

#[tokio::test(start_paused = true)]
async fn test_wait_for_stack_update_completion_import() {
    let fake = FakeCfn::new(vec![
        Reply::describe_stacks("IMPORT_IN_PROGRESS"),
        Reply::describe_stacks("IMPORT_COMPLETE"),
    ]);

    wait_for_stack_update_completion(&fake.client(), "Stack", None)
        .await
        .unwrap();

    fake.assert_done();
}

#[tokio::test]
async fn test_wait_for_stack_update_completion_failed() {
    let fake = FakeCfn::new(vec![Reply::describe_stacks("UPDATE_ROLLBACK_COMPLETE")]);

    let err = wait_for_stack_update_completion(&fake.client(), "Stack", None)
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Stack update failed UPDATE_ROLLBACK_COMPLETE"
    );
}

#[tokio::test(start_paused = true)]
async fn test_wait_for_stack_update_completion_error_while_polling() {
    let fake = FakeCfn::new(vec![
        Reply::describe_stacks("UPDATE_IN_PROGRESS"),
        Reply::error("DescribeStacks", "Throttling", "slow down"),
    ]);

    assert!(
        wait_for_stack_update_completion(&fake.client(), "Stack", None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn test_wait_for_stack_update_completion_initial_error() {
    let fake = FakeCfn::new(vec![Reply::error(
        "DescribeStacks",
        "Throttling",
        "slow down",
    )]);

    assert!(
        wait_for_stack_update_completion(&fake.client(), "Stack", None)
            .await
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// Change sets (import mode)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_resource_identifier_mapping() {
    let fake = FakeCfn::new(vec![
        Reply::get_template_summary(&[
            ("AWS::S3::Bucket", &["BucketA", "BucketB"], "BucketName"),
            ("AWS::SQS::Queue", &["Queue"], "QueueUrl"),
        ]),
        Reply::error("GetTemplateSummary", "ValidationError", "bad"),
    ]);
    let client = fake.client();

    let map = get_resource_identifier_mapping(&client, "{}")
        .await
        .unwrap();

    assert_eq!(map.len(), 3);
    assert_eq!(map["BucketB"], "BucketName");
    assert_eq!(map["Queue"], "QueueUrl");
    assert!(get_resource_identifier_mapping(&client, "{}")
        .await
        .is_err());
}

#[tokio::test]
async fn test_create_changeset() {
    let fake = FakeCfn::new(vec![
        Reply::get_template_summary(&[("AWS::S3::Bucket", &["NewBucket", "Other"], "BucketName")]),
        Reply::create_change_set(),
    ]);
    let renamed = resource_summary("Bucket", "my-bucket", "AWS::S3::Bucket");
    let kept = resource_summary("Other", "other-bucket", "AWS::S3::Bucket");

    let name = create_changeset(
        &fake.client(),
        "Target",
        json!({"Resources": {}}),
        TemplateFormat::Json,
        vec![&renamed, &kept],
        rename_bucket(),
    )
    .await
    .unwrap();

    assert!(name.starts_with("Target-"));
    let request = &fake.requests("CreateChangeSet")[0];
    assert_eq!(form_value(request, "ChangeSetName"), Some(name));
    assert_eq!(
        form_value(request, "ChangeSetType").as_deref(),
        Some("IMPORT")
    );
    assert_eq!(
        form_value(request, "ResourcesToImport.member.1.LogicalResourceId").as_deref(),
        Some("NewBucket")
    );
    assert_eq!(
        form_value(
            request,
            "ResourcesToImport.member.1.ResourceIdentifier.entry.1.value"
        )
        .as_deref(),
        Some("my-bucket")
    );
    assert_eq!(
        form_value(request, "ResourcesToImport.member.2.LogicalResourceId").as_deref(),
        Some("Other")
    );
}

#[tokio::test]
async fn test_create_changeset_errors() {
    let fake = FakeCfn::new(vec![
        Reply::error("GetTemplateSummary", "ValidationError", "bad"),
        Reply::get_template_summary(&[]),
        Reply::error("CreateChangeSet", "ValidationError", "bad"),
    ]);
    let client = fake.client();

    for _ in 0..2 {
        let result = create_changeset(
            &client,
            "Target",
            json!({}),
            TemplateFormat::Json,
            vec![],
            HashMap::new(),
        )
        .await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_execute_changeset() {
    let fake = FakeCfn::new(vec![
        Reply::execute_change_set(),
        Reply::error("ExecuteChangeSet", "InvalidChangeSetStatus", "bad"),
    ]);
    let client = fake.client();

    execute_changeset(&client, "Target", "cs").await.unwrap();
    assert!(execute_changeset(&client, "Target", "cs").await.is_err());
    assert_eq!(
        form_value(&fake.requests("ExecuteChangeSet")[0], "ChangeSetName").as_deref(),
        Some("cs")
    );
}

#[tokio::test]
async fn test_get_changeset_status() {
    let fake = FakeCfn::new(vec![
        Reply::describe_change_set("CREATE_COMPLETE", None),
        Reply::describe_change_set("FAILED", Some("No changes")),
        Reply::error("DescribeChangeSet", "ChangeSetNotFound", "missing"),
    ]);
    let client = fake.client();

    assert_eq!(
        get_changeset_status(&client, "Target", "cs").await.unwrap(),
        Some(cloudformation::types::ChangeSetStatus::CreateComplete)
    );
    assert_eq!(
        get_changeset_status(&client, "Target", "cs")
            .await
            .unwrap_err()
            .to_string(),
        "No changes"
    );
    assert!(get_changeset_status(&client, "Target", "cs").await.is_err());
}

#[tokio::test(start_paused = true)]
async fn test_wait_for_changeset_created() {
    let fake = FakeCfn::new(vec![
        Reply::describe_change_set("CREATE_PENDING", None),
        Reply::describe_change_set("CREATE_IN_PROGRESS", None),
        Reply::describe_change_set("CREATE_COMPLETE", None),
    ]);

    wait_for_changeset_created(&fake.client(), "Target", "cs")
        .await
        .unwrap();

    fake.assert_done();
}

#[tokio::test(start_paused = true)]
async fn test_wait_for_changeset_created_failures() {
    let fake = FakeCfn::new(vec![
        Reply::describe_change_set("DELETE_COMPLETE", None),
        Reply::describe_change_set("CREATE_PENDING", None),
        Reply::error("DescribeChangeSet", "Throttling", "slow down"),
        Reply::error("DescribeChangeSet", "Throttling", "slow down"),
    ]);
    let client = fake.client();

    assert_eq!(
        wait_for_changeset_created(&client, "Target", "cs")
            .await
            .unwrap_err()
            .to_string(),
        "Changeset creation failed DELETE_COMPLETE"
    );
    assert!(wait_for_changeset_created(&client, "Target", "cs")
        .await
        .is_err());
    assert!(wait_for_changeset_created(&client, "Target", "cs")
        .await
        .is_err());
    fake.assert_done();
}

// ---------------------------------------------------------------------------
// Stack refactor: same stack
// ---------------------------------------------------------------------------

async fn rename(fake: &FakeCfn, work_dir: &Path) -> Result<(), Box<dyn Error>> {
    refactor_stack_resources(
        &fake.client(),
        "Stack",
        bucket_template(),
        rename_bucket(),
        work_dir,
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn test_refactor_same_stack_success() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![
        Reply::validate_template(),
        Reply::create_stack_refactor(Some("refactor-1")),
        Reply::describe_stack_refactor(Some("CREATE_IN_PROGRESS"), None, None),
        Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
        Reply::execute_stack_refactor(),
        Reply::describe_stack_refactor(None, Some("EXECUTE_IN_PROGRESS"), None),
        Reply::describe_stack_refactor(None, Some("EXECUTE_COMPLETE"), None),
    ]);

    rename(&fake, dir.path()).await.unwrap();

    fake.assert_done();
    let request = &fake.requests("CreateStackRefactor")[0];
    assert_eq!(
        form_value(
            request,
            "ResourceMappings.member.1.Destination.LogicalResourceId"
        )
        .as_deref(),
        Some("NewBucket")
    );
    let body = form_value(request, "StackDefinitions.member.1.TemplateBody").unwrap();
    assert!(body.contains("NewBucket"));
    assert!(files_in(dir.path()).is_empty());
}

#[tokio::test]
async fn test_refactor_same_stack_validation_failure_saves_files() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![Reply::error(
        "ValidateTemplate",
        "ValidationError",
        "Template format error",
    )]);

    let err = rename(&fake, dir.path()).await.unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Updated template validation failed"));
    let files = files_in(dir.path());
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files[0].starts_with("Stack-error-refactor-context-"));
    assert!(files[1].starts_with("Stack-error-refactor-same-stack-refactored-"));
}

#[tokio::test]
async fn test_refactor_same_stack_validation_failure_unwritable_dir() {
    let fake = FakeCfn::new(vec![Reply::error(
        "ValidateTemplate",
        "ValidationError",
        "Template format error",
    )]);

    let err = rename(&fake, Path::new("/nonexistent/cfn-teleport"))
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Updated template validation failed"));
}

#[tokio::test]
async fn test_refactor_same_stack_api_errors() {
    let dir = tempfile::tempdir().unwrap();
    let scripts = vec![
        (
            vec![Reply::error(
                "CreateStackRefactor",
                "ValidationError",
                "nope",
            )],
            "Failed to create stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::error("DescribeStackRefactor", "Throttling", "slow down"),
            ],
            "Failed to describe stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_FAILED"), None, Some("bad mapping")),
            ],
            "Stack refactor validation failed: bad mapping",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("FAILED"), None, None),
            ],
            "Stack refactor validation failed: Unknown error",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::error("ExecuteStackRefactor", "ValidationError", "nope"),
            ],
            "Failed to execute stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::error("DescribeStackRefactor", "Throttling", "slow down"),
            ],
            "Failed to describe stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::describe_stack_refactor(None, Some("EXECUTE_FAILED"), Some("boom")),
            ],
            "Stack refactor execution failed: boom",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::describe_stack_refactor(None, Some("FAILED"), None),
            ],
            "Stack refactor execution failed: Unknown error",
        ),
    ];

    for (replies, expected) in scripts {
        let mut all = vec![Reply::validate_template()];
        all.extend(replies);
        let fake = FakeCfn::new(all);

        let err = rename(&fake, dir.path()).await.unwrap_err();

        assert!(err.to_string().starts_with(expected), "{err}");
        fake.assert_done();
    }
}

#[tokio::test]
async fn test_refactor_same_stack_template_without_resources() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![
        Reply::validate_template(),
        Reply::error("CreateStackRefactor", "ValidationError", "nope"),
    ]);

    let result = refactor_stack_resources(
        &fake.client(),
        "Stack",
        Template::new(json!({}), TemplateFormat::Json),
        HashMap::from([
            ("A".to_string(), "B".to_string()),
            ("C".to_string(), "D".to_string()),
        ]),
        dir.path(),
    )
    .await;

    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Stack refactor: cross stack
// ---------------------------------------------------------------------------

async fn move_bucket(fake: &FakeCfn, work_dir: &Path) -> Result<(), Box<dyn Error>> {
    refactor_stack_resources_cross_stack(
        &fake.client(),
        "Source",
        "Target",
        bucket_template(),
        Template::new(json!({"Resources": {}}), TemplateFormat::Yaml),
        rename_bucket(),
        work_dir,
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn test_refactor_cross_stack_success() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![
        Reply::validate_template(),
        Reply::validate_template(),
        Reply::create_stack_refactor(Some("refactor-1")),
        Reply::describe_stack_refactor(Some("CREATE_IN_PROGRESS"), None, None),
        Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
        Reply::execute_stack_refactor(),
        Reply::describe_stack_refactor(None, Some("EXECUTE_IN_PROGRESS"), None),
        Reply::describe_stack_refactor(None, Some("EXECUTE_COMPLETE"), None),
    ]);

    move_bucket(&fake, dir.path()).await.unwrap();

    fake.assert_done();
    let request = &fake.requests("CreateStackRefactor")[0];
    assert_eq!(
        form_value(request, "StackDefinitions.member.2.StackName").as_deref(),
        Some("Target")
    );
    assert_eq!(
        form_value(request, "ResourceMappings.member.1.Destination.StackName").as_deref(),
        Some("Target")
    );
    let target_body = form_value(request, "StackDefinitions.member.2.TemplateBody").unwrap();
    assert!(target_body.contains("NewBucket:"), "{target_body}");
}

#[tokio::test]
async fn test_refactor_cross_stack_dangling_reference() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![]);
    let source = Template::new(
        json!({"Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Policy": {"Type": "AWS::S3::BucketPolicy", "Properties": {"Bucket": {"Ref": "Bucket"}}}
        }}),
        TemplateFormat::Json,
    );

    let result = refactor_stack_resources_cross_stack(
        &fake.client(),
        "Source",
        "Target",
        source,
        Template::new(json!({"Resources": {}}), TemplateFormat::Json),
        rename_bucket(),
        dir.path(),
    )
    .await;

    assert!(result.is_err());
    assert!(fake.requests("ValidateTemplate").is_empty());
}

#[tokio::test]
async fn test_refactor_cross_stack_validation_failures_save_files() {
    let cases = vec![
        (
            Reply::error("ValidateTemplate", "ValidationError", "bad source"),
            Reply::validate_template(),
            "Source template validation failed",
        ),
        (
            Reply::validate_template(),
            Reply::error("ValidateTemplate", "ValidationError", "bad target"),
            "Target template validation failed",
        ),
    ];

    for (source_reply, target_reply, expected) in cases {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeCfn::new(vec![source_reply, target_reply]);

        let err = move_bucket(&fake, dir.path()).await.unwrap_err();

        assert!(err.to_string().starts_with(expected), "{err}");
        let files = files_in(dir.path());
        assert_eq!(files.len(), 3, "{files:?}");
        assert!(files[0].starts_with("Source-error-refactor-cross-stack-context-"));
        assert!(files[1].starts_with("Source-error-refactor-cross-stack-source-Source-"));
        assert!(files[2].starts_with("Source-error-refactor-cross-stack-target-Target-"));
    }
}

#[tokio::test]
async fn test_refactor_cross_stack_validation_failure_unwritable_dir() {
    let fake = FakeCfn::new(vec![
        Reply::error("ValidateTemplate", "ValidationError", "bad source"),
        Reply::validate_template(),
    ]);

    let err = move_bucket(&fake, Path::new("/nonexistent/cfn-teleport"))
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Source template validation failed"));
}

#[tokio::test]
async fn test_refactor_cross_stack_api_errors() {
    let dir = tempfile::tempdir().unwrap();
    let scripts = vec![
        (
            vec![Reply::error(
                "CreateStackRefactor",
                "ValidationError",
                "nope",
            )],
            "Failed to create stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::error("DescribeStackRefactor", "Throttling", "slow down"),
            ],
            "Failed to describe stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_FAILED"), None, Some("bad mapping")),
            ],
            "Stack refactor validation failed: bad mapping",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("FAILED"), None, None),
            ],
            "Stack refactor validation failed: Unknown error",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::error("ExecuteStackRefactor", "ValidationError", "nope"),
            ],
            "Failed to execute stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::error("DescribeStackRefactor", "Throttling", "slow down"),
            ],
            "Failed to describe stack refactor",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::describe_stack_refactor(None, Some("EXECUTE_FAILED"), Some("boom")),
            ],
            "Stack refactor execution failed: boom",
        ),
        (
            vec![
                Reply::create_stack_refactor(Some("refactor-1")),
                Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
                Reply::execute_stack_refactor(),
                Reply::describe_stack_refactor(None, Some("FAILED"), None),
            ],
            "Stack refactor execution failed: Unknown error",
        ),
    ];

    for (replies, expected) in scripts {
        let mut all = vec![Reply::validate_template(), Reply::validate_template()];
        all.extend(replies);
        let fake = FakeCfn::new(all);

        let err = move_bucket(&fake, dir.path()).await.unwrap_err();

        assert!(err.to_string().starts_with(expected), "{err}");
        fake.assert_done();
    }
}

// ---------------------------------------------------------------------------
// format_cfn_error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_format_cfn_error() {
    let fake = FakeCfn::new(vec![
        Reply::error("ValidateTemplate", "ValidationError", "bad template"),
        Reply::bare_error("ValidateTemplate"),
    ]);
    let client = fake.client();

    let detailed = client.validate_template().send().await.unwrap_err();
    assert_eq!(
        format_cfn_error("validate", &detailed),
        "Failed to validate\nError code: ValidationError\nMessage: bad template"
    );

    let bare = client.validate_template().send().await.unwrap_err();
    assert_eq!(
        format_cfn_error("validate", &bare),
        "Failed to validate\n(No additional error details provided by AWS)"
    );
}
