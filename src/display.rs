//! Resource lists as shown in prompts and summaries

use crate::dependencies::{
    ResourceDependencyInfo, EMOJI_BIDIRECTIONAL, EMOJI_BOTH_STACK_INTERFACE, EMOJI_INCOMING,
    EMOJI_OUTGOING, EMOJI_OUTPUTS, EMOJI_PARAMETERS,
};
use aws_sdk_cloudformation::types::StackResourceSummary;
use std::collections::HashMap;
use std::io;

/// One line of the resource list
struct Row<'a> {
    resource_type: &'a str,
    logical_id: &'a str,
    new_logical_id: &'a str,
    physical_id: &'a str,
    info: Option<&'a ResourceDependencyInfo>,
}

impl Row<'_> {
    fn is_renamed(&self) -> bool {
        self.logical_id != self.new_logical_id
    }
}

/// Column widths and marker space shared by all rows
struct Layout {
    type_width: usize,
    id_width: usize,
    /// Width of the longest new ID, if any resource is renamed
    new_id_width: Option<usize>,
    max_emoji_count: usize,
}

/// Formats resources as aligned columns: type, dependency markers, logical ID,
/// new logical ID (if any resource is renamed) and physical ID
pub(crate) async fn format_resources(
    resources: &[&StackResourceSummary],
    resource_id_map: Option<HashMap<String, String>>,
    dependency_info: Option<&HashMap<String, ResourceDependencyInfo>>,
) -> Result<Vec<String>, io::Error> {
    let rows: Vec<Row> = resources
        .iter()
        .map(|resource| {
            let logical_id = resource.logical_resource_id().unwrap_or_default();
            Row {
                resource_type: resource.resource_type().unwrap_or_default(),
                logical_id,
                new_logical_id: resource_id_map
                    .as_ref()
                    .and_then(|map| map.get(logical_id))
                    .map_or(logical_id, String::as_str),
                physical_id: resource.physical_resource_id().unwrap_or_default(),
                info: dependency_info.and_then(|info| info.get(logical_id)),
            }
        })
        .collect();

    let layout = Layout {
        type_width: rows
            .iter()
            .map(|r| r.resource_type.len())
            .max()
            .unwrap_or(0),
        id_width: rows.iter().map(|r| r.logical_id.len()).max().unwrap_or(0),
        new_id_width: rows
            .iter()
            .filter(|r| r.is_renamed())
            .map(|r| r.new_logical_id.len())
            .max(),
        max_emoji_count: rows
            .iter()
            .filter_map(|r| r.info)
            .map(emoji_count)
            .max()
            .unwrap_or(0),
    };

    Ok(rows.iter().map(|row| format_row(row, &layout)).collect())
}

fn format_row(row: &Row, layout: &Layout) -> String {
    let separator = if layout.max_emoji_count > 0 {
        format!("  {}  ", marker(row.info, layout.max_emoji_count))
    } else {
        " ".to_string()
    };

    let rename_column = match layout.new_id_width {
        Some(width) => {
            let indicator = if row.is_renamed() {
                format!(" ► {}", row.new_logical_id)
            } else {
                String::new()
            };
            format!("{:<width$}", indicator, width = width + 4)
        }
        None => String::new(),
    };

    format!(
        "{:<type_width$}{}{:<id_width$}{}  {}",
        row.resource_type,
        separator,
        row.logical_id,
        rename_column,
        row.physical_id,
        type_width = layout.type_width,
        id_width = layout.id_width,
    )
}

/// Number of marker emojis a resource gets (0-2)
fn emoji_count(info: &ResourceDependencyInfo) -> usize {
    usize::from(info.has_resource_deps()) + usize::from(info.has_stack_interface())
}

/// Left/right arrow for dependencies between resources
fn resource_marker(info: &ResourceDependencyInfo) -> Option<&'static str> {
    match (info.has_incoming_deps, info.has_outgoing_deps) {
        (true, true) => Some(EMOJI_BIDIRECTIONAL),
        (true, false) => Some(EMOJI_INCOMING),
        (false, true) => Some(EMOJI_OUTGOING),
        (false, false) => None,
    }
}

/// Up/down arrow for outputs and parameters
fn stack_interface_marker(info: &ResourceDependencyInfo) -> Option<&'static str> {
    match (info.referenced_by_outputs, info.depends_on_parameters) {
        (true, true) => Some(EMOJI_BOTH_STACK_INTERFACE),
        (true, false) => Some(EMOJI_OUTPUTS),
        (false, true) => Some(EMOJI_PARAMETERS),
        (false, false) => None,
    }
}

/// Marker column for one resource. Single markers are padded when some resource has two.
fn marker(info: Option<&ResourceDependencyInfo>, max_emoji_count: usize) -> String {
    let padding = " ".repeat(max_emoji_count + 1);
    let Some(info) = info else {
        return padding;
    };

    let pad_single = max_emoji_count == 2;
    match (resource_marker(info), stack_interface_marker(info)) {
        (Some(left_right), Some(up_down)) => format!("{} {}", left_right, up_down),
        (Some(left_right), None) if pad_single => format!("{}  ", left_right),
        (Some(left_right), None) => left_right.to_string(),
        (None, Some(EMOJI_BOTH_STACK_INTERFACE)) => EMOJI_BOTH_STACK_INTERFACE.to_string(),
        (None, Some(up_down)) if pad_single => format!("  {}", up_down),
        (None, Some(up_down)) => up_down.to_string(),
        (None, None) => padding,
    }
}
