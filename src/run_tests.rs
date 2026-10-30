//! Tests of the CLI flow in `run_with` and the helpers it uses

use super::*;
use crate::test_support::{form_value, Answer, FakeCfn, Reply, ScriptedPrompter};
use serde_json::{json, Value};

type Summary = cloudformation::types::StackResourceSummary;

fn args(extra: &[&str]) -> Args {
    Args::try_parse_from(std::iter::once("cfn-teleport").chain(extra.iter().copied())).unwrap()
}

async fn run(
    fake: &FakeCfn,
    prompter: &ScriptedPrompter,
    extra: &[&str],
    work_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    run_with(args(extra), &fake.client(), prompter, work_dir).await
}

fn stacks() -> Reply {
    Reply::list_stacks(
        &[("Target", "CREATE_COMPLETE"), ("Source", "UPDATE_COMPLETE")],
        None,
    )
}

/// Bucket (index 0) and Queue (index 1) after sorting by type
fn resources() -> Reply {
    Reply::list_stack_resources(&[
        ("Queue", "my-queue", "AWS::SQS::Queue"),
        ("Bucket", "my-bucket", "AWS::S3::Bucket"),
    ])
}

fn source_template() -> Value {
    json!({
        "Parameters": {"Name": {"Type": "String"}},
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Queue": {"Type": "AWS::SQS::Queue"}
        }
    })
}

fn target_template() -> Value {
    json!({"Resources": {"Topic": {"Type": "AWS::SNS::Topic"}}})
}

fn template(value: &Value) -> Reply {
    Reply::get_template(&value.to_string())
}

/// Replies for listing stacks, resources and both templates of a cross-stack move
fn cross_stack_setup(source: &Value) -> Vec<Reply> {
    vec![
        stacks(),
        resources(),
        template(source),
        template(&target_template()),
    ]
}

/// Replies for listing stacks, resources and the template of a same-stack rename
fn same_stack_setup(source: &Value) -> Vec<Reply> {
    vec![stacks(), resources(), template(source)]
}

fn successful_refactor() -> Vec<Reply> {
    vec![
        Reply::create_stack_refactor(Some("refactor-1")),
        Reply::describe_stack_refactor(Some("CREATE_COMPLETE"), None, None),
        Reply::execute_stack_refactor(),
        Reply::describe_stack_refactor(None, Some("EXECUTE_COMPLETE"), None),
    ]
}

fn with(mut replies: Vec<Reply>, more: Vec<Reply>) -> Vec<Reply> {
    replies.extend(more);
    replies
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

fn no_answers() -> ScriptedPrompter {
    ScriptedPrompter::new(vec![])
}

// ---------------------------------------------------------------------------
// Errors while listing stacks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_without_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![]);

    let err = run_with(
        args(&[]),
        &fake.client_without_credentials(),
        &no_answers(),
        dir.path(),
    )
    .await
    .unwrap_err();

    assert!(
        err.to_string().starts_with("AWS credentials not found."),
        "{err}"
    );
}

#[tokio::test]
async fn test_run_aws_error_with_code() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![Reply::error(
        "ListStacks",
        "AccessDenied",
        "not allowed",
    )]);

    let err = run(&fake, &no_answers(), &[], dir.path())
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "AWS Error (AccessDenied): not allowed");
}

#[tokio::test]
async fn test_run_aws_error_without_details() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![Reply::bare_error("ListStacks")]);

    let err = run(&fake, &no_answers(), &[], dir.path())
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "An AWS error occurred");
}

// ---------------------------------------------------------------------------
// Stack and resource selection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_source_selection_aborted() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![stacks()]);
    let prompter = ScriptedPrompter::new(vec![Answer::Select(None)]);

    let err = run(&fake, &prompter, &[], dir.path()).await.unwrap_err();

    assert_eq!(err.to_string(), "User did not select anything");
    assert_eq!(*prompter.prompts.borrow(), vec!["Select source stack"]);
}

#[tokio::test]
async fn test_run_stack_without_resources() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![stacks(), Reply::list_stack_resources(&[])]);

    let err = run(&fake, &no_answers(), &["-s", "Source"], dir.path())
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "No resources found in stack 'Source'");
}

#[tokio::test]
async fn test_run_target_selection_aborted() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(vec![stacks(), resources()]);
    let prompter = ScriptedPrompter::new(vec![Answer::Select(Some(0)), Answer::Select(None)]);

    let err = run(&fake, &prompter, &[], dir.path()).await.unwrap_err();

    assert_eq!(err.to_string(), "User did not select anything");
    // Stacks are sorted, so index 0 is the source stack
    assert_eq!(
        form_value(&fake.requests("ListStackResources")[0], "StackName").as_deref(),
        Some("Source")
    );
}

