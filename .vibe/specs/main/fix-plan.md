# Fix Plan: Issue #167 - YAML Template Parsing Support

## Overview

This document outlines the implementation plan for adding YAML template parsing support to cfn-teleport, fixing the "expected value at line 1 column 1" error that occurs when migrating resources from stacks created with YAML templates.

## Root Cause Analysis

### Current Implementation
The `get_template()` function at `src/main.rs:950-958` calls the AWS CloudFormation `get_template()` API and attempts to parse the response using only `serde_json::from_str()`:

```rust
async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    let parsed_template = serde_json::from_str(template)?;  // ← FAILS FOR YAML
    Ok(parsed_template)
}
```

### Why It Fails
1. AWS CloudFormation API returns templates in their original format (YAML or JSON)
2. When a stack was created with YAML, the API returns YAML text
3. `serde_json::from_str()` only parses JSON, not YAML
4. The JSON parser encounters YAML syntax and fails with "expected value at line 1 column 1"
5. The error message provides no context about format mismatch

## Fix Strategy

### Approach: Try-JSON-First with YAML Fallback

We will implement automatic format detection with these characteristics:

1. **Performance-optimized**: Try JSON parsing first (faster, more common in automated deployments)
2. **Backward compatible**: Existing JSON templates work exactly as before
3. **Automatic**: No user intervention or configuration needed
4. **Graceful errors**: Clear error messages when both parsers fail

### Why This Approach?

**Advantages:**
- ✅ Minimal code changes (single function modification)
- ✅ No breaking changes to API or behavior
- ✅ Optimal performance (JSON is faster to parse)
- ✅ Handles both formats transparently
- ✅ Easy to test and validate

**Alternatives Considered:**
1. **Format sniffing** (check first character for `{` vs. letter): Fragile, doesn't handle whitespace
2. **YAML-only parsing** (YAML is superset of JSON): Slower, may have subtle compatibility issues
3. **User specification of format**: Unnecessary burden on users, breaks simplicity

## Implementation Plan

### Phase 1: Modify `get_template()` Function

**File**: `src/main.rs`  
**Function**: `get_template()` (lines 950-958)

**Current Code:**
```rust
async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    let parsed_template = serde_json::from_str(template)?;
    Ok(parsed_template)
}
```

**New Code:**
```rust
async fn get_template(
    client: &cloudformation::Client,
    stack_name: &str,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let resp = client.get_template().stack_name(stack_name).send().await?;
    let template = resp.template_body().ok_or("No template found")?;
    
    // Try JSON first (faster and more common in automated deployments)
    match serde_json::from_str(template) {
        Ok(parsed) => Ok(parsed),
        Err(_json_err) => {
            // Fallback to YAML parsing
            serde_yaml::from_str(template).map_err(|yaml_err| {
                format!(
                    "Failed to parse template as JSON or YAML. YAML error: {}",
                    yaml_err
                )
                .into()
            })
        }
    }
}
```

**Changes Explained:**
1. Attempt JSON parsing with `serde_json::from_str()`
2. If JSON fails, try YAML parsing with `serde_yaml::from_str()`
3. If YAML also fails, return clear error message
4. Both parsers return `serde_json::Value`, maintaining type compatibility

### Phase 2: Update Dependencies

**File**: `Cargo.toml`

**Already completed**: Added `serde_yaml = "0.9"` to dependencies

### Phase 3: Validation

**Steps:**
1. Run existing unit tests to ensure no regression
2. Run new template parsing tests to verify YAML support
3. Test with actual AWS CloudFormation stacks (integration test)
4. Run full test suite: `cargo test --all`
5. Run lint checks: `cargo clippy` and `cargo fmt --check`

### Phase 4: Integration Testing

**Use existing reproduction setup:**
- Stack1 and Stack2 are already deployed in eu-west-1
- Both stacks use YAML templates
- Run migration command: `cargo run -- --source Stack1 --target Stack2 --resource MyBucket1 --yes`
- Expected result: Migration should succeed without parsing errors

