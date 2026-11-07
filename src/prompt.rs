use console::style;
use dialoguer::{console::Term, theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::error::Error;

/// Asks the user for input. The CLI uses dialoguer, tests use scripted answers.
pub(crate) trait Prompter {
    /// Returns the index of the selected item, or None if the user aborted.
    fn select(&self, prompt: &str, items: &[&str]) -> Result<Option<usize>, Box<dyn Error>>;

    /// Returns the indices of the selected items, or None if the user aborted.
    fn multi_select(
        &self,
        prompt: &str,
        items: &[String],
    ) -> Result<Option<Vec<usize>>, Box<dyn Error>>;

    /// Returns the entered text, or the default if the user just pressed enter.
    fn input(&self, prompt: &str, default: &str) -> Result<String, Box<dyn Error>>;

    /// Returns the answer, or None if the user aborted.
    fn confirm(&self, prompt: &str) -> Result<Option<bool>, Box<dyn Error>>;
}

/// Interactive prompts on the terminal
pub(crate) struct DialoguerPrompter;

impl Prompter for DialoguerPrompter {
    fn select(&self, prompt: &str, items: &[&str]) -> Result<Option<usize>, Box<dyn Error>> {
        Ok(Select::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .items(items)
            .report(false)
            .default(0)
            .interact_on_opt(&Term::stderr())?)
    }

    fn multi_select(
        &self,
        prompt: &str,
        items: &[String],
    ) -> Result<Option<Vec<usize>>, Box<dyn Error>> {
        // Custom theme with grey checkmark for unchecked items
        let theme = ColorfulTheme {
            unchecked_item_prefix: style("✔".to_string()).for_stderr().dim(),
            ..ColorfulTheme::default()
        };

        Ok(MultiSelect::with_theme(&theme)
            .with_prompt(prompt)
            .report(false)
            .items(items)
            .interact_on_opt(&Term::stderr())?)
    }

    fn input(&self, prompt: &str, default: &str) -> Result<String, Box<dyn Error>> {
        Ok(Input::new()
            .with_prompt(prompt)
            .default(default.to_string())
            .interact_text()?)
    }

    fn confirm(&self, prompt: &str) -> Result<Option<bool>, Box<dyn Error>> {
        Ok(Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact_on_opt(&Term::stderr())?)
    }
}

pub(crate) fn select_stack<'a>(
    prompter: &dyn Prompter,
    prompt: &str,
    items: &'a [&str],
) -> Result<&'a str, Box<dyn Error>> {
    match prompter.select(prompt, items)? {
        Some(index) => Ok(items[index]),
        None => Err("User did not select anything".into()),
    }
}

pub(crate) fn user_confirm(prompter: &dyn Prompter) -> Result<(), Box<dyn Error>> {
    let confirmed = prompter.confirm("Please confirm your selection:")?;

    println!();

    match confirmed {
        Some(true) => Ok(()),
        _ => Err("Selection has not been confirmed".into()),
    }
}