#[tokio::test]
async fn test_run_unknown_resource() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(cross_stack_setup(&source_template()));

    let err = run(
        &fake,
        &no_answers(),
        &[
            "-s",
            "Source",
            "-t",
            "Target",
            "-r",
            "Bucket",
            "-r",
            "Missing:New",
        ],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert_eq!(
        err.to_string(),
        "ERROR: The following resources do not exist on stack 'Source':\n - Missing"
    );
}

#[tokio::test]
async fn test_run_resource_selection_aborted() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));
    let prompter = ScriptedPrompter::new(vec![Answer::MultiSelect(None)]);

    let err = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Source"],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert_eq!(err.to_string(), "User did not select anything");
}

#[tokio::test]
async fn test_run_nothing_selected() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));
    let prompter = ScriptedPrompter::new(vec![Answer::MultiSelect(Some(vec![]))]);

    let err = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Source"],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert_eq!(err.to_string(), "No resources have been selected");
}

#[tokio::test]
async fn test_run_cross_stack_selection_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let source = json!({
        "Parameters": {"Name": {"Type": "String"}},
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Queue": {"Type": "AWS::SQS::Queue", "Properties": {"QueueName": {"Ref": "Name"}}}
        },
        "Outputs": {"BucketName": {"Value": {"Ref": "Bucket"}}}
    });
    let fake = FakeCfn::new(cross_stack_setup(&source));
    let prompter = ScriptedPrompter::new(vec![Answer::MultiSelect(Some(vec![0, 1]))]);

    let err = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Target"],
        dir.path(),
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(
        err.contains("referenced by stack outputs:\n  - Bucket (AWS::S3::Bucket)"),
        "{err}"
    );
    assert!(
        err.contains("  - Queue (AWS::SQS::Queue) - missing parameters: Name"),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// Same-stack rename
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn test_run_interactive_rename() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(with(
        same_stack_setup(&source_template()),
        with(vec![Reply::validate_template()], successful_refactor()),
    ));
    let prompter = ScriptedPrompter::new(vec![
        Answer::Select(Some(0)),
        Answer::Select(Some(0)),
        Answer::MultiSelect(Some(vec![0])),
        Answer::Input(Some("NewBucket".to_string())),
        Answer::Confirm(Some(true)),
    ]);

    run(&fake, &prompter, &[], dir.path()).await.unwrap();

    fake.assert_done();
    assert_eq!(
        *prompter.prompts.borrow(),
        vec![
            "Select source stack",
            "Select target stack",
            "Select resources to copy",
            "Optionally provide a new logical ID for resource 'Bucket'",
            "Please confirm your selection:",
        ]
    );
    let request = &fake.requests("CreateStackRefactor")[0];
    assert_eq!(
        form_value(
            request,
            "ResourceMappings.member.1.Destination.LogicalResourceId"
        )
        .as_deref(),
        Some("NewBucket")
    );
}

#[tokio::test]
async fn test_run_rename_with_empty_input_keeps_id() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));
    let prompter = ScriptedPrompter::new(vec![
        Answer::MultiSelect(Some(vec![0])),
        Answer::Input(Some(String::new())),
    ]);

    let err = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Source"],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("no resources are being renamed"));
}

#[tokio::test]
async fn test_run_rename_errors() {
    let cases: Vec<(&[&str], &str)> = vec![
        (&["-r", "Bucket"], "no resources are being renamed"),
        (
            &["-r", "Bucket:NewBucket", "-r", "Queue"],
            "did not provide new logical IDs for: Queue",
        ),
        (
            &["-r", "Bucket:Queue"],
            "already exist in stack Source:\n  Bucket -> Queue (resource 'Queue' already exists)",
        ),
    ];

    for (extra, expected) in cases {
        let dir = tempfile::tempdir().unwrap();
        let fake = FakeCfn::new(same_stack_setup(&source_template()));
        let mut all = vec!["-s", "Source", "-t", "Source"];
        all.extend(extra);

        let err = run(&fake, &no_answers(), &all, dir.path())
            .await
            .unwrap_err();

        assert!(err.to_string().contains(expected), "{err}");
    }
}

#[tokio::test]
async fn test_run_rename_not_confirmed() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));
    let prompter = ScriptedPrompter::new(vec![Answer::Confirm(Some(false))]);

    let err = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Source", "-r", "Bucket:NewBucket"],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert_eq!(err.to_string(), "Selection has not been confirmed");
}

#[tokio::test]
async fn test_run_rename_export() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));

    run(
        &fake,
        &no_answers(),
        &[
            "-s",
            "Source",
            "-t",
            "Source",
            "-r",
            "Bucket:NewBucket",
            "--yes",
            "--export",
        ],
        dir.path(),
    )
    .await
    .unwrap();

    let files = files_in(dir.path());
    assert_eq!(files.len(), 1);
    assert!(files[0].starts_with("Source-refactor-same-stack-refactored-"));
    let content = fs::read_to_string(dir.path().join(&files[0])).unwrap();
    assert!(content.contains("NewBucket"));
    assert!(fake.requests("CreateStackRefactor").is_empty());
}