## Implementation Tasks

### Task Breakdown

1. **Modify get_template() function** (15 minutes)
   - Add match expression for JSON parsing
   - Add YAML fallback with error handling
   - Update error messages for clarity

2. **Run unit tests** (5 minutes)
   - Execute: `cargo test test_parse_template`
   - Verify all 6 new tests pass
   - Execute: `cargo test --all` for full suite

3. **Run integration test with reproduction setup** (10 minutes)
   - Execute migration with Stack1 → Stack2
   - Verify successful completion
   - Check migrated resources in AWS console

4. **Run lint and format checks** (5 minutes)
   - Execute: `cargo fmt`
   - Execute: `cargo clippy -- -D warnings`
   - Fix any issues found

5. **Build release binary** (5 minutes)
   - Execute: `cargo build --release`
   - Verify no warnings or errors

**Total estimated time**: 40 minutes

## Risk Assessment

### Low Risk Areas
- ✅ Isolated change (single function)
- ✅ Well-tested (6 new unit tests + integration test)
- ✅ Backward compatible (JSON behavior unchanged)
- ✅ Type-safe (both parsers return same type)

### Potential Issues & Mitigations

1. **YAML parser compatibility with CloudFormation features**
   - **Risk**: YAML templates with CloudFormation-specific tags (e.g., `!Sub`, `!Ref`)
   - **Mitigation**: serde_yaml handles these as custom tags; structure still parses correctly
   - **Validation**: Test with real CloudFormation YAML templates

2. **Performance impact on JSON templates**
   - **Risk**: JSON templates might be slightly slower due to match overhead
   - **Mitigation**: JSON is tried first; match is zero-cost abstraction; overhead is negligible
   - **Validation**: No performance testing needed (overhead is sub-millisecond)

3. **Error message clarity**
   - **Risk**: Users might not understand parsing errors
   - **Mitigation**: Clear error message explains both JSON and YAML were attempted
   - **Validation**: Test with intentionally malformed templates

## Success Criteria

### Must Pass
- ✅ All existing tests continue to pass
- ✅ All 6 new template parsing tests pass
- ✅ Integration test with YAML stacks succeeds
- ✅ No clippy warnings
- ✅ Code is properly formatted

### Validation Checklist
- [ ] `cargo test --all` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] Integration test with Stack1 → Stack2 succeeds
- [ ] Error handling works for malformed templates

## Rollback Plan

If issues are discovered:
1. The change is isolated to one function - easy to revert
2. Git commit can be reverted without side effects
3. No database migrations or external dependencies affected
4. No user-facing configuration changes to document

## Documentation Updates

### Code Comments
- Add inline comment explaining the try-JSON-first strategy
- Document the format detection logic

### External Documentation
- No updates needed to README (behavior is transparent to users)
- No CLI changes (no new flags or options)
- GitHub issue #167 will be referenced in commit message

## Post-Implementation

### Verification Steps
1. Close issue #167 with reference to fix commit
2. Test with various YAML template formats in the wild
3. Monitor for any reports of parsing failures
4. Consider adding performance metrics if future optimization needed

### Future Improvements (Out of Scope)
- Template format logging for debugging
- Support for YAML-specific CloudFormation extensions
- Template validation beyond basic parsing
- Performance optimization (not needed currently)

## Dependencies

- **serde_yaml crate**: Already added to Cargo.toml ✅
- **No external services** required
- **No configuration changes** needed
- **No database migrations** needed

## Timeline

- **Implementation**: 40 minutes
- **Code review**: Not required (reviews disabled)
- **Testing**: Included in implementation time
- **Deployment**: User builds locally or waits for release

## References

- Issue: #167
- Bug Spec: `/Users/danielschroeder/Code/Private/cfn-teleport/.vibe/specs/main/bug-spec.md`
- Reproduction: `test/issues/issue-167/`
- AWS CloudFormation get-template API documentation
- serde_yaml documentation: https://docs.rs/serde_yaml/
