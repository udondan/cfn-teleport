use crate::cfn_yaml;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// CloudFormation template format
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TemplateFormat {
    Json,
    Yaml,
}

impl TemplateFormat {
    /// File extension for templates in this format
    pub(crate) fn extension(self) -> &'static str {
        match self {
            TemplateFormat::Json => "json",
            TemplateFormat::Yaml => "yaml",
        }
    }
}

/// Wrapper for CloudFormation templates that preserves their original format
#[derive(Clone, Debug)]
pub(crate) struct Template {
    pub(crate) content: serde_json::Value,
    pub(crate) format: TemplateFormat,
}

impl Template {
    pub(crate) fn new(content: serde_json::Value, format: TemplateFormat) -> Self {
        Self { content, format }
    }

    /// Serialize the template to string in its original format.
    /// YAML templates are serialized as YAML, JSON templates as JSON.
    pub(crate) fn to_string(&self) -> Result<String, Box<dyn Error>> {
        match self.format {
            TemplateFormat::Json => Ok(serde_json::to_string(&self.content)?),
            TemplateFormat::Yaml => Ok(cfn_yaml::emitter::to_yaml_string(&self.content)?),
        }
    }
}

/// YAML short-form intrinsic function tags, which are converted to long form
const CFN_SHORT_TAGS: &[&str] = &[
    "!Ref",
    "!Sub",
    "!GetAtt",
    "!Join",
    "!Select",
    "!Split",
    "!FindInMap",
    "!Base64",
    "!GetAZs",
    "!ImportValue",
    "!If",
    "!Not",
    "!And",
    "!Or",
    "!Equals",
];

/// Parses a template as JSON, or as YAML with CloudFormation tag support.
/// `origin` describes the template in the error message, e.g. "template file x.yaml".
pub(crate) fn parse_template_str(contents: &str, origin: &str) -> Result<Template, Box<dyn Error>> {
    // Try JSON first (faster and more common in automated deployments)
    if let Ok(parsed) = serde_json::from_str(contents) {
        return Ok(Template::new(parsed, TemplateFormat::Json));
    }

    let parsed = cfn_yaml::parse_yaml_to_json(contents).map_err(|yaml_err| {
        format!(
            "Failed to parse {} as JSON or YAML. YAML error: {}",
            origin, yaml_err
        )
    })?;

    if CFN_SHORT_TAGS.iter().any(|tag| contents.contains(tag)) {
        eprintln!("\n⚠️  Warning: Template contains CloudFormation intrinsic function tags (!Ref, !Sub, etc.)");
        eprintln!("   These will be converted to long-form when the template is updated:");
        eprintln!("   - '!Ref MyParam' becomes 'Ref: MyParam'");
        eprintln!("   - '!Sub ${{...}}' becomes 'Fn::Sub: ${{...}}'");
        eprintln!("   Both forms are functionally equivalent and accepted by CloudFormation.\n");
    }

    Ok(Template::new(parsed, TemplateFormat::Yaml))
}

/// Writes a template to disk in its original format (JSON or YAML)
pub(crate) fn write_template_to_file(
    template: &Template,
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let template_str = template
        .to_string()
        .map_err(|e| format!("Failed to serialize template: {}", e))?;

    let mut file = fs::File::create(path)
        .map_err(|e| format!("Failed to create file {}: {}", path.display(), e))?;

    file.write_all(template_str.as_bytes())
        .map_err(|e| format!("Failed to write template to {}: {}", path.display(), e))?;

    Ok(())
}

/// Reads a template from a file, detecting format automatically (JSON or YAML)
pub(crate) fn read_template_from_file(path: &Path) -> Result<Template, Box<dyn Error>> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    parse_template_str(&contents, &format!("template file {}", path.display()))
}
