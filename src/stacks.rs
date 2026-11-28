//! Reading stacks, resources and templates from CloudFormation

use crate::supported_resource_types;
use crate::template::{parse_template_str, read_template_from_file, Template};
use aws_sdk_cloudformation as cloudformation;
use cloudformation::types::{StackResourceSummary, StackStatus, StackSummary};
use std::error::Error;
use std::path::PathBuf;

const DEMO: bool = false;

/// Stack states that list_stacks should return (everything but deleted stacks)
const LISTED_STACK_STATES: [StackStatus; 21] = [
    StackStatus::CreateInProgress,
    StackStatus::CreateFailed,
    StackStatus::CreateComplete,
    StackStatus::RollbackInProgress,
    StackStatus::RollbackFailed,
    StackStatus::RollbackComplete,
    StackStatus::DeleteFailed,
    StackStatus::UpdateInProgress,
    StackStatus::UpdateCompleteCleanupInProgress,
    StackStatus::UpdateComplete,
    StackStatus::UpdateFailed,
    StackStatus::UpdateRollbackInProgress,
    StackStatus::UpdateRollbackFailed,
    StackStatus::UpdateRollbackCompleteCleanupInProgress,
    StackStatus::UpdateRollbackComplete,
    StackStatus::ReviewInProgress,
    StackStatus::ImportInProgress,
    StackStatus::ImportComplete,
    StackStatus::ImportRollbackInProgress,
    StackStatus::ImportRollbackFailed,
    StackStatus::ImportRollbackComplete,
];

pub(crate) async fn get_stacks(
    client: &cloudformation::Client,
) -> Result<Vec<StackSummary>, Box<cloudformation::Error>> {
    let mut stacks = Vec::new();
    let mut token = None;

    loop {
        let resp = client
            .list_stacks()
            .set_next_token(token)
            .set_stack_status_filter(Some(LISTED_STACK_STATES.to_vec()))
            .send()
            .await
            .map_err(|e| Box::new(cloudformation::Error::from(e)))?;

        stacks.extend_from_slice(resp.stack_summaries());

        token = resp.next_token().map(str::to_owned);
        if token.is_none() {
            break;
        }
    }

    stacks.retain(|stack| !stack.stack_status().unwrap().as_str().starts_with("DELETE"));

    // Sort the stacks by name
    let mut sorted_stacks = filter_demo_stacks(stacks, DEMO);
    sorted_stacks.sort_by_key(|stack| stack.stack_name().unwrap_or_default().to_string());

    Ok(sorted_stacks)
}

/// For recording the demo, only the test stacks are shown
pub(crate) fn filter_demo_stacks(stacks: Vec<StackSummary>, demo: bool) -> Vec<StackSummary> {
    if !demo {
        return stacks;
    }
    stacks
        .into_iter()
        .filter(|stack| {
            stack
                .stack_name()
                .unwrap_or_default()
                .contains("CfnTeleportTest")
        })
        .collect()
}

pub(crate) async fn get_resources(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Vec<StackResourceSummary>, Box<dyn Error>> {
    let resp = client
        .list_stack_resources()
        .stack_name(stack_name)
        .send()
        .await?;

    let mut resources = resp
        .stack_resource_summaries()
        .iter()
        .filter(|resource| {
            let resource_type = resource.resource_type().unwrap_or_default();
            supported_resource_types::SUPPORTED_RESOURCE_TYPES.contains(&resource_type)
        })
        .cloned()
        .collect::<Vec<_>>();

    // Sort the resources by type, logical ID, and name
    resources.sort_by_key(|resource| {
        (
            resource.resource_type().unwrap_or_default().to_string(),
            resource
                .logical_resource_id()
                .unwrap_or_default()
                .to_string(),
            resource
                .physical_resource_id()
                .unwrap_or_default()
                .to_string(),
        )
    });

    Ok(resources)
}

pub(crate) async fn filter_resources<'a>(
    resources: &'a [&StackResourceSummary],
    filter: &[String],
) -> Result<Vec<&'a StackResourceSummary>, Box<dyn Error>> {
    Ok(resources
        .iter()
        .filter(|resource| {
            let logical_id = resource.logical_resource_id().unwrap_or_default();
            filter.iter().any(|id| id == logical_id)
        })
        .copied()
        .collect())
}

/// Splits `ID[:NEW_ID]` into the old and new logical ID
pub(crate) fn split_ids(id: String) -> (String, String) {
    match id.split_once(':') {
        Some((old_id, rest)) => {
            // Anything after a second colon is ignored
            let new_id = rest.split(':').next().unwrap_or_default();
            (old_id.to_string(), new_id.to_string())
        }
        None => (id.clone(), id),
    }
}

pub(crate) async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
    template_file: Option<&PathBuf>,
) -> Result<Template, Box<dyn Error>> {
    // If template file is provided, read from file instead of AWS
    if let Some(file_path) = template_file {
        return read_template_from_file(file_path);
    }

    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template_str = resp.template_body().ok_or("No template found")?;

    parse_template_str(template_str, "template")
}

pub(crate) async fn validate_template(
    client: &cloudformation::Client,
    template: serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    client
        .validate_template()
        .template_body(serde_json::to_string(&template).unwrap())
        .send()
        .await?;
    Ok(())
}
