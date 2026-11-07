//! Moving and renaming resources with the CloudFormation Stack Refactoring API

use crate::export::{save_debug_artifacts, ErrorContext};
use crate::move_checks::validate_move_references;
use crate::reference_updater;
use crate::spinner;
use crate::stacks::validate_template;
use crate::template::Template;
use crate::template_edit::{add_resources, remove_resources, rename_resources};
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use cloudformation::operation::describe_stack_refactor::DescribeStackRefactorOutput;
use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::time::Duration;

/// A stack and the template it should get
pub(crate) struct StackTemplate<'a> {
    pub(crate) name: &'a str,
    pub(crate) template: Template,
}

/// Formats a CloudFormation API error with all available details.
///
/// Extracts error code and message from AWS SDK errors to provide actionable
/// information to users instead of generic "service error" messages.
pub(crate) fn format_cfn_error(operation: &str, err: &impl ProvideErrorMetadata) -> String {
    let mut parts = vec![format!("Failed to {}", operation)];

    if let Some(code) = err.code() {
        parts.push(format!("Error code: {}", code));
    }

    if let Some(msg) = err.message() {
        parts.push(format!("Message: {}", msg));
    }

    // If we only have the operation description, add a note about lack of details
    if parts.len() == 1 {
        parts.push("(No additional error details provided by AWS)".to_string());
    }

    parts.join("\n")
}

/// "s" for counts other than one
pub(crate) fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

/// Source and target templates for a cross-stack move: the resources are removed
/// from the source, added to the target, and all references are updated
pub(crate) fn cross_stack_templates(
    source: &Template,
    target: &Template,
    id_mapping: &HashMap<String, String>,
) -> (Template, Template) {
    let resource_ids: Vec<String> = id_mapping.keys().cloned().collect();
    let source_without_resources = remove_resources(source.content.clone(), resource_ids);

    // add_resources returns (template_with_deletion_policy, template_without).
    // For refactor mode, we use the template without DeletionPolicy modifications.
    let (_, target_with_resources) = add_resources(
        target.content.clone(),
        source.content.clone(),
        id_mapping.clone(),
    );

    (
        Template::new(
            reference_updater::update_template_references(source_without_resources, id_mapping),
            source.format,
        ),
        Template::new(
            reference_updater::update_template_references(target_with_resources, id_mapping),
            target.format,
        ),
    )
}

pub(crate) async fn refactor_stack_resources(
    client: &cloudformation::Client,
    stack_name: &str,
    template: Template,
    id_mapping: HashMap<String, String>,
    work_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    // The template must have valid references for validation to pass
    let updated = Template::new(
        rename_resources(template.content, &id_mapping),
        template.format,
    );

    if let Err(e) = validate_template(client, updated.content.clone()).await {
        let error = e.to_string();
        save_debug_artifacts(
            work_dir,
            "refactor",
            &[(updated, "refactored".to_string())],
            &ErrorContext {
                error: &error,
                source_stack: stack_name,
                target_stack: None,
                operation: "refactor-same-stack",
                resources: &id_mapping,
            },
        );
        return Err(format!(
            "Updated template validation failed: {}\n\
             Template and error context have been saved for debugging.",
            e
        )
        .into());
    }

    let count = id_mapping.len();
    let spinner = spinner::Spin::new(&format!(
        "Renaming {} resource{} in stack {}",
        count,
        plural(count),
        stack_name,
    ));

    execute_stack_refactor(
        client,
        vec![stack_definition(stack_name, &updated)?],
        resource_mappings(stack_name, stack_name, &id_mapping),
        spinner,
    )
    .await?;

    println!(
        "Renamed {} resource{} in stack {}",
        count,
        plural(count),
        stack_name
    );
    print_id_mapping(&id_mapping);
    Ok(())
}

/// Refactor CloudFormation stack resources across stacks using the Stack Refactoring API.
/// This function handles cross-stack moves only (use refactor_stack_resources for same-stack).
pub(crate) async fn refactor_stack_resources_cross_stack(
    client: &cloudformation::Client,
    source: StackTemplate<'_>,
    target: StackTemplate<'_>,
    id_mapping: HashMap<String, String>,
    work_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    // Resources staying in source must not reference moving resources, and the other way round
    validate_move_references(&source.template.content, &id_mapping)?;

    let (source_final, target_final) =
        cross_stack_templates(&source.template, &target.template, &id_mapping);

    let source_validation = validate_template(client, source_final.content.clone()).await;
    let target_validation = validate_template(client, target_final.content.clone()).await;

    let validation_error = match (&source_validation, &target_validation) {
        (Err(e), _) => Some(format!("Source template validation failed: {}", e)),
        (_, Err(e)) => Some(format!("Target template validation failed: {}", e)),
        _ => None,
    };

    if let Some(error) = validation_error {
        save_debug_artifacts(
            work_dir,
            "refactor-cross-stack",
            &[
                (source_final, format!("source-{}", source.name)),
                (target_final, format!("target-{}", target.name)),
            ],
            &ErrorContext {
                error: &error,
                source_stack: source.name,
                target_stack: Some(target.name),
                operation: "refactor-cross-stack",
                resources: &id_mapping,
            },
        );
        return Err(format!(
            "{}\nTemplates and error context have been saved for debugging.",
            error
        )
        .into());
    }

    let count = id_mapping.len();
    let spinner = spinner::Spin::new(&format!(
        "Moving {} resource{} from {} to {}",
        count,
        plural(count),
        source.name,
        target.name
    ));

    execute_stack_refactor(
        client,
        vec![
            stack_definition(source.name, &source_final)?,
            stack_definition(target.name, &target_final)?,
        ],
        resource_mappings(source.name, target.name, &id_mapping),
        spinner,
    )
    .await?;

    println!(
        "Moved {} resource{} from {} to {}",
        count,
        plural(count),
        source.name,
        target.name
    );
    print_id_mapping(&id_mapping);
    Ok(())
}

