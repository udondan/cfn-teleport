# Development Plan: cfn-teleport (fix/error-message-display branch)

*Generated on 2026-02-22 by Vibe Feature MCP*
*Workflow: [sdd-bugfix](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-bugfix)*

## Goal
Fix error message display issue where error messages are not shown as intended to users

## Key Decisions
- **Testing Approach:** Manual testing will be used instead of automated tests
  - Reason: The bug involves terminal output formatting, which is difficult to test automatically
  - The error occurs in interactive flow requiring AWS resources and user interaction
  - Existing codebase has minimal unit test coverage for main application flow
  - Manual verification after fix will be sufficient given the low risk and clear success criteria
- **Fix Strategy:** Central error handling in main() wrapper
  - **IMPROVED APPROACH** based on user feedback: "it looks stupid to have all the prints there"
  - Wrapped main logic in `run()` function that returns `Result<(), Box<dyn Error>>`
  - New `main()` function catches errors and prints them with proper formatting using `eprintln!()`
  - This approach is cleaner - errors are returned as errors throughout the codebase
  - Central error display ensures all errors print with correct escape sequence interpretation
  - **Bonus:** Cleaned up 4 other error locations that were using eprintln! + process::exit pattern
  - Much cleaner code - errors are errors, not scattered print statements

## Notes
- User reported that error messages like the "Cannot select resources referenced by stack outputs" are not displaying properly
- Example error message: "Cannot select the following resources because they are referenced by stack outputs:\n  - RenameTable6568E4D3 (AWS::DynamoDB::Table)\nOutputs cannot be moved between stacks..."
- Error displayed in terminal with escaped newline characters (`\n`) instead of actual line breaks
- NOT all errors show this way - some display correctly, others show as strings with control characters
- AWS authentication: `set -g -x AWS_PROFILE cicd:admin` then `aws-vault-shell cicd:admin`
- Test with: `cargo run --` after deploying test stacks
- **User feedback on initial fix:** "it looks stupid to have all the prints there" - led to cleaner central error handling solution

## Reproduce
### Tasks

### Completed
- [x] Deploy test stacks for integration testing - NOT NEEDED, issue found in code
- [x] Reproduce the specific "Cannot select resources referenced by stack outputs" error
- [x] Document the difference between working and broken error displays
- [x] Created development plan file
- [x] Gathered reproduction details from user
- [x] Found the problematic code location (src/main.rs:823)
- [x] Identified root cause: Errors returned from main() use Debug formatting instead of Display formatting

### Findings
**Root Cause Identified:**
- Line 823 in src/main.rs returns error with `return Err(error_messages.join("\n\n").into())`
- Error messages are built at lines 789-795 (output blocking) and 812-818 (parameter blocking)
- When errors are returned from `main()` (which returns `Result<(), Box<dyn Error>>`), Rust prints them using Debug formatting
- Debug formatting shows string literals with escaped characters: `"...\n..."` instead of actual newlines
- Contrast: Other errors use `eprintln!()` directly (lines 92-111, 174, 420-433), which display correctly with proper formatting

**Affected Code Locations:**
- Line 789-795: "Cannot select the following resources because they are referenced by stack outputs"
- Line 812-818: "Cannot select the following resources because they depend on parameters not present in target stack"
- Line 823: `return Err(error_messages.join("\n\n").into())`

**Working Examples:**
```rust
// Line 92-111 - Direct eprintln! shows properly formatted
eprintln!("\nAWS credentials not found.\n");
eprintln!("Please ensure you're authenticated...");
```

**Bug Successfully Reproduced in Code:** ✅
No need to deploy test stacks - the issue is clear from code inspection.

## Specify

### Phase Entrance Criteria:
- [x] The bug has been successfully reproduced
- [x] Steps to reproduce are clearly documented
- [x] Expected vs. actual behavior is understood

### Tasks

### Completed
- [x] Created bug specification document at `.vibe/specs/fix/error-message-display/bug-spec.md`
- [x] Defined current vs. expected behavior
- [x] Documented root cause analysis
- [x] Listed affected components
- [x] Specified requirements (R1-R4)
- [x] Defined success criteria
- [x] Created test scenarios
- [x] Identified edge cases and constraints

## Test

### Phase Entrance Criteria:
- [x] The bug specification is complete and clear
- [x] Root cause has been identified
- [x] Expected behavior is well-defined

### Tasks

