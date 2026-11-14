use aws_config::retry::RetryConfig;
use aws_config::BehaviorVersion;
use aws_sdk_cloudformation as cloudformation;
use clap::Parser;
use std::error::Error;
use std::process;

mod cfn_yaml;
mod cli;
mod dependencies;
mod display;
mod export;
mod files;
mod import;
mod migration_spec;
mod move_checks;
mod prompt;
mod refactor;
mod reference_updater;
mod selection;
mod spinner;
mod stack_update;
mod stacks;
mod supported_resource_types;
mod template;
mod template_edit;
mod workflow;

use cli::{validate_args, Args};
use prompt::DialoguerPrompter;
use workflow::run_with;

#[cfg(test)]
mod aws_tests;
#[cfg(test)]
#[path = "../tests/support/cfn_xml.rs"]
mod cfn_xml;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
#[path = "../tests/support/unit_support.rs"]
mod test_support;
#[cfg(test)]
mod unit_tests;

// The test modules use `super::*` to reach the helpers of all modules
#[cfg(test)]
use {
    dependencies::*, display::*, export::*, files::*, migration_spec::*, move_checks::*, prompt::*,
    refactor::*, selection::*, stack_update::*, stacks::*, std::collections::HashMap, std::fs,
    std::path::Path, template::*, template_edit::*,
};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // Print error with proper formatting (interprets escape sequences)
        eprintln!("\n{}\n", err);
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    validate_args(&args)?;

    // Configure retry with 3 retries (4 total attempts), exponential backoff
    let retry_config = RetryConfig::standard()
        .with_max_attempts(4) // 1 initial + 3 retries
        .with_initial_backoff(std::time::Duration::from_secs(1))
        .with_max_backoff(std::time::Duration::from_secs(4));

    let config = aws_config::defaults(BehaviorVersion::v2026_11_10())
        .retry_config(retry_config)
        .load()
        .await;
    let client = cloudformation::Client::new(&config);

    run_with(args, &client, &DialoguerPrompter, &std::env::current_dir()?).await
}
