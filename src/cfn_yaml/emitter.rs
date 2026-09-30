//! Serializes JSON values as block-style YAML with the libyaml emitter.
//!
//! CloudFormation reads templates as YAML 1.1, where plain scalars like `yes`,
//! `on` or `n` are booleans. YAML libraries following YAML 1.2 write these
//! strings unquoted, which turns them into booleans. Strings that could be read
//! as anything else are single-quoted. libyaml quotes strings that aren't valid
//! plain scalars, for example with indicators like `!`, `*` or `: `.

use serde_json::Value;
use std::mem::MaybeUninit;
use std::ptr;
use unsafe_libyaml as sys;

/// Serializes a JSON value as YAML.
pub fn to_yaml_string(value: &Value) -> Result<String, String> {
    let mut emitter = Emitter::new();
    emitter.emit(|event| unsafe {
        sys::yaml_stream_start_event_initialize(event, sys::YAML_UTF8_ENCODING).ok
    })?;
    emitter.emit(|event| unsafe {
        sys::yaml_document_start_event_initialize(
            event,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            true,
        )
        .ok
    })?;
    emitter.value(value)?;
    emitter.emit(|event| unsafe { sys::yaml_document_end_event_initialize(event, true).ok })?;
    emitter.emit(|event| unsafe { sys::yaml_stream_end_event_initialize(event).ok })?;
    emitter.finish()
}

struct Emitter {
    // Boxed, libyaml keeps pointers into the emitter and to the output.
    sys: Box<MaybeUninit<sys::yaml_emitter_t>>,
    output: Box<Vec<u8>>,
}

unsafe fn write_handler(data: *mut std::ffi::c_void, buffer: *mut u8, size: u64) -> i32 {
    let output = &mut *(data as *mut Vec<u8>);
    output.extend_from_slice(std::slice::from_raw_parts(buffer, size as usize));
    1
}

impl Emitter {
    fn new() -> Emitter {
        let mut emitter = Emitter {
            sys: Box::new(MaybeUninit::uninit()),
            output: Box::default(),
        };
        unsafe {
            let sys = emitter.sys.as_mut_ptr();
            if sys::yaml_emitter_initialize(sys).fail {
                panic!("malloc error: initializing the YAML emitter failed");
            }
            sys::yaml_emitter_set_unicode(sys, true);
            // Don't wrap long lines.
            sys::yaml_emitter_set_width(sys, -1);
            let output: *mut Vec<u8> = &mut *emitter.output;
            sys::yaml_emitter_set_output(sys, write_handler, output.cast());
        }
        emitter
    }

    /// Initializes an event with init and emits it.
    fn emit(&mut self, init: impl FnOnce(*mut sys::yaml_event_t) -> bool) -> Result<(), String> {
        let mut event = MaybeUninit::<sys::yaml_event_t>::uninit();
        if !init(event.as_mut_ptr()) {
            return Err("Creating a YAML event failed".to_string());
        }
        // yaml_emitter_emit takes ownership of the event, also on failure.
        if unsafe { sys::yaml_emitter_emit(self.sys.as_mut_ptr(), event.as_mut_ptr()) }.fail {
            return Err(self.error());
        }
        Ok(())
    }

    fn error(&self) -> String {
        let sys = unsafe { self.sys.assume_init_ref() };
        if sys.problem.is_null() {
            return "Writing YAML failed".to_string();
        }
        let problem = unsafe { std::ffi::CStr::from_ptr(sys.problem.cast()) };
        format!("Writing YAML failed: {}", problem.to_string_lossy())
    }

    fn value(&mut self, value: &Value) -> Result<(), String> {
        match value {
            Value::Null => self.scalar("null", sys::YAML_PLAIN_SCALAR_STYLE),
            Value::Bool(b) => self.scalar(&b.to_string(), sys::YAML_PLAIN_SCALAR_STYLE),
            Value::Number(n) => self.scalar(&n.to_string(), sys::YAML_PLAIN_SCALAR_STYLE),
            Value::String(s) => self.scalar(s, string_style(s)),
            Value::Array(seq) => {
                self.emit(|event| unsafe {
                    sys::yaml_sequence_start_event_initialize(
                        event,
                        ptr::null(),
                        ptr::null(),
                        true,
                        sys::YAML_BLOCK_SEQUENCE_STYLE,
                    )
                    .ok
                })?;
                for item in seq {
                    self.value(item)?;
                }
                self.emit(|event| unsafe { sys::yaml_sequence_end_event_initialize(event).ok })
            }
            Value::Object(map) => {
                self.emit(|event| unsafe {
                    sys::yaml_mapping_start_event_initialize(
                        event,
                        ptr::null(),
                        ptr::null(),
                        true,
                        sys::YAML_BLOCK_MAPPING_STYLE,
                    )
                    .ok
                })?;
                for (key, value) in map {
                    self.scalar(key, string_style(key))?;
                    self.value(value)?;
                }
                self.emit(|event| unsafe { sys::yaml_mapping_end_event_initialize(event).ok })
            }
        }
    }