#[tokio::test]
async fn test_run_rename_export_template_without_resources_map() {
    for source in [json!({}), json!({"Resources": []})] {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("out");
        let fake = FakeCfn::new(same_stack_setup(&source));

        run(
            &fake,
            &no_answers(),
            &[
                "-s",
                "Source",
                "-t",
                "Source",
                "-r",
                "Bucket:NewBucket",
                "--yes",
                "--export",
                "--out-dir",
                out_dir.to_str().unwrap(),
            ],
            dir.path(),
        )
        .await
        .unwrap();

        assert_eq!(files_in(&out_dir).len(), 1);
    }
}

// ---------------------------------------------------------------------------
// Cross-stack move with the refactor API
// ---------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn test_run_cross_stack_refactor() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(with(
        cross_stack_setup(&source_template()),
        with(
            vec![Reply::validate_template(), Reply::validate_template()],
            successful_refactor(),
        ),
    ));

    run(
        &fake,
        &no_answers(),
        &["-s", "Source", "-t", "Target", "-r", "Bucket", "--yes"],
        dir.path(),
    )
    .await
    .unwrap();

    fake.assert_done();
}

#[tokio::test]
async fn test_run_cross_stack_export() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("out");
    let fake = FakeCfn::new(cross_stack_setup(&source_template()));

    run(
        &fake,
        &no_answers(),
        &[
            "-s",
            "Source",
            "-t",
            "Target",
            "-r",
            "Bucket:Moved",
            "--yes",
            "--export",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ],
        dir.path(),
    )
    .await
    .unwrap();

    let files = files_in(&out_dir);
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(files[0].starts_with("Source-refactor-cross-stack-source-Source-"));
    assert!(files[1].starts_with("Source-refactor-cross-stack-target-Target-"));
    let target = fs::read_to_string(out_dir.join(&files[1])).unwrap();
    assert!(target.contains("Moved") && target.contains("Topic"));
}

#[tokio::test]
async fn test_run_cross_stack_dangling_reference() {
    let dir = tempfile::tempdir().unwrap();
    let source = json!({"Resources": {
        "Bucket": {"Type": "AWS::S3::Bucket"},
        "Queue": {"Type": "AWS::SQS::Queue", "Properties": {"QueueName": {"Ref": "Bucket"}}}
    }});
    let fake = FakeCfn::new(cross_stack_setup(&source));

    let err = run(
        &fake,
        &no_answers(),
        &["-s", "Source", "-t", "Target", "-r", "Bucket", "--yes"],
        dir.path(),
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Cannot move resources due to dangling references"));
}

#[tokio::test]
async fn test_run_migration_spec_export() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("spec.json");
    fs::write(&spec, r#"{"resources": {"Bucket": "Moved"}}"#).unwrap();
    let out_dir = dir.path().join("out");
    let fake = FakeCfn::new(cross_stack_setup(&source_template()));

    run(
        &fake,
        &no_answers(),
        &[
            "-s",
            "Source",
            "-t",
            "Target",
            "--migration-spec",
            spec.to_str().unwrap(),
            "--yes",
            "--export",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ],
        dir.path(),
    )
    .await
    .unwrap();

    let target = files_in(&out_dir)
        .into_iter()
        .find(|f| f.contains("target-Target"))
        .unwrap();
    assert!(fs::read_to_string(out_dir.join(target))
        .unwrap()
        .contains("Moved"));
}

#[tokio::test(start_paused = true)]
async fn test_run_with_template_files() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.json");
    fs::write(&source, source_template().to_string()).unwrap();
    let target = dir.path().join("target.yaml");
    fs::write(&target, "Resources:\n  Topic:\n    Type: AWS::SNS::Topic\n").unwrap();
    let fake = FakeCfn::new(with(
        vec![stacks(), resources()],
        with(
            vec![Reply::validate_template(), Reply::validate_template()],
            successful_refactor(),
        ),
    ));

    run(
        &fake,
        &no_answers(),
        &[
            "-s",
            "Source",
            "-t",
            "Target",
            "-r",
            "Bucket",
            "--yes",
            "--source-template",
            source.to_str().unwrap(),
            "--target-template",
            target.to_str().unwrap(),
        ],
        dir.path(),
    )
    .await
    .unwrap();

    fake.assert_done();
    assert!(fake.requests("GetTemplate").is_empty());
    let request = &fake.requests("CreateStackRefactor")[0];
    let target_body = form_value(request, "StackDefinitions.member.2.TemplateBody").unwrap();
    assert!(target_body.contains("Bucket:"), "{target_body}");
}

