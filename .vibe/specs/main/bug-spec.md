# Bug Specification: Issue #167 - YAML Template Parsing Failure

## Problem Statement

cfn-teleport fails with error "expected value at line 1 column 1" when attempting to migrate resources from CloudFormation stacks that were created using YAML templates. The error occurs during template retrieval, preventing any resource migration operations.

## Root Cause

The `get_template()` function in `src/main.rs:950-958` uses `serde_json::from_str()` to parse templates returned by the AWS CloudFormation API. However, the CloudFormation API returns templates in their original format (YAML or JSON). When a stack was created with a YAML template, the API returns YAML, which cannot be parsed by the JSON-only parser.

## Current Behavior (Incorrect)

1. User runs: `cfn-teleport --source Stack1 --target Stack2 --resource MyBucket1 --yes`
2. Code calls AWS CloudFormation `get_template()` API
3. API returns template in YAML format (original format)
4. Code attempts to parse YAML string as JSON using `serde_json::from_str()`
5. JSON parser fails with error: "expected value at line 1 column 1"
6. Program terminates with unhelpful error message

## Expected Behavior (Correct)

1. User runs: `cfn-teleport --source Stack1 --target Stack2 --resource MyBucket1 --yes`
2. Code calls AWS CloudFormation `get_template()` API
3. API returns template in YAML or JSON format
4. Code detects the template format and parses accordingly:
   - If JSON: parse with `serde_json::from_str()`
   - If YAML: parse with YAML parser (e.g., `serde_yaml::from_str()`)
5. Parsed template is converted to consistent internal representation (`serde_json::Value`)
6. Migration proceeds normally with dependency analysis and resource transfer

## Success Criteria

### Must Have
1. ✅ cfn-teleport successfully parses CloudFormation templates in YAML format
2. ✅ cfn-teleport successfully parses CloudFormation templates in JSON format
3. ✅ Both YAML and JSON templates are converted to the same internal representation
4. ✅ Existing JSON template functionality continues to work without regression
5. ✅ Error messages are clear when template parsing fails for legitimate reasons

### Should Have
1. ✅ Automatic format detection (no user intervention required)
2. ✅ Support for both YAML 1.1 and YAML 1.2 (CloudFormation uses YAML 1.1)
3. ✅ Graceful handling of malformed templates with helpful error messages

### Nice to Have
1. Template format logging for debugging purposes
2. Support for templates with CloudFormation-specific YAML extensions

## Test Scenarios

### Scenario 1: YAML Template Migration (Primary Bug Fix)
**Given**: Two stacks (Stack1, Stack2) created with YAML templates  
**When**: User runs `cfn-teleport --source Stack1 --target Stack2 --resource MyBucket1`  
**Then**: Migration succeeds without parsing errors

### Scenario 2: JSON Template Migration (Regression Test)
**Given**: Two stacks created with JSON templates  
**When**: User runs cfn-teleport migration command  
**Then**: Migration succeeds exactly as before the fix

### Scenario 3: Mixed Format Migration
**Given**: Stack1 created with YAML template, Stack2 created with JSON template  
**When**: User runs cfn-teleport migration command  
**Then**: Migration succeeds, handling both formats correctly

### Scenario 4: Malformed Template Handling
**Given**: A stack with a corrupted template (neither valid YAML nor JSON)  
**When**: User runs cfn-teleport migration command  
**Then**: Clear error message explains the template parsing failure

## Technical Considerations

### Template Format Detection
The code should detect template format by attempting JSON parsing first (faster), then falling back to YAML if JSON parsing fails. This approach:
- Maintains performance for JSON templates (majority case in automated deployments)
- Adds YAML support without breaking existing functionality
- Avoids complex format sniffing logic

### Dependencies
- Add `serde_yaml` crate to `Cargo.toml` for YAML parsing
- Ensure YAML parser outputs compatible `serde_json::Value` structure

### Backwards Compatibility
- No changes to command-line interface
- No changes to function signatures that are part of public API
- Internal representation remains `serde_json::Value` for all templates

## Out of Scope

- Converting YAML templates to JSON format for storage
- Supporting YAML-specific CloudFormation features that don't exist in JSON
- Template validation beyond basic parsing
- Performance optimization of template parsing (current performance is acceptable)

## Impact Assessment

### User Impact
- **High positive impact**: Fixes critical bug preventing YAML template usage
- Users with YAML templates (common in manual deployments) can now use cfn-teleport
- No negative impact on existing JSON template users

### Code Impact
- **Low complexity**: Single function modification (`get_template`)
- **Low risk**: Change is isolated to template parsing
- **High testability**: Easy to create unit and integration tests

### Breaking Changes
- None. This is a pure bug fix with no API changes.

## References

- Issue: #167
- Reproduction: `test/issues/issue-167/`
- Code location: `src/main.rs:950-958` (get_template function)
- AWS Documentation: CloudFormation get-template API returns original format
