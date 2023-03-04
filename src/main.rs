use aws_sdk_cloudformation as cloudformation;
use dialoguer::{console::Term, theme::ColorfulTheme, Select};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = aws_config::load_from_env().await;
    let client = cloudformation::Client::new(&config);
    let stacks = get_stacks(&client).await?;
    let stack_names: Vec<&str> = stacks
        .iter()
        .map(|s| s.stack_name().unwrap_or_default())
        .collect();

    let source_stack = select_stack("Select source stack", stack_names)?;

    println!("{:?}", source_stack);

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

    Ok(stacks)
}

fn select_stack(prompt: &str, items: Vec<&str>) -> Result<&str, Box<dyn Error>> {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact_on_opt(&Term::stderr())?;

    match selection {
        Some(index) => Ok(items[index]),
        None => Err("User did not select anything".into()),
    }
}