// ---------------------------------------------------------------------------
// Cross-stack move with import mode
// ---------------------------------------------------------------------------

const IMPORT: &[&str] = &["-s", "Source", "-t", "Target", "-m", "import", "--yes"];

fn import_args<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut all = IMPORT.to_vec();
    all.extend(extra);
    all
}

fn four_validations() -> Vec<Reply> {
    (0..4).map(|_| Reply::validate_template()).collect()
}

#[tokio::test]
async fn test_run_import_blocks_parameter_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let source = json!({
        "Parameters": {"Name": {"Type": "String"}},
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket", "Properties": {"BucketName": {"Ref": "Name"}}},
            "Queue": {"Type": "AWS::SQS::Queue"}
        }
    });
    let fake = FakeCfn::new(cross_stack_setup(&source));

    let err = run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket"]),
        dir.path(),
    )
    .await
    .unwrap_err()
    .to_string();

    assert!(err.starts_with("Cannot use import mode for resources that depend on stack parameters:\n  - Bucket (AWS::S3::Bucket) - depends on parameters: Name\n"), "{err}");
}

#[tokio::test]
async fn test_run_import_validation_failure_saves_files() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(with(
        cross_stack_setup(&source_template()),
        vec![
            Reply::validate_template(),
            Reply::error("ValidateTemplate", "ValidationError", "bad template"),
        ],
    ));

    let err = run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket"]),
        dir.path(),
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Unable to proceed, because the template is invalid"));
    let files = files_in(dir.path());
    assert_eq!(files.len(), 5, "{files:?}");
    assert!(files[0].starts_with("Source-error-import-context-"));
    for (file, suffix) in
        files[1..]
            .iter()
            .zip(["removed", "retained", "target-final", "target-with-policy"])
    {
        assert!(
            file.starts_with(&format!("Source-error-import-{suffix}-")),
            "{file}"
        );
    }
}

#[tokio::test]
async fn test_run_import_validation_failure_unwritable_dir() {
    let fake = FakeCfn::new(with(
        cross_stack_setup(&source_template()),
        vec![Reply::error(
            "ValidateTemplate",
            "ValidationError",
            "bad template",
        )],
    ));

    let err = run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket"]),
        Path::new("/nonexistent/cfn-teleport"),
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Unable to proceed, because the template is invalid"));
}

#[tokio::test]
async fn test_run_import_export() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(with(
        cross_stack_setup(&source_template()),
        four_validations(),
    ));

    run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket:Moved", "--export"]),
        dir.path(),
    )
    .await
    .unwrap();

    let files = files_in(dir.path());
    assert_eq!(files.len(), 4, "{files:?}");
    assert!(files.iter().all(|f| f.starts_with("Source-import-")));
    assert!(fake.requests("UpdateStack").is_empty());
}

fn import_replies(retain_first: bool) -> Vec<Reply> {
    let mut replies = four_validations();
    if retain_first {
        replies.push(Reply::update_stack());
        replies.push(Reply::describe_stacks("UPDATE_COMPLETE"));
    }
    replies.extend([
        Reply::update_stack(),
        Reply::describe_stacks("UPDATE_IN_PROGRESS"),
        Reply::describe_stacks("UPDATE_COMPLETE"),
        Reply::get_template_summary(&[("AWS::S3::Bucket", &["Moved"], "BucketName")]),
        Reply::create_change_set(),
        Reply::describe_change_set("CREATE_COMPLETE", None),
        Reply::execute_change_set(),
        Reply::describe_stacks("IMPORT_COMPLETE"),
        Reply::update_stack(),
        Reply::describe_stacks("UPDATE_COMPLETE"),
    ]);
    replies
}

#[tokio::test(start_paused = true)]
async fn test_run_import() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(with(
        cross_stack_setup(&source_template()),
        import_replies(true),
    ));

    run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket:Moved"]),
        dir.path(),
    )
    .await
    .unwrap();

    fake.assert_done();
    let updates = fake.requests("UpdateStack");
    assert_eq!(updates.len(), 3);
    let retained = form_value(&updates[0], "TemplateBody").unwrap();
    assert!(
        retained.contains(r#""DeletionPolicy":"Retain""#),
        "{retained}"
    );
    assert_eq!(
        form_value(&updates[2], "StackName").as_deref(),
        Some("Target")
    );
    let change_set = &fake.requests("CreateChangeSet")[0];
    assert_eq!(
        form_value(change_set, "ResourcesToImport.member.1.LogicalResourceId").as_deref(),
        Some("Moved")
    );
}

