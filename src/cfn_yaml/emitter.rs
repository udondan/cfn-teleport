//! Serializes JSON values as block-style YAML.
//!
//! CloudFormation reads templates as YAML 1.1, where plain scalars like `yes`,
//! `on` or `n` are booleans. YAML libraries following YAML 1.2 write these
//! strings unquoted, which turns them into booleans. Strings are only written
//! plain if they can't be read as anything else, all others are double-quoted.

use serde_json::Value;

/// Serializes a JSON value as YAML.
pub fn to_yaml_string(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(map) if !map.is_empty() => write_mapping(&mut out, map, 0),
        Value::Array(seq) if !seq.is_empty() => write_sequence(&mut out, seq, 0),
        _ => {
            out.push_str(&scalar(value));
            out.push('\n');
        }
    }
    out
}

fn write_mapping(out: &mut String, map: &serde_json::Map<String, Value>, indent: usize) {
    for (i, (key, value)) in map.iter().enumerate() {
        // The first entry of a mapping in a sequence item follows the "- ".
        if i > 0 || out.is_empty() || out.ends_with('\n') {
            push_indent(out, indent);
        }
        out.push_str(&string(key));
        out.push(':');
        write_value(out, value, indent);
    }
}

fn write_sequence(out: &mut String, seq: &[Value], indent: usize) {
    for (i, item) in seq.iter().enumerate() {
        if i > 0 || out.is_empty() || out.ends_with('\n') {
            push_indent(out, indent);
        }
        out.push('-');
        match item {
            Value::Object(map) if !map.is_empty() => {
                out.push(' ');
                write_mapping(out, map, indent + 2);
            }
            Value::Array(seq) if !seq.is_empty() => {
                out.push(' ');
                write_sequence(out, seq, indent + 2);
            }
            _ => {
                out.push(' ');
                out.push_str(&scalar(item));
                out.push('\n');
            }
        }
    }
}

/// Writes a mapping value after its "key:".
fn write_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            out.push('\n');
            write_mapping(out, map, indent + 2);
        }
        Value::Array(seq) if !seq.is_empty() => {
            out.push('\n');
            write_sequence(out, seq, indent + 2);
        }
        _ => {
            out.push(' ');
            out.push_str(&scalar(value));
            out.push('\n');
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.push_str(&" ".repeat(indent));
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => string(s),
        Value::Array(_) => "[]".to_string(),
        Value::Object(_) => "{}".to_string(),
    }
}

/// Writes a string plain if it can only be read as that string, double-quoted
/// otherwise. JSON string escapes are valid in double-quoted YAML scalars.
fn string(s: &str) -> String {
    if is_plain_safe(s) {
        s.to_string()
    } else {
        serde_json::to_string(s).expect("serializing a string can't fail")
    }
}

/// YAML 1.1 booleans and nulls. Compared case-insensitively, which also covers
/// spellings like "yEs" that are strings but quoted to be safe.
const RESERVED: &[&str] = &["y", "n", "yes", "no", "true", "false", "on", "off", "null"];

fn is_plain_safe(s: &str) -> bool {
    let bytes = s.as_bytes();
    // A leading letter or underscore rules out numbers, timestamps and all
    // indicator characters.
    match bytes.first() {
        Some(c) if c.is_ascii_alphabetic() || *c == b'_' => {}
        _ => return false,
    }
    if RESERVED.iter().any(|r| s.eq_ignore_ascii_case(r)) {
        return false;
    }
    if bytes.last() == Some(&b' ') || bytes.last() == Some(&b':') {
        return false;
    }
    bytes.iter().enumerate().all(|(i, c)| match c {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/' => true,
        // ": " starts a mapping value.
        b':' => bytes.get(i + 1) != Some(&b' '),
        // " #" starts a comment.
        b' ' => bytes.get(i + 1) != Some(&b'#') && bytes.get(i + 1) != Some(&b' '),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfn_yaml::parse_yaml_to_json;
    use serde_json::json;

    fn roundtrip(value: &Value) -> Value {
        let yaml = to_yaml_string(value);
        parse_yaml_to_json(&yaml).unwrap_or_else(|e| panic!("{e}:\n{yaml}"))
    }

    #[test]
    fn quotes_yaml_1_1_booleans_and_nulls() {
        for s in [
            "yes", "No", "on", "OFF", "y", "N", "true", "False", "null", "~",
        ] {
            let value = json!({ "Value": s });
            assert_eq!(roundtrip(&value), value, "{s}");
            assert!(!to_yaml_string(&value).contains(&format!(" {s}\n")), "{s}");
        }
    }

    #[test]
    fn roundtrips_tricky_values() {
        let value: Value = serde_json::from_str(
            r##"{
            "s_num": "123", "s_oct": "0755", "s_float": "1.5", "s_exp": "1e3", "s_hex": "0x1F",
            "s_inf": ".inf", "s_date": "2010-09-09", "s_colon": "a: b", "s_colon_end": "a:",
            "s_hash": "a #b", "s_lead": " x", "s_trail": "x ", "s_double_space": "a  b",
            "s_star": "*x", "s_amp": "&x", "s_bang": "!Ref x", "s_percent": "%x", "s_at": "@x",
            "s_multi": "line1\nline2\n", "s_empty": "", "s_quote": "it's \"q\"", "s_backslash": "a\\b",
            "s_tab": "a\tb", "s_unicode": "äöü ✓", "s_dash": "- x", "s_brace": "{x}", "s_pipe": "| x",
            "s_arn": "arn:aws:s3:::bucket/*", "s_sub": "${AWS::StackName}-x",
            "n_int": 42, "n_neg": -1, "n_float": 1.5,
            "b": true, "nul": null, "arr": [], "obj": {},
            "nested": [{"a": [1, "2", [], {}]}, [["x"], {"b": {"c": "yes"}}], "y"],
            "Fn::Sub": "${AWS::StackName}-x", "key with space": 1, "123": "numkey", "on": "key"
        }"##,
        )
        .unwrap();
        assert_eq!(roundtrip(&value), value);
    }

    #[test]
    fn writes_block_style() {
        let value = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Resources": {
                "Bucket": {
                    "Type": "AWS::S3::Bucket",
                    "Properties": {
                        "BucketName": { "Fn::Sub": "${AWS::StackName}-bucket" },
                        "Tags": [{ "Key": "Enabled", "Value": "yes" }]
                    }
                }
            }
        });
        let expected = r#"AWSTemplateFormatVersion: "2010-09-09"
Resources:
  Bucket:
    Properties:
      BucketName:
        Fn::Sub: "${AWS::StackName}-bucket"
      Tags:
        - Key: Enabled
          Value: "yes"
    Type: AWS::S3::Bucket
"#;
        assert_eq!(to_yaml_string(&value), expected);
    }

    #[test]
    fn writes_top_level_scalars_and_sequences() {
        for value in [
            json!("yes"),
            json!(1),
            json!(null),
            json!([]),
            json!({}),
            json!([1, [2, {"a": 3}]]),
        ] {
            assert_eq!(roundtrip(&value), value);
        }
    }

    #[test]
    fn roundtrips_test_templates() {
        for entry in std::fs::read_dir("test/cloudformation").unwrap() {
            let path = entry.unwrap().path();
            let contents = std::fs::read_to_string(&path).unwrap();
            let value: Value = match path.extension().and_then(|e| e.to_str()) {
                Some("json") => serde_json::from_str(&contents).unwrap(),
                Some("yaml") | Some("yml") => parse_yaml_to_json(&contents).unwrap(),
                _ => continue,
            };
            assert_eq!(roundtrip(&value), value, "{}", path.display());
        }
    }
}
