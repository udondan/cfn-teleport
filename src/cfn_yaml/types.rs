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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn loc() -> Location {
        Location::new(3, 7)
    }

    #[test]
    fn location_new_and_display() {
        let l = loc();
        assert_eq!(l, Location { line: 3, col: 7 });
        assert_eq!(l.to_string(), "line 3, column 7");
    }

    #[test]
    fn location_of_every_variant() {
        let values = [
            MarkedValue::Null(loc()),
            MarkedValue::BadValue("x".to_string(), loc()),
            MarkedValue::String("x".to_string(), loc()),
            MarkedValue::Regex("x".to_string(), loc()),
            MarkedValue::Bool(true, loc()),
            MarkedValue::Int(1, loc()),
            MarkedValue::Float(1.5, loc()),
            MarkedValue::Char('c', loc()),
            MarkedValue::List(vec![], loc()),
            MarkedValue::Map(indexmap::IndexMap::new(), loc()),
            MarkedValue::RangeInt(RangeType::Range(1, 2), loc()),
            MarkedValue::RangeFloat(RangeType::Range(1.0, 2.0), loc()),
            MarkedValue::RangeChar(RangeType::Range('a', 'b'), loc()),
        ];
        assert!(values.iter().all(|v| *v.location() == loc()));
    }

    #[test]
    fn converts_simple_variants_to_json() {
        assert_eq!(
            MarkedValue::BadValue("bad".to_string(), loc()).to_json_value(),
            json!("bad")
        );
        assert_eq!(
            MarkedValue::Regex("a.*".to_string(), loc()).to_json_value(),
            json!("a.*")
        );
        assert_eq!(MarkedValue::Char('c', loc()).to_json_value(), json!("c"));
        assert_eq!(
            MarkedValue::Float(f64::NAN, loc()).to_json_value(),
            json!(null)
        );
    }

    #[test]
    fn converts_ranges_to_json() {
        assert_eq!(
            MarkedValue::RangeInt(RangeType::Range(1, 2), loc()).to_json_value(),
            json!({"type": "range", "start": 1, "end": 2, "inclusive": true})
        );
        assert_eq!(
            MarkedValue::RangeInt(RangeType::OpenRange(1, 2), loc()).to_json_value(),
            json!({"type": "range", "start": 1, "end": 2, "inclusive": false})
        );
        assert_eq!(
            MarkedValue::RangeFloat(RangeType::Range(1.5, 2.5), loc()).to_json_value(),
            json!({"type": "range", "start": 1.5, "end": 2.5, "inclusive": true})
        );
        assert_eq!(
            MarkedValue::RangeFloat(RangeType::OpenRange(1.5, 2.5), loc()).to_json_value(),
            json!({"type": "range", "start": 1.5, "end": 2.5, "inclusive": false})
        );
        assert_eq!(
            MarkedValue::RangeChar(RangeType::Range('a', 'z'), loc()).to_json_value(),
            json!({"type": "range", "start": "a", "end": "z", "inclusive": true})
        );
        assert_eq!(
            MarkedValue::RangeChar(RangeType::OpenRange('a', 'z'), loc()).to_json_value(),
            json!({"type": "range", "start": "a", "end": "z", "inclusive": false})
        );
    }
}