#[tokio::test(start_paused = true)]
async fn test_run_import_resource_already_retained() {
    let dir = tempfile::tempdir().unwrap();
    let source = json!({"Resources": {
        "Bucket": {"Type": "AWS::S3::Bucket", "DeletionPolicy": "Retain"},
        "Queue": {"Type": "AWS::SQS::Queue"}
    }});
    let fake = FakeCfn::new(with(cross_stack_setup(&source), import_replies(false)));

    run(
        &fake,
        &no_answers(),
        &import_args(&["-r", "Bucket:Moved"]),
        dir.path(),
    )
    .await
    .unwrap();

    fake.assert_done();
    assert_eq!(fake.requests("UpdateStack").len(), 2);
}

// ---------------------------------------------------------------------------
// Helpers used by the flow
// ---------------------------------------------------------------------------

fn summary(logical: &str, typ: &str) -> Summary {
    Summary::builder()
        .logical_resource_id(logical)
        .physical_resource_id(format!("{logical}-physical"))
        .resource_type(typ)
        .build()
}

fn info(incoming: bool, outgoing: bool, outputs: bool, params: bool) -> ResourceDependencyInfo {
    ResourceDependencyInfo {
        has_incoming_deps: incoming,
        has_outgoing_deps: outgoing,
        referenced_by_outputs: outputs,
        depends_on_parameters: params,
    }
}

#[test]
fn test_split_ids() {
    assert_eq!(
        split_ids("Old:New".to_string()),
        ("Old".to_string(), "New".to_string())
    );
    assert_eq!(
        split_ids("Same".to_string()),
        ("Same".to_string(), "Same".to_string())
    );
}

#[test]
fn test_select_stack() {
    let prompter = ScriptedPrompter::new(vec![Answer::Select(Some(1)), Answer::Select(None)]);
    let items = ["A", "B"];

    assert_eq!(select_stack(&prompter, "Pick", &items).unwrap(), "B");
    assert!(select_stack(&prompter, "Pick", &items).is_err());
}

#[test]
fn test_user_confirm() {
    let prompter = ScriptedPrompter::new(vec![
        Answer::Confirm(Some(true)),
        Answer::Confirm(Some(false)),
        Answer::Confirm(None),
    ]);

    assert!(user_confirm(&prompter).is_ok());
    assert!(user_confirm(&prompter).is_err());
    assert!(user_confirm(&prompter).is_err());
}

#[test]
fn test_prompter_answer_type_mismatch() {
    let prompter = ScriptedPrompter::new(vec![
        Answer::Confirm(None),
        Answer::Select(None),
        Answer::Select(None),
        Answer::Select(None),
    ]);

    assert!(prompter.select("p", &[]).is_err());
    assert!(prompter.confirm("p").is_err());
    assert!(prompter.input("p", "default").is_err());
    assert!(prompter.multi_select("p", &[]).is_err());
}

#[tokio::test]
async fn test_filter_resources() {
    let a = summary("A", "AWS::S3::Bucket");
    let b = summary("B", "AWS::S3::Bucket");

    let resources = [&a, &b];
    let filtered = filter_resources(&resources, &["B".to_string()])
        .await
        .unwrap();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].logical_resource_id(), Some("B"));
}

#[tokio::test]
async fn test_select_resources_cross_stack_parameters() {
    let bucket = summary("Bucket", "AWS::S3::Bucket");
    let queue = summary("Queue", "AWS::SQS::Queue");
    let template = json!({
        "Parameters": {"Name": {"Type": "String"}},
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Queue": {"Type": "AWS::SQS::Queue", "Properties": {"QueueName": {"Ref": "Name"}}}
        }
    });
    let target_with_param = json!({"Parameters": {"Name": {"Type": "String"}}, "Resources": {}});
    let prompter = ScriptedPrompter::new(vec![
        Answer::MultiSelect(Some(vec![1])),
        Answer::MultiSelect(Some(vec![1])),
        Answer::MultiSelect(Some(vec![0, 1])),
    ]);
    let resources = [&bucket, &queue];

    // Without a target template every parameter is missing
    let err = select_resources(&prompter, "p", &resources, &template, None, true)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("missing parameters: Name"));

    // The target has the parameter
    let selected = select_resources(
        &prompter,
        "p",
        &resources,
        &template,
        Some(&target_with_param),
        true,
    )
    .await
    .unwrap();
    assert_eq!(selected[0].logical_resource_id(), Some("Queue"));

    // Same-stack renames are never blocked
    let selected = select_resources(&prompter, "p", &resources, &template, None, false)
        .await
        .unwrap();
    assert_eq!(selected.len(), 2);
}

