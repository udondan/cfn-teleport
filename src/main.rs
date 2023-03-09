use aws_sdk_cloudformation as cloudformation;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, MultiSelect, Select};
use std::error::Error;
use uuid::Uuid;
mod supported_resource_types;
use std::collections::HashMap;
use std::io;
use std::io::Write;

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

    if resources.is_empty() {
        return Err(format!("No resources found in stack '{}'", source_stack).into());
    }

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

    if source_stack == target_stack {
        let mut duplicate_ids = Vec::new();
        for (old_id, new_id) in &new_logical_ids_map {
            if old_id == new_id {
                duplicate_ids.push(old_id);
            }
        }

        if !duplicate_ids.is_empty() {
            let error_message = format!(
                "Unable to proceed, because you said you want to rename resources in stack {} but did not provide new logical IDs for: {}",
                source_stack,
                duplicate_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
            );
            return Err(error_message.into());
        }

        println!(
            "The following resources in stack {} will be renamed:",
            source_stack
        );
    } else {
        println!(
            "The following resources will be moved from stack {} to {}:",
            source_stack, target_stack
        );
    }

    for resource in format_resources(&selected_resources).await? {
        println!("  {}", resource);
    }

    user_confirm()?;

    let template_source = get_template(&client, source_stack).await?;
    let template_source_str = serde_json::to_string(&template_source)?;

    let resource_ids_to_remove: Vec<_> = new_logical_ids_map.keys().cloned().collect();

    let template_retained =
        retain_resources(template_source.clone(), resource_ids_to_remove.clone());
    let template_retained_str = serde_json::to_string(&template_retained)?;

    if template_source_str != template_retained_str {
        //@TODO: this output is not accurate. if the tmeplate has changed, it only means at least one of the resource will be rateind, not neccessarily all selecteed resources
        print!("Retaining resources {}", resource_ids_to_remove.join(", "));
        update_stack(&client, source_stack, template_retained).await?;
        wait_for_stack_update_completion(&client, source_stack).await?;
    }

    let template_removed =
        remove_resources(template_source.clone(), resource_ids_to_remove.clone());
    print!("Removing resources {}", resource_ids_to_remove.join(", "));
    update_stack(&client, source_stack, template_removed).await?;
    wait_for_stack_update_completion(&client, source_stack).await?;

    let template_target = add_resources(
        get_template(&client, target_stack).await?,
        template_source.clone(),
        new_logical_ids_map,
    );

    let changeset_name =
        create_changeset(&client, target_stack, template_target, selected_resources).await?;
    print!("Creating changeset {}", changeset_name);
    wait_for_changeset_created(&client, target_stack, &changeset_name).await?;

    print!("Executing changeset {}", changeset_name);
    execute_changeset(&client, target_stack, &changeset_name).await?;
    wait_for_stack_update_completion(&client, target_stack).await?;

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
) -> Result<serde_json::Value, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    let parsed_template = serde_json::from_str(&template)?;
    Ok(parsed_template)
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

fn retain_resources(mut template: serde_json::Value, resource_ids: Vec<&str>) -> serde_json::Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        if let Some(resource) = resources.get_mut(resource_id) {
            resource["DeletionPolicy"] = serde_json::Value::String("Retain".to_string());
        }
    }

    template
}

fn remove_resources(mut template: serde_json::Value, resource_ids: Vec<&str>) -> serde_json::Value {
    let resources = template["Resources"].as_object_mut().unwrap();

    for resource_id in resource_ids {
        resources.remove(resource_id);
    }

    template
}

fn add_resources(
    mut target_template: serde_json::Value,
    source_template: serde_json::Value,
    resource_id_map: HashMap<&str, String>,
) -> serde_json::Value {
    let target_resources = target_template["Resources"].as_object_mut().unwrap();
    let source_resources = source_template["Resources"].as_object().unwrap();

    for (resource_id, new_resource_id) in resource_id_map {
        if let Some(resource) = source_resources.get(resource_id) {
            target_resources.insert(new_resource_id, resource.clone());
        }
    }

    target_template
}

async fn update_stack(
    client: &cloudformation::Client,
    stack_name: &str,
    template: serde_json::Value,
) -> Result<(), cloudformation::Error> {
    match client
        .update_stack()
        .stack_name(stack_name)
        .template_body(serde_json::to_string(&template).unwrap())
        .send()
        .await
    {
        Ok(_output) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn get_stack_status(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<Option<cloudformation::model::StackStatus>, Box<dyn std::error::Error>> {
    let describe_stacks_output = match client.describe_stacks().stack_name(stack_name).send().await
    {
        Ok(output) => output,
        Err(err) => return Err(Box::new(err)),
    };

    let stacks = describe_stacks_output.stacks().unwrap_or_default();
    let stack = stacks.first();

    if let Some(stack) = stack {
        Ok(stack.stack_status.clone())
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to determine stack status",
        )))
    }
}

