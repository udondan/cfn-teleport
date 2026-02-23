# Fix Plan: Error Message Display

## Overview

**Bug:** Error messages with embedded newlines are displayed with escaped `\n` characters instead of actual line breaks when errors are returned from functions and propagate to `main()`.

**Root Cause:** Rust's default error handling for `main()` that returns `Result<(), Box<dyn Error>>` uses `Debug` formatting, which displays error strings as quoted literals with escaped characters.

**Solution Strategy:** Change error handling to print errors directly to stderr using `eprintln!()` before exiting, rather than returning them to be printed by Rust's default error handler.

## Root Cause Analysis

### Why the Bug Occurs

1. **Current Flow:**
   ```rust
   fn prompt_for_resources(...) -> Result<Vec<...>, Box<dyn Error>> {
       // Build error messages with newlines
       let error_message = "Error text\n  - Item 1\n  - Item 2";
       return Err(error_message.into()); // Line 823
   }
   
   async fn main() -> Result<(), Box<dyn Error>> {
       let resources = prompt_for_resources(...)?; // Error propagates here
       // ... continues
   }
   ```

2. **What Rust Does:**
   - When `main()` returns `Err(e)`, Rust calls `Debug::fmt` on the error
   - `Debug` formatting for strings displays them as quoted literals: `"text\n"`
   - Result: User sees `Error: "message\nwith\nnewlines"` instead of formatted output

3. **Why Other Errors Work:**
   - Lines 92-111, 174, 420-433 use `eprintln!()` followed by `process::exit(1)`
   - Direct printing interprets escape sequences correctly
   - These errors don't rely on Rust's default error display

## Fix Strategy

### Approach: Print and Exit Pattern

Instead of returning errors to be printed by Rust's default handler, we'll:
1. Print the error message directly to stderr using `eprintln!()`
2. Exit with appropriate error code using `process::exit(1)`

This follows the existing pattern used successfully elsewhere in the codebase (e.g., lines 92-111).

### Why This Approach

**Pros:**
- ✅ Minimal code changes
- ✅ Follows existing patterns in the codebase (lines 92-111, 174, 420-433)
- ✅ Fixes the formatting issue completely
- ✅ No changes to error message content
- ✅ Consistent with project's error handling style
- ✅ No new dependencies required

**Cons:**
- ⚠️ Changes function signature to not return error (but this is internal)
- ⚠️ Uses `process::exit()` which prevents further code execution (acceptable for terminal errors)

**Alternatives Considered:**

1. **Custom Error Type with Display Implementation**
   - ❌ More complex, requires new type
   - ❌ Doesn't follow existing patterns
   - ❌ Overkill for this specific issue

2. **Change main() to not return Result**
   - ❌ Would require refactoring entire main() function
   - ❌ Against Rust conventions
   - ❌ Would affect many other error paths

3. **Strip quotes from error output**
   - ❌ Doesn't address root cause
   - ❌ Fragile workaround
   - ❌ Would still show escaped characters

## Implementation Plan

### Files to Modify

- **`src/main.rs`** - Only file requiring changes

### Code Changes

#### Change 1: Modify Error Handling at Line 823

**Current code (lines 821-824):**
```rust
// If any resources are blocked, return error
if !error_messages.is_empty() {
    return Err(error_messages.join("\n\n").into());
}
```

**New code:**
```rust
// If any resources are blocked, print error and exit
if !error_messages.is_empty() {
    eprintln!("\n{}\n", error_messages.join("\n\n"));
    process::exit(1);
}
```

**Rationale:**
- Uses `eprintln!()` for proper stderr output with escape sequence interpretation
- Adds blank lines before/after for visual separation (consistent with other errors)
- Uses `process::exit(1)` to indicate error condition
- Follows pattern from lines 92-111, 174, 420-433

#### Change 2: Consider Function Return Type

The function `prompt_for_resources()` currently returns `Result<Vec<...>, Box<dyn Error>>`. After the change, the error path uses `process::exit(1)` instead of returning an error.