#[test]
fn test_compute_dependency_markers() {
    let bucket = summary("Bucket", "AWS::S3::Bucket");
    let policy = summary("Policy", "AWS::S3::BucketPolicy");
    let queue = summary("Queue", "AWS::SQS::Queue");
    let template = json!({
        "Parameters": {"Name": {"Type": "String"}},
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Policy": {"Type": "AWS::S3::BucketPolicy", "Properties": {"Bucket": {"Ref": "Bucket"}}},
            "Queue": {"Type": "AWS::SQS::Queue", "Properties": {"QueueName": {"Ref": "Name"}}}
        },
        "Outputs": {"Out": {"Value": {"Ref": "Bucket"}}}
    });

    let markers = compute_dependency_markers(&[&bucket, &policy, &queue], &template);

    let b = &markers["Bucket"];
    assert!(b.has_incoming_deps && !b.has_outgoing_deps && b.referenced_by_outputs);
    let p = &markers["Policy"];
    assert!(!p.has_incoming_deps && p.has_outgoing_deps && !p.referenced_by_outputs);
    let q = &markers["Queue"];
    assert!(q.depends_on_parameters && !q.has_incoming_deps && !q.has_outgoing_deps);
}

#[test]
fn test_generate_legend() {
    assert!(generate_legend(&HashMap::new()).is_none());
    assert!(generate_legend(&HashMap::from([(
        "A".to_string(),
        info(false, false, false, false)
    )]))
    .is_none());

    let deps_only = generate_legend(&HashMap::from([(
        "A".to_string(),
        info(true, false, false, false),
    )]))
    .unwrap();
    assert!(deps_only.contains("Resource dependencies"));
    assert!(!deps_only.contains("Stack interface"));

    let interface_only = generate_legend(&HashMap::from([(
        "A".to_string(),
        info(false, false, false, true),
    )]))
    .unwrap();
    assert!(!interface_only.contains("Resource dependencies"));
    assert!(interface_only.contains("Stack interface"));

    let all = generate_legend(&HashMap::from([
        ("A".to_string(), info(true, true, true, true)),
        ("B".to_string(), info(false, true, true, false)),
    ]))
    .unwrap();
    assert!(all.contains("Resource dependencies") && all.contains("Stack interface"));
}

#[tokio::test]
async fn test_format_resources_markers() {
    let ids = [
        "Both",
        "Incoming",
        "Outgoing",
        "Outputs",
        "Params",
        "Interface",
        "Plain",
        "Unknown",
        "DepsUp",
    ];
    let summaries: Vec<Summary> = ids
        .iter()
        .map(|id| summary(id, "AWS::S3::Bucket"))
        .collect();
    let refs: Vec<&Summary> = summaries.iter().collect();
    let deps = HashMap::from([
        ("Both".to_string(), info(true, true, false, false)),
        ("Incoming".to_string(), info(true, false, false, false)),
        ("Outgoing".to_string(), info(false, true, false, false)),
        ("Outputs".to_string(), info(false, false, true, false)),
        ("Params".to_string(), info(false, false, false, true)),
        ("Interface".to_string(), info(false, false, true, true)),
        ("Plain".to_string(), info(false, false, false, false)),
        ("DepsUp".to_string(), info(true, false, true, false)),
    ]);

    let lines = format_resources(&refs, None, Some(&deps)).await.unwrap();

    let line = |id: &str| lines[ids.iter().position(|i| *i == id).unwrap()].clone();
    assert!(line("Both").contains(&format!("{EMOJI_BIDIRECTIONAL}  ")));
    assert!(line("Incoming").contains(&format!("{EMOJI_INCOMING}  ")));
    assert!(line("Outgoing").contains(EMOJI_OUTGOING));
    assert!(line("Outputs").contains(&format!("  {EMOJI_OUTPUTS}")));
    assert!(line("Params").contains(&format!("  {EMOJI_PARAMETERS}")));
    assert!(line("Interface").contains(EMOJI_BOTH_STACK_INTERFACE));
    assert!(line("DepsUp").contains(&format!("{EMOJI_INCOMING} {EMOJI_OUTPUTS}")));
    assert!(line("Plain").starts_with("AWS::S3::Bucket     "));
    assert!(line("Unknown").starts_with("AWS::S3::Bucket     "));

    // With renames
    let renamed = format_resources(
        &refs[..2],
        Some(HashMap::from([("Both".to_string(), "New".to_string())])),
        Some(&deps),
    )
    .await
    .unwrap();
    assert!(renamed[0].contains("Both     ► New"), "{}", renamed[0]);
    assert!(!renamed[1].contains('►'));
}

#[tokio::test]
async fn test_format_resources_single_marker_width() {
    let a = summary("A", "AWS::S3::Bucket");
    let b = summary("B", "AWS::S3::Bucket");
    let deps = HashMap::from([
        ("A".to_string(), info(false, false, true, false)),
        ("B".to_string(), info(false, true, false, false)),
    ]);

    let lines = format_resources(&[&a, &b], None, Some(&deps))
        .await
        .unwrap();

    assert_eq!(
        lines[0],
        format!("AWS::S3::Bucket  {EMOJI_OUTPUTS}  A  A-physical")
    );
    assert_eq!(
        lines[1],
        format!("AWS::S3::Bucket  {EMOJI_OUTGOING}  B  B-physical")
    );
}

