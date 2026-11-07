//! Stack updates and import change sets

use crate::spinner;
use crate::template::{Template, TemplateFormat};
use aws_sdk_cloudformation as cloudformation;
use cloudformation::types::{
    Capability, ChangeSetStatus, ChangeSetType, ResourceToImport, StackResourceSummary, StackStatus,
};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

// @TODO: we can detect the required capabilities from the output of validate_template()
const CAPABILITIES: [Capability; 3] = [
    Capability::CapabilityIam,
    Capability::CapabilityNamedIam,
    Capability::CapabilityAutoExpand,
];

pub(crate) async fn update_stack(
    client: &cloudformation::Client,
    stack_name: &str,
    template: serde_json::Value,
    format: TemplateFormat,
) -> Result<(), Box<dyn Error>> {
    let template_body = Template::new(template, format).to_string()?;

    client
        .update_stack()
        .stack_name(stack_name)
        .template_body(template_body)
        .set_capabilities(Some(CAPABILITIES.to_vec()))
        .send()
        .await?;
    Ok(())
}

pub(crate) async fn get_stack_status(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Option<StackStatus>, Box<dyn Error>> {
    let output = client
        .describe_stacks()
        .stack_name(stack_name)
        .send()
        .await?;

    match output.stacks().first() {
        Some(stack) => Ok(stack.stack_status.clone()),
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to determine stack status",
        ))),
    }
}

pub(crate) async fn wait_for_stack_update_completion(
    client: &cloudformation::Client,
    stack_name: &str,
    spinner: Option<spinner::Spin>,
) -> Result<(), Box<dyn Error>> {
    while let Some(status) = get_stack_status(client, stack_name).await? {
        match status {
            StackStatus::UpdateInProgress
            | StackStatus::UpdateCompleteCleanupInProgress
            | StackStatus::ImportInProgress => tokio::time::sleep(Duration::from_secs(1)).await,
            StackStatus::UpdateComplete | StackStatus::ImportComplete => break,
            failed => return Err(format!("Stack update failed {}", failed.as_str()).into()),
        }
    }

    if let Some(mut spinner) = spinner {
        spinner.complete();
    }

    Ok(())
}

pub(crate) async fn get_resource_identifier_mapping(
    client: &cloudformation::Client,
    template_body: &str,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let output = client
        .get_template_summary()
        .template_body(template_body)
        .send()
        .await?;

    let mut map = HashMap::new();
    for item in output.resource_identifier_summaries() {
        for logical_id in item.logical_resource_ids() {
            let resource_identifier = item.resource_identifiers().first().unwrap();
            map.insert(logical_id.to_string(), resource_identifier.to_string());
        }
    }
    Ok(map)
}

pub(crate) async fn create_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    template: Template,
    resources_to_import: Vec<&StackResourceSummary>,
    new_logical_ids_map: HashMap<String, String>,
) -> Result<String, Box<dyn Error>> {
    let template_string = template.to_string()?;
    let resource_identifiers = get_resource_identifier_mapping(client, &template_string).await?;
    let resources = resources_to_import
        .iter()
        .map(|resource| resource_to_import(resource, &new_logical_ids_map, &resource_identifiers))
        .collect::<Vec<_>>();

    let change_set_name = format!("{}-{}", stack_name, Uuid::new_v4());

    client
        .create_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name.clone())
        .template_body(template_string)
        .change_set_type(ChangeSetType::Import)
        .set_resources_to_import(Some(resources))
        .set_capabilities(Some(CAPABILITIES.to_vec()))
        .send()
        .await?;

    Ok(change_set_name)
}

/// The import definition of a resource under its new logical ID
fn resource_to_import(
    resource: &StackResourceSummary,
    new_logical_ids_map: &HashMap<String, String>,
    resource_identifiers: &HashMap<String, String>,
) -> ResourceToImport {
    let logical_id = resource.logical_resource_id().unwrap_or_default();
    let logical_id_new = new_logical_ids_map
        .get(logical_id)
        .map_or(logical_id, String::as_str);
    let physical_id = resource.physical_resource_id().unwrap_or_default();
    let resource_identifier = resource_identifiers.get(logical_id_new).unwrap();

    ResourceToImport::builder()
        .resource_type(resource.resource_type().unwrap_or_default())
        .logical_resource_id(logical_id_new)
        .set_resource_identifier(Some(HashMap::from([(
            resource_identifier.to_string(),
            physical_id.to_string(),
        )])))
        .build()
}

pub(crate) async fn execute_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    change_set_name: &str,
) -> Result<(), Box<dyn Error>> {
    client
        .execute_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name)
        .send()
        .await?;
    Ok(())
}

pub(crate) async fn get_changeset_status(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<Option<ChangeSetStatus>, Box<dyn Error>> {
    let change_set = client
        .describe_change_set()
        .stack_name(stack_name)
        .change_set_name(changeset_name)
        .send()
        .await?;

    if change_set.status == Some(ChangeSetStatus::Failed) {
        println!("{:?}", change_set);
        return Err(change_set.status_reason().unwrap().to_string().into());
    }

    Ok(change_set.status)
}

pub(crate) async fn wait_for_changeset_created(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<(), Box<dyn Error>> {
    while let Some(status) = get_changeset_status(client, stack_name, changeset_name).await? {
        match status {
            ChangeSetStatus::CreateInProgress | ChangeSetStatus::CreatePending => {
                tokio::time::sleep(Duration::from_secs(1)).await
            }
            ChangeSetStatus::CreateComplete => break,
            failed => return Err(format!("Changeset creation failed {}", failed.as_str()).into()),
        }
    }

    Ok(())
}