async fn wait_for_stack_update_completion(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stack_status = get_stack_status(&client, stack_name).await?;

    while let Some(status) = stack_status.clone() {
        if status == cloudformation::model::StackStatus::UpdateInProgress
            || status == cloudformation::model::StackStatus::UpdateCompleteCleanupInProgress
            || status == cloudformation::model::StackStatus::ImportInProgress
        {
            print!(".");
            std::io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_secs(1));
            stack_status = get_stack_status(&client, stack_name).await?;
        } else {
            if status != cloudformation::model::StackStatus::UpdateComplete
                && status != cloudformation::model::StackStatus::ImportComplete
            {
                return Err(
                    format!("Stack update failed {}", stack_status.unwrap().as_str()).into(),
                );
            }
            break;
        }
    }

    println!(" {}", stack_status.unwrap().as_str());
    Ok(())
}

async fn get_resource_identifier_mapping(
    client: &cloudformation::Client,
    template_body: &str,
) -> Result<HashMap<String, String>, cloudformation::Error> {
    match client
        .get_template_summary()
        .template_body(template_body)
        .send()
        .await
    {
        Ok(output) => {
            let mut map = HashMap::new();
            for item in output.resource_identifier_summaries().iter() {
                item.iter().for_each(|item| {
                    item.logical_resource_ids()
                        .unwrap()
                        .iter()
                        .for_each(|logical_id| {
                            item.resource_identifiers()
                                .unwrap()
                                .iter()
                                .for_each(|resource_id| {
                                    map.insert(logical_id.to_string(), resource_id.to_string());
                                });
                        });
                });
            }
            Ok(map)
        }
        Err(err) => Err(err.into()),
    }
}

async fn create_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    template: serde_json::Value,
    resources_to_import: Vec<&cloudformation::model::StackResourceSummary>,
) -> Result<std::string::String, cloudformation::Error> {
    let template_string = serde_json::to_string(&template).unwrap();
    let resource_identifiers = get_resource_identifier_mapping(&client, &template_string).await?;
    let resources = resources_to_import
        .iter()
        .map(|resource| {
            let resource_type = resource.resource_type().unwrap_or_default();
            let logical_id = resource.logical_resource_id().unwrap_or_default();
            let physical_id = resource.physical_resource_id().unwrap_or_default();
            let resouce_identifier = resource_identifiers.get(logical_id).unwrap();

            cloudformation::model::ResourceToImport::builder()
                .resource_type(resource_type.to_string())
                .logical_resource_id(logical_id.to_string())
                .set_resource_identifier(Some(
                    vec![(resouce_identifier.to_string(), physical_id.to_string())]
                        .into_iter()
                        .collect(),
                ))
                .build()
        })
        .collect::<Vec<_>>();

    let change_set_name = format!("{}-{}", stack_name, Uuid::new_v4());

    match client
        .create_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name.clone())
        .template_body(template_string)
        .change_set_type(cloudformation::model::ChangeSetType::Import)
        .set_resources_to_import(resources.into())
        .send()
        .await
    {
        Ok(_) => Ok(change_set_name),
        Err(err) => Err(err.into()),
    }
}

async fn execute_changeset(
    client: &cloudformation::Client,
    stack_name: &str,
    change_set_name: &str,
) -> Result<(), cloudformation::Error> {
    match client
        .execute_change_set()
        .stack_name(stack_name)
        .change_set_name(change_set_name)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn get_changeset_status(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<Option<cloudformation::model::ChangeSetStatus>, Box<dyn std::error::Error>> {
    let change_set = match client
        .describe_change_set()
        .stack_name(stack_name)
        .change_set_name(changeset_name)
        .send()
        .await
    {
        Ok(output) => output,
        Err(err) => return Err(Box::new(err)),
    };

    Ok(change_set.status.clone())
}

async fn wait_for_changeset_created(
    client: &cloudformation::Client,
    stack_name: &str,
    changeset_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut changeset_status = get_changeset_status(&client, stack_name, changeset_name).await?;

    while let Some(status) = changeset_status.clone() {
        if status == cloudformation::model::ChangeSetStatus::CreateInProgress
            || status == cloudformation::model::ChangeSetStatus::CreatePending
        {
            print!(".");
            std::io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_secs(1));
            changeset_status = get_changeset_status(&client, stack_name, changeset_name).await?;
        } else {
            if status != cloudformation::model::ChangeSetStatus::CreateComplete {
                return Err(format!(
                    "Changeset creation failed {}",
                    changeset_status.unwrap().as_str()
                )
                .into());
            }
            break;
        }
    }

    println!(" {}", changeset_status.unwrap().as_str());
    Ok(())
}