#[tokio::test]
async fn test_format_resources_without_markers() {
    let a = summary("A", "AWS::S3::Bucket");
    let deps = HashMap::from([("A".to_string(), info(false, false, false, false))]);

    let plain = format_resources(&[&a], None, Some(&deps)).await.unwrap();
    assert_eq!(plain[0], "AWS::S3::Bucket A  A-physical");

    let renamed = format_resources(
        &[&a],
        Some(HashMap::from([("A".to_string(), "B".to_string())])),
        None,
    )
    .await
    .unwrap();
    assert_eq!(renamed[0], "AWS::S3::Bucket A ► B   A-physical");
}

#[test]
fn test_retain_and_remove_resources() {
    let template = json!({"Resources": {"A": {"Type": "T"}, "B": {"Type": "T"}}});

    let retained = retain_resources(template.clone(), vec!["A".into(), "Missing".into()]);
    assert_eq!(retained["Resources"]["A"]["DeletionPolicy"], "Retain");
    assert!(retained["Resources"]["B"].get("DeletionPolicy").is_none());

    let removed = remove_resources(template, vec!["A".into(), "Missing".into()]);
    assert_eq!(removed, json!({"Resources": {"B": {"Type": "T"}}}));
}

#[test]
fn test_set_default_deletion_policy() {
    let template = json!({"Resources": {
        "Cluster": {"Type": "AWS::RDS::DBCluster"},
        "ClusterInstance": {
            "Type": "AWS::RDS::DBInstance",
            "Properties": {"DBClusterIdentifier": "c"}
        },
        "Instance": {"Type": "AWS::RDS::DBInstance"},
        "Bucket": {"Type": "AWS::S3::Bucket"},
        "Kept": {"Type": "AWS::S3::Bucket", "DeletionPolicy": "Retain"},
        "Odd": "not an object"
    }});
    let ids = [
        "Cluster",
        "ClusterInstance",
        "Instance",
        "Bucket",
        "Kept",
        "Odd",
        "Missing",
    ];

    let result = set_default_deletion_policy(template, ids.iter().map(|s| s.to_string()).collect());

    let policy = |id: &str| result["Resources"][id]["DeletionPolicy"].clone();
    assert_eq!(policy("Cluster"), "Snapshot");
    assert_eq!(policy("ClusterInstance"), "Delete");
    assert_eq!(policy("Instance"), "Snapshot");
    assert_eq!(policy("Bucket"), "Delete");
    assert_eq!(policy("Kept"), "Retain");
    assert_eq!(result["Resources"]["Odd"], "not an object");
}

#[test]
fn test_add_resources() {
    let target = json!({"Resources": {"T": {"Type": "AWS::SNS::Topic"}}});
    let source = json!({"Resources": {"A": {"Type": "AWS::S3::Bucket"}}});
    let map = HashMap::from([
        ("A".to_string(), "B".to_string()),
        ("Missing".to_string(), "X".to_string()),
    ]);

    let (with_policy, without) = add_resources(target, source, map);

    assert_eq!(with_policy["Resources"]["B"]["DeletionPolicy"], "Delete");
    assert!(without["Resources"]["B"].get("DeletionPolicy").is_none());
    assert!(without["Resources"].get("X").is_none());
    assert!(without["Resources"].get("T").is_some());
}

// ---------------------------------------------------------------------------
// Remaining error paths of the CLI flow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_run_rename_input_fails() {
    let dir = tempfile::tempdir().unwrap();
    let fake = FakeCfn::new(same_stack_setup(&source_template()));
    // A confirm answer where the new logical ID is asked for makes the prompt fail
    let prompter = ScriptedPrompter::new(vec![
        Answer::MultiSelect(Some(vec![0])),
        Answer::Confirm(Some(true)),
    ]);

    let result = run(
        &fake,
        &prompter,
        &["-s", "Source", "-t", "Source"],
        dir.path(),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(prompter.prompts.borrow().len(), 2);
}

#[tokio::test]
async fn test_run_export_into_file_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    fs::write(&file, "").unwrap();
    let out = file.to_str().unwrap();
    let export = ["--yes", "--export", "--out-dir", out];

    let same: Vec<&str> = [
        &["-s", "Source", "-t", "Source", "-r", "Bucket:New"][..],
        &export,
    ]
    .concat();
    let cross: Vec<&str> = [
        &["-s", "Source", "-t", "Target", "-r", "Bucket:New"][..],
        &export,
    ]
    .concat();
    let import: Vec<&str> = import_args(&["-r", "Bucket:New", "--export", "--out-dir", out]);
    let cases = vec![
        (same, same_stack_setup(&source_template())),
        (cross, cross_stack_setup(&source_template())),
        (
            import,
            with(cross_stack_setup(&source_template()), four_validations()),
        ),
    ];

    for (extra, replies) in cases {
        let fake = FakeCfn::new(replies);

        let err = run(&fake, &no_answers(), &extra, dir.path())
            .await
            .unwrap_err();

        assert!(
            err.to_string()
                .starts_with("Path exists but is not a directory"),
            "{extra:?}: {err}"
        );
    }
}