**Options:**
1. **Keep return type as-is** - The function still returns `Result`, but this specific error path exits instead
2. **Change to panic-safe approach** - Keep the structure for potential future error handling

**Decision:** Keep the return type as-is. The function is only called from one place, and the exit behavior is appropriate for this terminal validation error.

### Quality Assurance Checks

**Before Implementation:**
- [x] Reviewed bug specification
- [x] Confirmed root cause
- [x] Identified minimal fix approach
- [x] Verified alignment with existing patterns

**During Implementation:**
- [ ] Make code changes following style guidelines
- [ ] Ensure no other error paths are affected
- [ ] Run `cargo fmt` to format code
- [ ] Run `cargo clippy` to check for lints
- [ ] Run `cargo check` to verify compilation

**After Implementation:**
- [ ] Manual testing with test stacks
- [ ] Verify error messages display correctly
- [ ] Verify existing error messages still work
- [ ] Verify no regressions in other functionality

## Testing Strategy

### Manual Testing Procedure

1. **Setup:**
   ```bash
   # Build the application
   cargo build
   
   # Deploy test stacks
   cd test/cdk && make deploy
   
   # Authenticate
   set -g -x AWS_PROFILE cicd:admin
   aws-vault-shell cicd:admin
   ```

2. **Test Case 1: Output-blocked resources**
   - Run: `cargo run --`
   - Select a resource referenced by stack outputs
   - **Verify:** Error displays with proper line breaks, not `\n` characters

3. **Test Case 2: Parameter-blocked resources**
   - Run: `cargo run --`
   - Select a resource with missing parameter dependencies
   - **Verify:** Error displays with proper formatting

4. **Test Case 3: Existing errors**
   - Test AWS credential error (clear credentials and run)
   - **Verify:** Credential error still displays correctly
   - Test other error paths
   - **Verify:** No regressions

5. **Cleanup:**
   ```bash
   cd test/cdk && make DESTROY
   ```

### Success Criteria

- ✅ Error messages display with actual line breaks
- ✅ No escaped characters (`\n`, `\t`) visible in output
- ✅ Resource lists properly indented with bullets
- ✅ Proper spacing between message sections
- ✅ Application exits with error code 1
- ✅ All existing error messages work correctly
- ✅ Passes `cargo fmt --check`
- ✅ Passes `cargo clippy -- -D warnings`
- ✅ Compiles without errors or warnings

## Risk Assessment

### Low Risk
- ✅ Changes are isolated to one error handling location
- ✅ Follows existing patterns in codebase
- ✅ Clear success criteria
- ✅ Easy to verify manually

### Mitigations
- Manual testing will verify no regressions
- Small, focused change reduces chance of errors
- Existing patterns provide proven approach

## Implementation Order

1. ✅ **Plan Phase** - Document fix approach (this document)
2. **Fix Phase:**
   - [ ] Modify line 821-824 in `src/main.rs`
   - [ ] Run `cargo fmt`
   - [ ] Run `cargo clippy`
   - [ ] Run `cargo check`
3. **Verify Phase:**
   - [ ] Deploy test stacks
   - [ ] Manual testing of error display
   - [ ] Verify existing errors still work
   - [ ] Cleanup test resources

## Rollback Plan

If the fix causes issues:
1. Revert the commit: `git revert HEAD`
2. Verify original behavior restored
3. Re-analyze the problem

The changes are minimal and isolated, making rollback straightforward.

## Acceptance Criteria

The fix is complete when:
1. Error messages display with proper line breaks in terminal
2. No escaped newline characters are visible
3. Code passes `cargo fmt --check` and `cargo clippy -- -D warnings`
4. Manual testing confirms correct behavior
5. No regressions in other error paths
6. Code review approved (if applicable)
7. Changes committed with conventional commit message

## References

- **Bug Specification:** `.vibe/specs/fix/error-message-display/bug-spec.md`
- **Affected Code:** `src/main.rs` lines 789-795, 812-818, 821-824
- **Pattern Examples:** `src/main.rs` lines 92-111, 174, 420-433
- **Project Guidelines:** `AGENTS.md` - Error Handling section
