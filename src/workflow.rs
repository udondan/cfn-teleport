//! The overall flow: pick stacks and resources, confirm, then rename, move or export

use crate::cli::{Args, Mode};
use crate::display::format_resources;
use crate::export::{export_templates, print_export_result};
use crate::import::{apply_import, prepare_import, ImportJob};
use crate::move_checks::validate_move_references;
use crate::prompt::{select_stack, user_confirm, Prompter};
use crate::refactor::{
    cross_stack_templates, refactor_stack_resources, refactor_stack_resources_cross_stack,
    StackTemplate,
};
use crate::selection::{choose_resources, Stacks};
use crate::stacks::{get_resources, get_stacks, get_template};
use crate::template::Template;
use crate::template_edit::{rename_resources, section_keys};
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use aws_sdk_cloudformation::types::StackResourceSummary;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

const CREDENTIALS_HELP: &str = "AWS credentials not found.\n\n\
Please ensure you're authenticated with AWS using one of the following methods:\n  \
• AWS CLI: Run 'aws configure'\n  \
• Environment variables: Set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY\n  \
• IAM role (if running on EC2/ECS/Lambda)\n\n\
For more information, visit:\n  \
https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-files.html";

/// Runs the move or rename. `work_dir` is where export and debug files go by default.
pub(crate) async fn run_with(
    args: Args,
    client: &cloudformation::Client,
    prompter: &dyn Prompter,
    work_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let stack_list = get_stacks(client).await.map_err(|err| stacks_error(&err))?;
    let stack_names: Vec<&str> = stack_list
        .iter()
        .map(|s| s.stack_name().unwrap_or_default())
        .collect();

    let source = pick_stack(
        prompter,
        args.source.clone(),
        "Select source stack",
        &stack_names,
    )?;

    let resources = get_resources(client, &source).await?;
    if resources.is_empty() {
        return Err(format!("No resources found in stack '{}'", source).into());
    }

    let target = pick_stack(
        prompter,
        args.target.clone(),
        "Select target stack",
        &stack_names,
    )?;

    let source_template = get_template(client, &source, args.source_template.as_ref()).await?;
    // The target template is needed early for cross-stack parameter validation
    let target_template = if source != target {
        Some(get_template(client, &target, args.target_template.as_ref()).await?)
    } else {
        None
    };

    let stacks = Stacks {
        source,
        target,
        source_template,
        target_template,
    };

    let resource_refs: Vec<&StackResourceSummary> = resources.iter().collect();
    let (selected, id_map) = choose_resources(&args, prompter, &stacks, &resource_refs).await?;

    print_plan(&stacks, &selected, &id_map).await?;

    if !args.yes {
        user_confirm(prompter)?;
    }

    let job = Job {
        client,
        work_dir,
        out_dir: args.out_dir.as_ref(),
        export: args.export,
        source_stack: &stacks.source,
        target_stack: &stacks.target,
        id_map,
    };

    match stacks.target_template {
        None => job.run_same_stack(stacks.source_template).await,
        Some(target_template) => {
            validate_move_references(&stacks.source_template.content, &job.id_map)?;
            match args.mode {
                Mode::Refactor => {
                    job.run_cross_stack_refactor(stacks.source_template, target_template)
                        .await
                }
                Mode::Import => {
                    job.run_import(selected, stacks.source_template, target_template)
                        .await
                }
            }
        }
    }
}

/// Turns a failure to list stacks into a helpful message, especially for missing credentials
fn stacks_error(err: &cloudformation::Error) -> Box<dyn Error> {
    let is_credentials_error = std::iter::successors(err.source(), |e| (*e).source()).any(|e| {
        let text = e.to_string();
        text.contains("CredentialsNotLoaded")
            || text.contains("no providers in chain provided credentials")
    });

    if is_credentials_error {
        return CREDENTIALS_HELP.into();
    }

    let message = err.message().unwrap_or("An AWS error occurred");
    match err.code() {
        Some(code) => format!("AWS Error ({}): {}", code, message).into(),
        None => message.into(),
    }
}

/// The stack given on the command line, or one chosen by the user
fn pick_stack(
    prompter: &dyn Prompter,
    given: Option<String>,
    prompt: &str,
    stack_names: &[&str],
) -> Result<String, Box<dyn Error>> {
    match given {
        Some(stack) => Ok(stack),
        None => Ok(select_stack(prompter, prompt, stack_names)?.to_string()),
    }
}

