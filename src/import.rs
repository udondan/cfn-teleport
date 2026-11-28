//! Moving resources by removing them from one stack and importing them into another

use crate::export::{save_debug_artifacts, ErrorContext};
use crate::move_checks::check_import_parameter_dependencies;
use crate::reference_updater;
use crate::spinner;
use crate::stack_update::{
    create_changeset, execute_changeset, update_stack, wait_for_changeset_created,
    wait_for_stack_update_completion,
};
use crate::stacks::validate_template;
use crate::template::Template;
use crate::template_edit::{add_resources, remove_resources, retain_resources};
use aws_sdk_cloudformation as cloudformation;
use aws_sdk_cloudformation::types::StackResourceSummary;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// All templates of an import: the source stack is first updated to retain the
/// resources, then they are removed, and the target imports them with a
/// deletion policy before getting its final template.
pub(crate) struct ImportTemplates {
    pub(crate) retained: Template,
    pub(crate) removed: Template,
    pub(crate) target_with_policy: Template,
    pub(crate) target_final: Template,
}

impl ImportTemplates {
    pub(crate) fn build(
        source: &Template,
        target: &Template,
        id_mapping: &HashMap<String, String>,
    ) -> Self {
        let resource_ids: Vec<String> = id_mapping.keys().cloned().collect();
        let (with_policy, target_final) = add_resources(
            target.content.clone(),
            source.content.clone(),
            id_mapping.clone(),
        );
        let update_references =
            |template| reference_updater::update_template_references(template, id_mapping);

        Self {
            retained: Template::new(
                retain_resources(source.content.clone(), resource_ids.clone()),
                source.format,
            ),
            removed: Template::new(
                remove_resources(source.content.clone(), resource_ids),
                source.format,
            ),
            target_with_policy: Template::new(update_references(with_policy), target.format),
            target_final: Template::new(update_references(target_final), target.format),
        }
    }

    /// Templates with the suffixes used for exported files
    pub(crate) fn for_export(
        &self,
        source_stack: &str,
        target_stack: &str,
    ) -> Vec<(Template, String)> {
        vec![
            (
                self.retained.clone(),
                format!("source-{}-retained", source_stack),
            ),
            (
                self.removed.clone(),
                format!("source-{}-removed", source_stack),
            ),
            (
                self.target_with_policy.clone(),
                format!("target-{}-with-deletion-policy", target_stack),
            ),
            (
                self.target_final.clone(),
                format!("target-{}-final", target_stack),
            ),
        ]
    }

    fn for_debugging(&self) -> Vec<(Template, String)> {
        vec![
            (self.retained.clone(), "retained".to_string()),
            (self.removed.clone(), "removed".to_string()),
            (
                self.target_with_policy.clone(),
                "target-with-policy".to_string(),
            ),
            (self.target_final.clone(), "target-final".to_string()),
        ]
    }
}

/// What to import from where
pub(crate) struct ImportJob<'a> {
    pub(crate) client: &'a cloudformation::Client,
    pub(crate) source_stack: &'a str,
    pub(crate) target_stack: &'a str,
    pub(crate) id_mapping: &'a HashMap<String, String>,
    pub(crate) work_dir: &'a Path,
}

/// Checks that the resources can be imported and builds the templates.
/// Invalid templates are saved for debugging.
pub(crate) async fn prepare_import(
    job: &ImportJob<'_>,
    selected: &[&StackResourceSummary],
    source: &Template,
    target: &Template,
) -> Result<ImportTemplates, Box<dyn Error>> {
    check_import_parameter_dependencies(selected, job.id_mapping, &source.content)?;

    let templates = ImportTemplates::build(source, target, job.id_mapping);

    for template in [
        &templates.retained,
        &templates.removed,
        &templates.target_final,
        &templates.target_with_policy,
    ] {
        if let Err(e) = validate_template(job.client, template.content.clone()).await {
            let error = e.to_string();
            save_debug_artifacts(
                job.work_dir,
                "import",
                &templates.for_debugging(),
                &ErrorContext {
                    error: &error,
                    source_stack: job.source_stack,
                    target_stack: Some(job.target_stack),
                    operation: "import",
                    resources: job.id_mapping,
                },
            );
            return Err(format!(
                "Unable to proceed, because the template is invalid: {}\n\
                 Templates and error context have been saved for debugging.",
                error
            )
            .into());
        }
    }

    Ok(templates)
}

/// Removes the resources from the source stack and imports them into the target stack
pub(crate) async fn apply_import(
    job: &ImportJob<'_>,
    selected: Vec<&StackResourceSummary>,
    source: &Template,
    templates: ImportTemplates,
) -> Result<(), Box<dyn Error>> {
    let client = job.client;
    let count = job.id_mapping.len();

    let spinner = spinner::Spin::new(&format!(
        "Removing {} resources from stack {}",
        count, job.source_stack
    ));

    if source.to_string()? != templates.retained.to_string()? {
        update_template(client, job.source_stack, templates.retained).await?;
        wait_for_stack_update_completion(client, job.source_stack, None).await?;
    }

    update_template(client, job.source_stack, templates.removed).await?;
    wait_for_stack_update_completion(client, job.source_stack, Some(spinner)).await?;

    let spinner = spinner::Spin::new(&format!(
        "Importing {} resources into stack {}",
        count, job.target_stack,
    ));

    let changeset_name = create_changeset(
        client,
        job.target_stack,
        templates.target_with_policy,
        selected,
        job.id_mapping.clone(),
    )
    .await?;

    wait_for_changeset_created(client, job.target_stack, &changeset_name).await?;
    execute_changeset(client, job.target_stack, &changeset_name).await?;
    wait_for_stack_update_completion(client, job.target_stack, None).await?;

    update_template(client, job.target_stack, templates.target_final).await?;
    wait_for_stack_update_completion(client, job.target_stack, Some(spinner)).await?;

    Ok(())
}

async fn update_template(
    client: &cloudformation::Client,
    stack_name: &str,
    template: Template,
) -> Result<(), Box<dyn Error>> {
    update_stack(client, stack_name, template.content, template.format).await
}
