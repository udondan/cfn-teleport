// Copyright 2020-2022 Amazon Web Services, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// This module contains code adapted from AWS CloudFormation Guard
// https://github.com/aws-cloudformation/cloudformation-guard
// See readme.md for attribution details

#![allow(clippy::all)]
#![allow(dead_code)]

mod cstr;
mod errors;
mod event;
pub(crate) mod loader;
mod mappings;
mod parser;
mod tag;
pub(crate) mod types;
mod util;

use errors::Result;
use std::error::Error as StdError;

/// Parse CloudFormation YAML template with intrinsic function tag support
pub(crate) fn parse_cf_yaml(yaml_str: &str) -> Result<types::MarkedValue> {
    let mut loader = loader::Loader::new();
    loader.load(yaml_str.to_string())
}

/// Parse CF YAML and convert to serde_json::Value
pub fn parse_yaml_to_json(
    yaml_str: &str,
) -> std::result::Result<serde_json::Value, Box<dyn StdError>> {
    let marked_value = parse_cf_yaml(yaml_str)
        .map_err(|e| format!("Failed to parse CloudFormation YAML: {}", e))?;
    Ok(marked_value.to_json_value())
}
