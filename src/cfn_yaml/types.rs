// Copyright 2020-2022 Amazon Web Services, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//
// This module contains types adapted from AWS CloudFormation Guard
// https://github.com/aws-cloudformation/cloudformation-guard
// Licensed under Apache-2.0

use std::fmt;

/// Location information for values in the parsed YAML
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct Location {
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl Location {
    pub(crate) fn new(line: usize, col: usize) -> Self {
        Location { line, col }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.col)
    }
}

/// Range type for values
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RangeType<T> {
    Range(T, T),
    OpenRange(T, T),
}

/// Internal value representation with location tracking
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MarkedValue {
    Null(Location),
    BadValue(String, Location),
    String(String, Location),
    Regex(String, Location),
    Bool(bool, Location),
    Int(i64, Location),
    Float(f64, Location),
    Char(char, Location),
    List(Vec<MarkedValue>, Location),
    Map(
        indexmap::IndexMap<(String, Location), MarkedValue>,
        Location,
    ),
    RangeInt(RangeType<i64>, Location),
    RangeFloat(RangeType<f64>, Location),
    RangeChar(RangeType<char>, Location),
}

impl MarkedValue {
    pub(crate) fn location(&self) -> &Location {
        match self {
            Self::Null(loc)
            | Self::BadValue(_, loc)
            | Self::String(_, loc)
            | Self::Regex(_, loc)
            | Self::Bool(_, loc)
            | Self::Int(_, loc)
            | Self::Float(_, loc)
            | Self::Char(_, loc)
            | Self::List(_, loc)
            | Self::Map(_, loc)
            | Self::RangeInt(_, loc)
            | Self::RangeFloat(_, loc)
            | Self::RangeChar(_, loc) => loc,
        }
    }

    /// Convert MarkedValue to serde_json::Value, discarding location information
    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        match self {
            MarkedValue::Null(_) => serde_json::Value::Null,
            MarkedValue::BadValue(s, _) => serde_json::Value::String(s.clone()),
            MarkedValue::String(s, _) => serde_json::Value::String(s.clone()),
            MarkedValue::Regex(s, _) => serde_json::Value::String(s.clone()),
            MarkedValue::Bool(b, _) => serde_json::Value::Bool(*b),
            MarkedValue::Int(i, _) => serde_json::Value::Number((*i).into()),
            MarkedValue::Float(f, _) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            MarkedValue::Char(c, _) => serde_json::Value::String(c.to_string()),
            MarkedValue::List(list, _) => {
                serde_json::Value::Array(list.iter().map(|v| v.to_json_value()).collect())
            }
            MarkedValue::Map(map, _) => {
                let mut obj = serde_json::Map::new();
                for ((key, _loc), value) in map.iter() {
                    obj.insert(key.clone(), value.to_json_value());
                }
                serde_json::Value::Object(obj)
            }
            MarkedValue::RangeInt(range, _) => match range {
                RangeType::Range(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start,
                    "end": end,
                    "inclusive": true
                }),
                RangeType::OpenRange(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start,
                    "end": end,
                    "inclusive": false
                }),
            },
            MarkedValue::RangeFloat(range, _) => match range {
                RangeType::Range(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start,
                    "end": end,
                    "inclusive": true
                }),
                RangeType::OpenRange(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start,
                    "end": end,
                    "inclusive": false
                }),
            },
            MarkedValue::RangeChar(range, _) => match range {
                RangeType::Range(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start.to_string(),
                    "end": end.to_string(),
                    "inclusive": true
                }),
                RangeType::OpenRange(start, end) => serde_json::json!({
                    "type": "range",
                    "start": start.to_string(),
                    "end": end.to_string(),
                    "inclusive": false
                }),
            },
        }
    }
}
