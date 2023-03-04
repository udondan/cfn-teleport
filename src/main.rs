use aws_sdk_cloudformation as cloudformation;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, MultiSelect, Select};
use std::error::Error;
mod supported_resource_types;

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

    let mut max_lengths = [0; 3];

    let mut formatted_resources = Vec::new();

    for resource in &resources {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let physical_id = resource.physical_resource_id().unwrap_or_default();

        max_lengths[0] = max_lengths[0].max(resource_type.len());
        max_lengths[1] = max_lengths[1].max(logical_id.len());
        max_lengths[2] = max_lengths[2].max(physical_id.len());
    }

    for resource in &resources {
        let resource_type = resource.resource_type().unwrap_or_default();
        let logical_id = resource.logical_resource_id().unwrap_or_default();
        let physical_id = resource.physical_resource_id().unwrap_or_default();

        let output = format!(
            "{:<width1$}  {:<width2$}  {}",
            resource_type,
            logical_id,
            physical_id,
            width1 = max_lengths[0] + 2,
            width2 = max_lengths[1] + 2,
        );

        formatted_resources.push(output);
    }

    let resource_strings: Vec<_> = formatted_resources.iter().map(|s| s.as_str()).collect();
    let selected_resources = select_resources("Select resources to copy", &resource_strings)?;

    println!(
        "The following resources will be moved from stack {} to {}:",
        source_stack, target_stack
    );

    for resource in selected_resources {
        println!("  {}", resource);
    }

    user_confirm()?;

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

fn select_resources<'a>(
    prompt: &str,
    items: &'a Vec<&str>,
) -> Result<Vec<&'a str>, Box<dyn Error>> {
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .report(false)
        .items(items)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(indices) => Ok(indices.iter().map(|i| items[*i]).collect()),
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