/// Shows what will happen. Same-stack operations must rename every selected resource
/// to an ID that isn't taken yet.
async fn print_plan(
    stacks: &Stacks,
    selected: &[&StackResourceSummary],
    id_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    if stacks.is_cross_stack() {
        println!(
            "The following resources will be moved from stack {} to {}:",
            stacks.source, stacks.target
        );
    } else {
        check_same_stack_rename(&stacks.source, &stacks.source_template, id_map)?;
        println!(
            "The following resources in stack {} will be renamed:",
            stacks.source
        );
    }

    for resource in format_resources(selected, Some(id_map.clone()), None).await? {
        println!("  {}", resource);
    }
    Ok(())
}

fn check_same_stack_rename(
    stack_name: &str,
    template: &Template,
    id_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    if id_map.iter().all(|(old_id, new_id)| old_id == new_id) {
        return Err(
            "Source and target stack are the same, but no resources are being renamed. \
             Same-stack operations require renaming at least one resource."
                .into(),
        );
    }

    let not_renamed: Vec<&str> = id_map
        .iter()
        .filter(|(old_id, new_id)| old_id == new_id)
        .map(|(old_id, _)| old_id.as_str())
        .collect();

    if !not_renamed.is_empty() {
        return Err(format!(
            "Unable to proceed, because you said you want to rename resources in stack {} but did not provide new logical IDs for: {}",
            stack_name,
            not_renamed.join(", ")
        )
        .into());
    }

    // The new name must not exist in the stack yet
    let existing_resources = section_keys(&template.content, "Resources");
    let collisions: Vec<String> = id_map
        .iter()
        .filter(|(_, new_id)| existing_resources.contains(*new_id))
        .map(|(old_id, new_id)| {
            format!(
                "{} -> {} (resource '{}' already exists)",
                old_id, new_id, new_id
            )
        })
        .collect();

    if collisions.is_empty() {
        return Ok(());
    }

    Err(format!(
        "Cannot rename resources: target name(s) already exist in stack {}:\n  {}",
        stack_name,
        collisions.join("\n  ")
    )
    .into())
}

/// Everything needed to carry out the confirmed operation
struct Job<'a> {
    client: &'a cloudformation::Client,
    work_dir: &'a Path,
    out_dir: Option<&'a PathBuf>,
    export: bool,
    source_stack: &'a str,
    target_stack: &'a str,
    id_map: HashMap<String, String>,
}

impl Job<'_> {
    fn write_export(
        &self,
        templates: &[(Template, String)],
        operation: &str,
    ) -> Result<(), Box<dyn Error>> {
        let paths = export_templates(
            templates,
            self.out_dir,
            self.work_dir,
            self.source_stack,
            operation,
        )?;
        print_export_result(&paths);
        Ok(())
    }

    /// Renames resources with the Stack Refactoring API, or exports the renamed template
    async fn run_same_stack(self, template: Template) -> Result<(), Box<dyn Error>> {
        if self.export {
            let renamed = Template::new(
                rename_resources(template.content, &self.id_map),
                template.format,
            );
            return self.write_export(
                &[(renamed, "refactored".to_string())],
                "refactor-same-stack",
            );
        }

        refactor_stack_resources(
            self.client,
            self.source_stack,
            template,
            self.id_map,
            self.work_dir,
        )
        .await
    }

    /// Moves resources with the Stack Refactoring API (safer, atomic, but fewer
    /// supported resource types), or exports the resulting templates
    async fn run_cross_stack_refactor(
        self,
        source: Template,
        target: Template,
    ) -> Result<(), Box<dyn Error>> {
        if self.export {
            let (source_final, target_final) =
                cross_stack_templates(&source, &target, &self.id_map);
            return self.write_export(
                &[
                    (source_final, format!("source-{}", self.source_stack)),
                    (target_final, format!("target-{}", self.target_stack)),
                ],
                "refactor-cross-stack",
            );
        }

        refactor_stack_resources_cross_stack(
            self.client,
            StackTemplate {
                name: self.source_stack,
                template: source,
            },
            StackTemplate {
                name: self.target_stack,
                template: target,
            },
            self.id_map,
            self.work_dir,
        )
        .await
    }

    /// Moves resources by removing them from the source and importing them into the target,
    /// or exports the four templates involved
    async fn run_import(
        self,
        selected: Vec<&StackResourceSummary>,
        source: Template,
        target: Template,
    ) -> Result<(), Box<dyn Error>> {
        let import = ImportJob {
            client: self.client,
            source_stack: self.source_stack,
            target_stack: self.target_stack,
            id_mapping: &self.id_map,
            work_dir: self.work_dir,
        };
        let templates = prepare_import(&import, &selected, &source, &target).await?;

        if self.export {
            return self.write_export(
                &templates.for_export(self.source_stack, self.target_stack),
                "import",
            );
        }

        apply_import(&import, selected, &source, templates).await
    }
}