    fn scalar(&mut self, value: &str, style: sys::yaml_scalar_style_t) -> Result<(), String> {
        let length = i32::try_from(value.len()).map_err(|_| "String too long for YAML")?;
        self.emit(|event| unsafe {
            sys::yaml_scalar_event_initialize(
                event,
                ptr::null(),
                ptr::null(),
                value.as_ptr(),
                length,
                true,
                true,
                style,
            )
            .ok
        })
    }

    fn finish(mut self) -> Result<String, String> {
        if unsafe { sys::yaml_emitter_flush(self.sys.as_mut_ptr()) }.fail {
            return Err(self.error());
        }
        let output = std::mem::take(&mut *self.output);
        String::from_utf8(output).map_err(|e| format!("Writing YAML failed: {e}"))
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        unsafe { sys::yaml_emitter_delete(self.sys.as_mut_ptr()) }
    }
}

/// Quotes strings a YAML 1.1 reader would read as another type. Multi-line
/// strings are written as literal blocks. libyaml falls back to quoting if the
/// requested style can't represent the string.
fn string_style(s: &str) -> sys::yaml_scalar_style_t {
    if is_ambiguous(s) {
        sys::YAML_SINGLE_QUOTED_SCALAR_STYLE
    } else if s.contains('\n') {
        sys::YAML_LITERAL_SCALAR_STYLE
    } else {
        sys::YAML_ANY_SCALAR_STYLE
    }
}

/// YAML 1.1 booleans and nulls. Compared case-insensitively, which also covers
/// spellings like "yEs" that are strings but quoted to be safe.
const RESERVED: &[&str] = &[
    "", "~", "y", "n", "yes", "no", "true", "false", "on", "off", "null",
];

fn is_ambiguous(s: &str) -> bool {
    if RESERVED.iter().any(|r| s.eq_ignore_ascii_case(r)) || s.parse::<f64>().is_ok() {
        return true;
    }
    // Numbers, dates and times in all YAML 1.1 forms, like 0x1F, 1_000, 1:30,
    // 2010-09-09, -.5 or .inf.
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => true,
        Some('+' | '-' | '.') => {
            matches!(chars.next(), Some(c) if c.is_ascii_digit() || c == '.')
                || matches!(
                    s.to_ascii_lowercase().as_str(),
                    ".inf" | "+.inf" | "-.inf" | ".nan"
                )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfn_yaml::parse_yaml_to_json;
    use serde_json::json;

    fn roundtrip(value: &Value) -> Value {
        let yaml = to_yaml_string(value).unwrap();
        parse_yaml_to_json(&yaml).unwrap_or_else(|e| panic!("{e}:\n{yaml}"))
    }

    #[test]
    fn quotes_yaml_1_1_booleans_and_nulls() {
        for s in [
            "yes", "No", "on", "OFF", "y", "N", "true", "False", "null", "~",
        ] {
            let value = json!({ "Value": s });
            assert_eq!(roundtrip(&value), value, "{s}");
            assert!(
                !to_yaml_string(&value).unwrap().contains(&format!(" {s}\n")),
                "{s}"
            );
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
        let expected = r#"AWSTemplateFormatVersion: '2010-09-09'
Resources:
  Bucket:
    Properties:
      BucketName:
        Fn::Sub: ${AWS::StackName}-bucket
      Tags:
      - Key: Enabled
        Value: 'yes'
    Type: AWS::S3::Bucket
"#;
        assert_eq!(to_yaml_string(&value).unwrap(), expected);
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
        // Absolute path: other tests change the working directory of the
        // process while this one runs.
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/test/cloudformation");
        for entry in std::fs::read_dir(dir).unwrap() {
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