#[tokio::test]
async fn test_run_import_spec_with_resource_missing_on_stack() {
    let dir = tempfile::tempdir().unwrap();
    let spec = dir.path().join("spec.json");
    fs::write(
        &spec,
        r#"{"resources": {"Bucket": "Moved", "Extra": "Extra"}}"#,
    )
    .unwrap();
    // "Extra" is in the template but not among the stack resources
    let source = json!({"Resources": {
        "Bucket": {"Type": "AWS::S3::Bucket"},
        "Queue": {"Type": "AWS::SQS::Queue"},
        "Extra": {"Type": "AWS::SNS::Topic"}
    }});
    let fake = FakeCfn::new(with(cross_stack_setup(&source), four_validations()));
    let out_dir = dir.path().join("out");

    run(
        &fake,
        &no_answers(),
        &import_args(&[
            "--migration-spec",
            spec.to_str().unwrap(),
            "--export",
            "--out-dir",
            out_dir.to_str().unwrap(),
        ]),
        dir.path(),
    )
    .await
    .unwrap();

    assert_eq!(files_in(&out_dir).len(), 4);
}

// ---------------------------------------------------------------------------
// File and template helpers
// ---------------------------------------------------------------------------

#[test]
fn test_format_timestamp() {
    assert_eq!(format_timestamp(0), "19700101-000000");
    assert_eq!(format_timestamp(951_786_123), "20000229-010203");
    assert_eq!(format_timestamp(1_677_628_800), "20230301-000000");
    assert_eq!(format_timestamp(1_709_210_096), "20240229-123456");
    assert_eq!(format_timestamp(1_735_689_599), "20241231-235959");
}

#[test]
fn test_resolve_file_collision_without_extension() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file"), "").unwrap();

    let path = resolve_file_collision(dir.path(), "file").unwrap();

    assert_eq!(path, dir.path().join("file.1"));
}

#[test]
fn test_resolve_file_collision_too_many() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("t.json"), "").unwrap();
    for i in 1..=100 {
        fs::write(dir.path().join(format!("t.{i}.json")), "").unwrap();
    }

    let err = resolve_file_collision(dir.path(), "t.json").unwrap_err();

    assert_eq!(
        err.to_string(),
        "Too many file collisions (tried up to .100)"
    );
}

#[test]
fn test_read_template_from_file_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let tagged = dir.path().join("tagged.yaml");
    fs::write(
        &tagged,
        "Resources:\n  Bucket:\n    Type: AWS::S3::Bucket\n    Properties:\n      BucketName: !Ref Name\n",
    )
    .unwrap();
    let invalid = dir.path().join("invalid.yaml");
    fs::write(&invalid, "a: [b\n").unwrap();

    let template = read_template_from_file(&tagged).unwrap();
    let err = read_template_from_file(&invalid).unwrap_err().to_string();

    assert!(matches!(template.format, TemplateFormat::Yaml));
    assert_eq!(
        template.content["Resources"]["Bucket"]["Properties"]["BucketName"],
        json!({"Ref": "Name"})
    );
    assert!(err.starts_with("Failed to parse template file"), "{err}");
}

#[test]
fn test_validate_output_directory_under_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file"), "").unwrap();

    let err = validate_output_directory(Path::new("file/sub"), dir.path()).unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Failed to create output directory"));
}

#[cfg(unix)]
#[test]
fn test_validate_output_directory_not_writable() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    // Root can write anyway, so validation passes then
    let probe = locked.join("probe");
    let writable = fs::File::create(&probe).is_ok();
    let _ = fs::remove_file(&probe);

    let result = validate_output_directory(&locked, dir.path());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    let message = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert_eq!(message.starts_with("Directory is not writable"), !writable);
}

#[test]
fn test_validate_move_references_outputs() {
    let template = json!({
        "Resources": {
            "Bucket": {"Type": "AWS::S3::Bucket"},
            "Queue": {"Type": "AWS::SQS::Queue"}
        },
        "Outputs": {
            "BucketName": {"Value": {"Ref": "Bucket"}},
            "QueueUrl": {"Value": {"Ref": "Queue"}}
        }
    });
    let map = HashMap::from([("Bucket".to_string(), "Bucket".to_string())]);

    let err = validate_move_references(&template, &map)
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("Output section references resource 'Bucket'"),
        "{err}"
    );
    assert!(!err.contains("'Queue'"), "{err}");
}
