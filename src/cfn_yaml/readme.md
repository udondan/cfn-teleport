# CloudFormation YAML Parser

This module contains vendored code for parsing CloudFormation YAML templates with intrinsic function tags (!Ref, !Sub, !GetAtt, etc.).

## Attribution and License Chain

This code was copied from [AWS CloudFormation Guard](https://github.com/aws-cloudformation/cloudformation-guard) (Apache 2.0 license):

- **Source**: `guard/src/rules/libyaml/` directory
- **Commit**: `4d8fb33226b23b708f1d92527c8ccd22efdf26f0` (2026-02-11)
- **License**: Apache 2.0 (compatible with cfn-teleport's Apache 2.0 license)

The CloudFormation Guard code itself contains portions copied from [serde_yaml](https://github.com/dtolnay/serde-yaml) by [David Tolnay](https://github.com/dtolnay), with customizations to support CloudFormation intrinsic function tags.

## Why Vendored?

1. **cfn-guard doesn't expose the libyaml module publicly** - it's marked as `mod libyaml` (private) in their crate
2. **serde_yaml doesn't support CloudFormation tags** - CloudFormation uses special YAML tags like `!Ref`, `!Sub`, `!GetAtt` that standard YAML parsers cannot handle
3. **Small, stable codebase** - YAML parsing logic is mature and rarely changes (~1000 lines)

## Files

### Copied from cfn-guard (import paths modified):

- `cstr.rs` - C string utilities
- `event.rs` - YAML event types
- `loader.rs` - Main YAML loader with CF tag support
- `parser.rs` - YAML parser wrapper using unsafe-libyaml
- `tag.rs` - YAML tag handling
- `util.rs` - Utility functions

### Created for cfn-teleport integration:

- `errors.rs` - Simplified error types (extracted from cfn-guard's `rules/errors.rs`)
- `mappings.rs` - CF tag mappings (extracted from cfn-guard's `rules/mod.rs`)
- `types.rs` - `MarkedValue`, `Location`, `RangeType` (extracted from cfn-guard's `rules/values.rs` and `rules/path_value.rs`)
- `mod.rs` - Public API: `parse_yaml_to_json()` function

### Excluded:

- `loader_tests.rs` - Removed due to extra test dependencies (rstest, pretty_assertions)

## Modifications Made

1. Updated import paths from `crate::rules::libyaml::*` to `crate::cfn_yaml::*`
2. Fixed visibility in `tag.rs` from `pub(in crate::rules::libyaml)` to `pub(crate)`
3. Fixed lifetime issue in `mappings.rs::short_form_to_long()`
4. Added `MarkedValue::to_json_value()` for converting to `serde_json::Value`
5. Created simplified public API for parsing CF YAML templates

## Important Limitation: Tag Format Conversion

**Parsing**: This module successfully parses CloudFormation YAML with intrinsic function tags:
```yaml
BucketName: !Ref MyParameter
QueueName: !Sub '${AWS::StackName}-queue'
```

**Serialization**: When templates are serialized back to YAML (using `serde_yml`), the short-form tags are converted to long-form:
```yaml
BucketName:
  Ref: MyParameter
QueueName:
  Fn::Sub: '${AWS::StackName}-queue'
```

**Why this happens**: 
- This module only handles **parsing** (YAML → JSON)
- Serialization (JSON → YAML) is done by `serde_yml`, which doesn't support CloudFormation tags
- The internal representation stores intrinsic functions as JSON objects: `!Ref X` → `{"Ref": "X"}`
- When serialized, these become nested YAML structures, not tags

**Impact**:
- ✅ **Functionally equivalent**: Both forms are valid CloudFormation YAML and behave identically
- ✅ **CloudFormation accepts both**: The AWS APIs process both formats the same way
- ❌ **Format not preserved**: Original `!Tag` syntax is lost, replaced with long-form
- ⚠️ **Users are warned**: cfn-teleport displays a warning when CF tags are detected in input

**Example transformation**:
```yaml
# Input template
Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: !Ref BucketParameter

# Output template (after processing)
Resources:
  MyBucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName:
        Ref: BucketParameter
```

Both templates are functionally identical in CloudFormation.

## Maintenance

Since this code is vendored, updates won't happen automatically. Monitor these repositories for security fixes or important updates:

1. **Primary source**: [cloudformation-guard](https://github.com/aws-cloudformation/cloudformation-guard)
2. **Upstream source**: [serde_yaml](https://github.com/dtolnay/serde-yaml)
3. **Native library**: [unsafe-libyaml](https://github.com/dtolnay/unsafe-libyaml)

The YAML parsing code is stable and mature, but should be checked periodically for security updates.