fn print_id_mapping(id_mapping: &HashMap<String, String>) {
    for (old_id, new_id) in id_mapping {
        println!("  {} → {}", old_id, new_id);
    }
}

fn stack_definition(
    stack_name: &str,
    template: &Template,
) -> Result<StackDefinition, Box<dyn Error>> {
    Ok(StackDefinition::builder()
        .stack_name(stack_name)
        .template_body(template.to_string()?)
        .build())
}

/// Where each resource comes from and where it goes
fn resource_mappings(
    source_stack: &str,
    target_stack: &str,
    id_mapping: &HashMap<String, String>,
) -> Vec<ResourceMapping> {
    let location = |stack_name: &str, logical_id: &str| {
        ResourceLocation::builder()
            .stack_name(stack_name)
            .logical_resource_id(logical_id)
            .build()
    };

    id_mapping
        .iter()
        .map(|(old_id, new_id)| {
            ResourceMapping::builder()
                .source(location(source_stack, old_id))
                .destination(location(target_stack, new_id))
                .build()
        })
        .collect()
}

/// Creates the stack refactor, waits for its validation, executes it and waits for completion
async fn execute_stack_refactor(
    client: &cloudformation::Client,
    stack_definitions: Vec<StackDefinition>,
    resource_mappings: Vec<ResourceMapping>,
    mut spinner: spinner::Spin,
) -> Result<(), Box<dyn Error>> {
    let create_result = client
        .create_stack_refactor()
        .set_stack_definitions(Some(stack_definitions))
        .set_resource_mappings(Some(resource_mappings))
        .send()
        .await
        .map_err(|e| format_cfn_error("create stack refactor", &e))?;

    let refactor_id = create_result
        .stack_refactor_id()
        .ok_or("No stack refactor ID returned")?;

    wait_for_refactor(client, refactor_id, &CREATE, &mut spinner).await?;

    client
        .execute_stack_refactor()
        .stack_refactor_id(refactor_id)
        .send()
        .await
        .map_err(|e| format_cfn_error("execute stack refactor", &e))?;

    wait_for_refactor(client, refactor_id, &EXECUTE, &mut spinner).await?;
    spinner.complete();
    Ok(())
}

/// A stage of a stack refactor that has to be waited for
struct RefactorPhase {
    complete: &'static str,
    failed: &'static str,
    error: &'static str,
    poll_interval: Duration,
    status: fn(&DescribeStackRefactorOutput) -> Option<&str>,
}

const CREATE: RefactorPhase = RefactorPhase {
    complete: "CREATE_COMPLETE",
    failed: "CREATE_FAILED",
    error: "Stack refactor validation failed",
    poll_interval: Duration::from_secs(2),
    status: |output| output.status().map(|s| s.as_str()),
};

const EXECUTE: RefactorPhase = RefactorPhase {
    complete: "EXECUTE_COMPLETE",
    failed: "EXECUTE_FAILED",
    error: "Stack refactor execution failed",
    poll_interval: Duration::from_secs(5),
    status: |output| output.execution_status().map(|s| s.as_str()),
};

async fn wait_for_refactor(
    client: &cloudformation::Client,
    refactor_id: &str,
    phase: &RefactorPhase,
    spinner: &mut spinner::Spin,
) -> Result<(), Box<dyn Error>> {
    loop {
        let output = client
            .describe_stack_refactor()
            .stack_refactor_id(refactor_id)
            .send()
            .await
            .map_err(|e| format_cfn_error("describe stack refactor", &e))?;

        match (phase.status)(&output) {
            Some(status) if status == phase.complete => return Ok(()),
            Some(status) if status == phase.failed || status == "FAILED" => {
                spinner.fail();
                return Err(format!(
                    "{}: {}",
                    phase.error,
                    output.status_reason().unwrap_or("Unknown error")
                )
                .into());
            }
            _ => tokio::time::sleep(phase.poll_interval).await,
        }
    }
}
