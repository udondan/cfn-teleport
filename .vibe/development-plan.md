# Development Plan: cfn-teleport (main branch)

*Generated on 2026-02-22 by Vibe Feature MCP*
*Workflow: [sdd-bugfix](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-bugfix)*

## Goal
Fix issue #167: JSON parsing error "expected value at line 1 column 1" when migrating S3 bucket resources between CloudFormation stacks using cfn-teleport
## Key Decisions
- **Root Cause Identified**: AWS CloudFormation `get_template()` API returns templates in YAML format when they were originally created as YAML. The code at `src/main.rs:956` uses `serde_json::from_str()` which only parses JSON, causing the error when encountering YAML templates.
- **Fix Strategy**: Implemented try-JSON-first with YAML fallback approach. This maintains performance for JSON templates (majority case) while adding YAML support transparently.
- **Implementation**: Modified only the `get_template()` function (12 lines changed). Added `serde_yaml` dependency. No breaking changes.
- **Verification**: All tests pass (41/41 including 6 new tests). Integration tests confirm YAML migrations work. No regressions detected.
- **Format Preservation**: Discovered that templates are converted to JSON during updates. This is acceptable for the initial fix. Created follow-up issue for preserving original template format (YAML→YAML, JSON→JSON).

## Notes
- Issue #167: User reports error when migrating S3 bucket from Stack1 to Stack2
- Error message: "expected value at line 1 column 1" - indicates JSON parsing failure
- Reproduction infrastructure created in test/issues/issue-167/ directory
- Test stacks deployed in eu-west-1 region using AWS account 031700846815
- Both stacks contain S3 buckets with unique names to avoid conflicts
- **ROOT CAUSE**: CloudFormation API returns templates in their original format (YAML or JSON). When templates are created as YAML, `get_template()` returns YAML, but the code tries to parse as JSON only.

## Reproduce
### Tasks

### Completed
- [x] Created development plan file
- [x] Created test directory test/issues/issue-167/
- [x] Created Stack1 CloudFormation template with two S3 buckets
- [x] Created Stack2 CloudFormation template with two S3 buckets  
- [x] Deployed Stack1 to AWS (eu-west-1)
- [x] Deployed Stack2 to AWS (eu-west-1)
- [x] Built cfn-teleport release binary (v0.47.0)
- [x] Ran migration command and confirmed error reproduction
- [x] Documented reproduction steps in test/issues/issue-167/README.md
- [x] Identified error location: src/main.rs:956 in get_template() function
- [x] Confirmed root cause: AWS API returns YAML templates but code only parses JSON

## Specify

### Phase Entrance Criteria:
- [x] Bug has been successfully reproduced with test infrastructure
- [x] Error message and behavior are documented
- [x] Root cause hypothesis has been identified

### Tasks

### Completed
- [x] Created bug specification document at .vibe/specs/main/bug-spec.md
- [x] Documented current (incorrect) and expected (correct) behavior
- [x] Defined success criteria (must have, should have, nice to have)
- [x] Identified test scenarios for validation
- [x] Assessed impact and scope of the fix

## Test

### Phase Entrance Criteria:
- [x] Bug specification is complete and documented
- [x] Expected behavior is clearly defined
- [x] Test scenarios have been identified

### Tasks

### Completed
- [x] Add serde_yaml dependency to Cargo.toml
- [x] Create unit test for parsing JSON templates (regression test)
- [x] Create unit test for parsing YAML templates (primary bug fix test)
- [x] Create unit test for format auto-detection (JSON first, then YAML)
- [x] Create unit test for malformed template handling
- [x] Create unit test for YAML templates with CloudFormation structure
- [x] Run tests to confirm they pass with proper expectations

## Plan

### Phase Entrance Criteria:
- [x] Test cases have been written that demonstrate the bug and expected behavior
- [x] Tests verify both error conditions and expected success behavior
- [x] Test infrastructure is in place

### Tasks

### Completed
- [x] Analyzed root cause in get_template() function
- [x] Designed try-JSON-first with YAML fallback strategy
- [x] Created detailed fix plan document at .vibe/specs/main/fix-plan.md
- [x] Identified specific code changes needed (single function modification)
- [x] Assessed risks and mitigations
- [x] Defined validation checklist

## Fix

### Phase Entrance Criteria:
- [x] Implementation plan is documented with clear approach
- [x] Code changes are scoped and understood
- [x] Potential side effects have been considered

### Tasks

### Completed
- [x] Modified get_template() function to support YAML parsing with JSON fallback
- [x] Added 6 comprehensive unit tests for template parsing
- [x] Ran unit tests - all 6 new tests pass
- [x] Ran full test suite - all 41 tests pass
- [x] Ran integration test with reproduction setup - migration successful!
- [x] Ran lint checks - no warnings
- [x] Ran format checks - code properly formatted
- [x] Built release binary - no errors
- [x] Noted format preservation as follow-up enhancement (YAML templates convert to JSON during updates)

## Verify

### Phase Entrance Criteria:
- [x] Bug fix has been implemented
- [x] Code compiles without errors or warnings
- [x] All tests pass successfully

### Tasks

### Completed
- [x] Verified original reproduction case is fixed (migrated MyBucket1 successfully)
- [x] Tested YAML template migration (multiple resources migrated successfully)
- [x] Tested reverse migration (bidirectional support confirmed)
- [x] Verified all 6 new unit tests pass
- [x] Verified all 41 total tests pass (no regressions)
- [x] Verified JSON template parsing works (regression test)
- [x] Verified error handling for malformed templates
- [x] Confirmed all success criteria from bug spec are met
- [x] Documented comprehensive verification summary
- [x] Created feature branch: fix/issue-167-yaml-template-support
- [x] Committed changes with conventional commit message
- [x] Noted format preservation as follow-up enhancement issue

## Summary

**Bug fix successfully completed!**

✅ Issue #167 fixed: YAML CloudFormation templates now parse correctly  
✅ All 41 tests pass (35 existing + 6 new)  
✅ Integration tests confirm YAML migrations work  
✅ No regressions detected  
✅ Code quality checks pass  
✅ Committed to branch: fix/issue-167-yaml-template-support

**Known Limitation**: Templates are converted to JSON format when updated (existing behavior).
Format preservation (YAML→YAML, JSON→JSON) should be addressed in a follow-up issue.

**Next Steps**: Push branch and create pull request.



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
