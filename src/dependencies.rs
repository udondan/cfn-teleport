//! Dependency markers shown next to resources in the selection list

use crate::reference_updater;
use crate::template_edit::{parameter_names, section_keys};
use aws_sdk_cloudformation::types::StackResourceSummary;
use std::collections::HashMap;

// Dependency marker emojis
pub(crate) const EMOJI_INCOMING: &str = "➡️";
pub(crate) const EMOJI_OUTGOING: &str = "⬅️";
pub(crate) const EMOJI_BIDIRECTIONAL: &str = "↔️";
pub(crate) const EMOJI_OUTPUTS: &str = "⬆️";
pub(crate) const EMOJI_PARAMETERS: &str = "⬇️";
pub(crate) const EMOJI_BOTH_STACK_INTERFACE: &str = "↕️";

/// Holds dependency information for a resource
pub(crate) struct ResourceDependencyInfo {
    pub(crate) has_incoming_deps: bool, // Other resources reference this one
    pub(crate) has_outgoing_deps: bool, // This resource references others
    pub(crate) referenced_by_outputs: bool, // Outputs reference this one
    pub(crate) depends_on_parameters: bool, // This resource references stack parameters
}

impl ResourceDependencyInfo {
    /// References between this resource and other resources
    pub(crate) fn has_resource_deps(&self) -> bool {
        self.has_incoming_deps || self.has_outgoing_deps
    }

    /// References from outputs or to parameters
    pub(crate) fn has_stack_interface(&self) -> bool {
        self.referenced_by_outputs || self.depends_on_parameters
    }
}

/// Computes dependency markers for resources based on template analysis.
///
/// Analyzes the template to identify which resources:
/// - Have incoming dependencies (other resources reference them)
/// - Have outgoing dependencies (they reference other resources)
/// - Are referenced by stack outputs
/// - Depend on stack parameters
///
/// # Returns
/// HashMap mapping logical resource IDs to their dependency information
pub(crate) fn compute_dependency_markers(
    resources: &[&StackResourceSummary],
    template: &serde_json::Value,
) -> HashMap<String, ResourceDependencyInfo> {
    let all_references = reference_updater::find_all_references(template);
    let output_references = all_references.get("Outputs").cloned().unwrap_or_default();
    let parameter_names = parameter_names(template);
    let resource_names = section_keys(template, "Resources");

    let mut dependency_map = HashMap::new();

    for resource in resources {
        let logical_id = resource.logical_resource_id().unwrap_or_default();

        // Referenced by any other resource (the Outputs section is handled separately)
        let has_incoming_deps = all_references.iter().any(|(referencing, referenced)| {
            referencing != "Outputs" && referenced.contains(logical_id)
        });

        let resource_references = all_references.get(logical_id).cloned().unwrap_or_default();

        dependency_map.insert(
            logical_id.to_string(),
            ResourceDependencyInfo {
                has_incoming_deps,
                // References to other resources (not parameters or pseudo-parameters)
                has_outgoing_deps: resource_references
                    .iter()
                    .any(|name| resource_names.contains(name)),
                referenced_by_outputs: output_references.contains(logical_id),
                depends_on_parameters: resource_references
                    .iter()
                    .any(|name| parameter_names.contains(name)),
            },
        );
    }

    dependency_map
}

/// Generates a legend for dependency markers based on what markers are present.
///
/// Only returns a legend if there are actually markers to display.
/// Only includes entries for markers that are present in the dependency info.
pub(crate) fn generate_legend(
    dependency_info: &HashMap<String, ResourceDependencyInfo>,
) -> Option<String> {
    let has_resource_deps = dependency_info.values().any(|i| i.has_resource_deps());
    let has_stack_interface = dependency_info.values().any(|i| i.has_stack_interface());

    if !has_resource_deps && !has_stack_interface {
        return None;
    }

    let mut legend = String::from("Legend:");

    if has_resource_deps {
        legend.push_str(&format!(
            "\n  Resource dependencies: {}  incoming  {}  outgoing    {}  bidirectional",
            EMOJI_INCOMING, EMOJI_OUTGOING, EMOJI_BIDIRECTIONAL
        ));
    }

    if has_stack_interface {
        legend.push_str(&format!(
            "\n  Stack interface:       {}  outputs   {}  parameters  {}  both",
            EMOJI_OUTPUTS, EMOJI_PARAMETERS, EMOJI_BOTH_STACK_INTERFACE
        ));
    }

    Some(legend)
}