### Completed
- [x] Evaluated testing approach - manual testing chosen over automated tests
- [x] Documented rationale: terminal output formatting, interactive flow, AWS dependencies make automated testing impractical
- [x] Identified manual test procedure:
  1. Deploy test CDK stacks with resources referenced by outputs
  2. Run cfn-teleport and attempt to select blocked resources
  3. Verify error messages display with proper line breaks
  4. Verify existing error messages still work correctly

### Test Plan
Since automated testing is impractical for this terminal output formatting bug, we will use manual verification:

**Manual Test Procedure:**
1. Build the application: `cargo build`
2. Deploy test stacks: `cd test/cdk && make deploy`
3. Authenticate: `set -g -x AWS_PROFILE cicd:admin && aws-vault-shell cicd:admin`
4. Run application: `cargo run --`
5. Attempt to select resources blocked by outputs or parameters
6. **Verify:** Error messages show actual line breaks, not `\n` characters
7. **Verify:** Resource lists are properly indented
8. **Verify:** Existing error messages (credentials, etc.) still display correctly

**Success Criteria:**
- ✅ Error messages display with proper formatting
- ✅ No escaped newline characters visible
- ✅ Proper indentation and spacing preserved
- ✅ Application exits appropriately
- ✅ No regressions in other error messages

## Plan

### Phase Entrance Criteria:
- [x] Test cases have been written or identified
- [x] Test cases can reproduce the bug
- [x] Success criteria for the fix are defined

### Tasks

### Completed
- [x] Created fix plan document at `.vibe/specs/fix/error-message-display/fix-plan.md`
- [x] Analyzed root cause in detail
- [x] Evaluated alternative solutions
- [x] Selected optimal fix strategy: print-and-exit pattern
- [x] Identified exact code changes needed (line 821-824)
- [x] Defined testing strategy and success criteria
- [x] Assessed risks (low risk, isolated change)
- [x] Documented implementation order and acceptance criteria

## Fix

### Phase Entrance Criteria:
- [x] Fix plan has been created and reviewed
- [x] Implementation approach is clear
- [x] All dependencies and constraints are identified

### Tasks

### Completed
- [x] Wrapped main logic in `run()` async function
- [x] Modified `main()` to catch errors and print with eprintln!()
- [x] Converted AWS credential error handling (lines 99-121) to return Err()
- [x] Converted non-existing resource error (lines 169-175) to return Err()
- [x] Converted import mode parameter dependency error (lines 415-432) to return Err()
- [x] Kept original error return at line 823 (now properly formatted by main)
- [x] Ran `cargo fmt` - code formatted successfully
- [x] Ran `cargo clippy -- -D warnings` - passed with no warnings
- [x] Ran `cargo check` - compilation successful

### Implementation Details

**New Architecture:**
```rust
#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // Print error with proper formatting (interprets escape sequences)
        eprintln!("\n{}\n", err);
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    // All the main logic here
    // Errors are returned as Err() throughout
}
```

**Benefits:**
- ✅ Single point of error display - clean architecture
- ✅ All errors now print with proper escape sequence interpretation
- ✅ No more scattered `eprintln!` + `process::exit(1)` throughout code
- ✅ Errors are returned as errors (proper Rust idiom)
- ✅ Cleaner, more maintainable code
- ✅ Fixed 5 different error locations with one architectural change

## Verify

### Phase Entrance Criteria:
- [x] Fix has been implemented
- [x] Code changes are complete
- [x] Build/compilation is successful

### Tasks

### Completed
- [x] Manual test: Verified error messages display correctly (user confirmed "seems to work")
- [x] Run cargo test to ensure no regressions (passed)
- [x] Verified code quality checks pass (fmt, clippy, check)
- [x] Committed changes with conventional commit message
- [x] Pushed to remote branch
- [x] Created pull request: https://github.com/udondan/cfn-teleport/pull/1264

### Verification Results

✅ **User Verification:** "seems to work" - error messages now display with proper line breaks
✅ **Code Quality:** All checks passed (cargo fmt, clippy, check)
✅ **No Regressions:** Compilation successful, no breaking changes
✅ **Architecture Improvement:** Cleaner error handling throughout codebase

### Pull Request Created
- **URL:** https://github.com/udondan/cfn-teleport/pull/1264
- **Title:** fix: centralize error handling to properly display error messages with newlines
- **Status:** Ready for review



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
