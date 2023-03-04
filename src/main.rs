use aws_sdk_cloudformation as cloudformation;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, MultiSelect, Select};
use std::error::Error;
mod supported_resource_types;
use std::collections::HashMap;
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = aws_config::load_from_env().await;
    let client = cloudformation::Client::new(&config);
    let stacks = get_stacks(&client).await?;
    let stack_names: Vec<&str> = stacks
        .iter()
        .map(|s| s.stack_name().unwrap_or_default())
        .collect();

    let source_stack = select_stack("Select source stack", &stack_names)?;

    let resources = get_resources(&client, source_stack).await?;

    let target_stack = select_stack("Select target stack", &stack_names)?;

    let resource_refs = &resources.iter().collect::<Vec<_>>();

    let selected_resources = select_resources("Select resources to copy", resource_refs).await?;

    if selected_resources.is_empty() {
        return Err("No resources have been selected".into());
    }

    let mut new_logical_ids_map = HashMap::new();

    for resource in selected_resources.clone() {
        let old_logical_id = resource.logical_resource_id().unwrap_or_default();
        let mut new_logical_id = String::new();

        println!(
            "Provide a new logical ID for resource '{}', or leave blank to use the original ID:",
            old_logical_id
        );
        io::stdin().read_line(&mut new_logical_id)?;
        new_logical_id = new_logical_id.trim().to_string();
        if new_logical_id.is_empty() {
            new_logical_id = resource
                .logical_resource_id()
                .unwrap_or_default()
                .to_string();
        }
        new_logical_ids_map.insert(old_logical_id, new_logical_id);
    }

    println!(
        "The following resources will be moved from stack {} to {}:",
        source_stack, target_stack
    );

    for resource in format_resources(&selected_resources).await? {
        println!("  {}", resource);
    }

    user_confirm()?;

    let template = get_template(&client, source_stack).await?;

    //@TODO: check if the selected resources do have "DeletionPolicy": "Retain". If not, add it.
    //@TODO: if the template has been changed, update the stack and wait for completion
    //@TODO: remove the selected resources from the source stack template
    //@TODO: update the stack and wait for completion
    //@TODO: download the tempalte of the target stack
    //@TODO: import resources into the target stack

    println!(
        "CloudFormation template of the selected stack:\n{}\n",
        template
    );

    Ok(())
}

async fn get_stacks(
    client: &cloudformation::Client,
) -> Result<Vec<cloudformation::model::StackSummary>, cloudformation::Error> {
    let resp = client.list_stacks().send().await?;

    let stacks = resp.stack_summaries().unwrap_or_default().to_vec();

    let stacks = stacks
        .into_iter()
        .filter(|stack| !stack.stack_status().unwrap().as_str().starts_with("DELETE"))
        .collect::<Vec<_>>();

    // Sort the stacks by name
    let mut sorted_stacks = stacks.clone();
    sorted_stacks.sort_by_key(|stack| stack.stack_name().unwrap_or_default().to_string());

    Ok(sorted_stacks)
}

fn select_stack<'a>(prompt: &str, items: &'a Vec<&str>) -> Result<&'a str, Box<dyn Error>> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(items)
        .report(false)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => Ok(items[index]),
        None => Err("User did not select anything".into()),
    }
}

async fn get_resources(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Vec<cloudformation::model::StackResourceSummary>, cloudformation::Error> {
    let resp = client
        .list_stack_resources()
        .stack_name(stack_name)
        .send()
        .await?;

    let resources = resp.stack_resource_summaries().unwrap_or_default().to_vec();

    // Filter resources based on supported types
    let filtered_resources = resources
        .into_iter()
        .filter(|resource| {
            let resource_type = resource.resource_type().unwrap_or_default();
            supported_resource_types::SUPPORTED_RESOURCE_TYPES.contains(&resource_type)
        })
        .collect::<Vec<_>>();

    // Sort the resources by type, logical ID, and name
    let mut sorted_resources = filtered_resources.clone();
    sorted_resources.sort_by_key(|resource| {
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

    Ok(sorted_resources)
}

async fn select_resources<'a>(
    prompt: &str,
    resources: &'a Vec<&aws_sdk_cloudformation::model::StackResourceSummary>,
) -> Result<Vec<&'a aws_sdk_cloudformation::model::StackResourceSummary>, Box<dyn Error>> {
    let items = format_resources(resources).await?;
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .report(false)
        .items(&items)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(indices) => Ok(indices
            .into_iter()
            .map(|index| resources[index])
            .collect::<Vec<_>>()),
        None => Err("User did not select anything".into()),
    }
}

fn user_confirm() -> Result<(), Box<dyn Error>> {
    let confirmed = Confirm::new()
        .with_prompt("Please confirm your selection")
        .default(false)
        .interact_on_opt(&Term::stderr())?;

    match confirmed {
        Some(true) => Ok(()),
        _ => Err("Selection has not been cofirmed".into()),
    }
}

async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<String, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    Ok(template.to_owned())
}

async fn format_resources(
    resources: &Vec<&cloudformation::model::StackResourceSummary>,
) -> Result<Vec<String>, io::Error> {
    let mut max_lengths = [0; 3];
    let mut formatted_resources = Vec::new();

    for resource in resources.iter() {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let physical_id = resource.physical_resource_id().unwrap_or_default();

        max_lengths[0] = max_lengths[0].max(resource_type.len());
        max_lengths[1] = max_lengths[1].max(logical_id.len());
        max_lengths[2] = max_lengths[2].max(physical_id.len());
    }

    for resource in resources.iter() {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let physical_id = resource.physical_resource_id().unwrap_or_default();

        let output = format!(
            "{:<width1$}  {:<width2$}  {}",
            resource_type,
            logical_id,
            physical_id,
            width1 = max_lengths[0] + 2,
            width2 = max_lengths[1] + 2
        );

        formatted_resources.push(output);
    }

    Ok(formatted_resources)
}
