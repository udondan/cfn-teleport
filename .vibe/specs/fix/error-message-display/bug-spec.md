# Bug Specification: Error Message Display

## Bug Description

### Current Behavior (Incorrect)
When the application encounters validation errors related to resource selection (e.g., resources blocked by stack outputs or missing parameters), error messages are displayed in the terminal with escaped newline characters (`\n`) shown as literal text instead of actual line breaks.

**Example of current output:**
```
Error: "Cannot select the following resources because they are referenced by stack outputs:\n  - RenameTable6568E4D3 (AWS::DynamoDB::Table)\nOutputs cannot be moved between stacks. Consider:\n- Removing or updating the outputs before moving\n- Using same-stack rename instead (references will be updated automatically)"
```

### Expected Behavior (Correct)
Error messages should be displayed with proper formatting, where newline characters create actual line breaks for improved readability.

**Expected output:**
```
Cannot select the following resources because they are referenced by stack outputs:
  - RenameTable6568E4D3 (AWS::DynamoDB::Table)

Outputs cannot be moved between stacks. Consider:
- Removing or updating the outputs before moving
- Using same-stack rename instead (references will be updated automatically)
```

## Root Cause

**Technical Issue:**
- Error messages are constructed with embedded newline characters (`\n`) at lines 789-795 and 812-818 in `src/main.rs`
- These messages are returned as `Err()` values at line 823: `return Err(error_messages.join("\n\n").into())`
- When errors propagate to `main()` (which returns `Result<(), Box<dyn Error>>`), Rust's default error handling prints them using `Debug` formatting
- `Debug` formatting displays the error as a quoted string literal, showing `\n` as escaped characters instead of interpreting them as line breaks

**Why Some Errors Work Correctly:**
- Other error messages in the codebase (e.g., lines 92-111, 174, 420-433) use `eprintln!()` directly
- Direct `eprintln!()` calls properly interpret newline characters and format output correctly

## Affected Components

### Affected Error Messages
1. **Line 789-795:** "Cannot select the following resources because they are referenced by stack outputs"
   - Triggered when attempting to move resources that are referenced by CloudFormation stack outputs
   - Includes a bulleted list of blocked resources
   
2. **Line 812-818:** "Cannot select the following resources because they depend on parameters not present in target stack"
   - Triggered when attempting to move resources that depend on stack parameters not available in the target stack
   - Includes resource list with missing parameter details

### Function Context
- **Function:** Resource selection logic in `src/main.rs`
- **Error return location:** Line 823
- **Trigger:** Interactive resource selection when validation checks fail

## Correct Behavior Specification

### Requirements

**R1: Error Message Formatting**
- All error messages must display with proper line breaks in the terminal
- Multi-line error messages must be human-readable without escaped control characters
- Formatting must be consistent with other error messages in the application

**R2: Error Output Method**
- Error messages should be printed directly to stderr using appropriate formatting mechanisms
- Error messages must not rely on Rust's default Debug formatting for display
- The application should maintain consistent error handling patterns throughout

**R3: User Experience**
- Users must be able to easily read and understand error messages
- Error messages must clearly identify:
  - What went wrong
  - Which resources are affected
  - What actions the user can take to resolve the issue
- List formatting (bullet points, indentation) must be preserved

**R4: Backward Compatibility**
- The fix must not change error exit codes or behavior
- Error handling flow must remain unchanged
- All existing error messages must continue to work as expected

### Success Criteria

1. ✅ When validation errors occur, error messages display with actual line breaks
2. ✅ No escaped characters (`\n`, `\t`, etc.) are visible in terminal output
3. ✅ Resource lists are properly formatted with indentation and bullets
4. ✅ Error messages include proper spacing between sections
5. ✅ The application exits with an appropriate error code
6. ✅ All other error messages continue to work correctly
7. ✅ The fix follows existing code style and patterns in the codebase

## Test Scenarios

### Test Case 1: Resources Blocked by Stack Outputs
**Precondition:** A resource in the source stack is referenced by a stack output

**Steps:**
1. Run cfn-teleport to move resources between stacks
2. Attempt to select a resource that is referenced by a stack output
3. Observe the error message

**Expected Result:**
```
Cannot select the following resources because they are referenced by stack outputs:
  - ResourceLogicalId (AWS::ResourceType)

Outputs cannot be moved between stacks. Consider:
- Removing or updating the outputs before moving
- Using same-stack rename instead (references will be updated automatically)
```

### Test Case 2: Resources Blocked by Missing Parameters
**Precondition:** A resource in the source stack depends on stack parameters not present in target stack

**Steps:**
1. Run cfn-teleport to move resources between stacks
2. Attempt to select a resource that depends on missing parameters
3. Observe the error message

**Expected Result:**
```
Cannot select the following resources because they depend on parameters not present in target stack:
  - ResourceLogicalId (AWS::ResourceType) - missing parameters: ParamName1, ParamName2

Consider:
- Adding the required parameters to the target stack
- Removing parameter dependencies from these resources
```

### Test Case 3: Multiple Validation Errors
**Precondition:** Multiple validation errors occur simultaneously (both output and parameter issues)

**Steps:**
1. Run cfn-teleport with resources that have both types of validation issues
2. Observe that both error messages are properly formatted and separated

**Expected Result:**
Both error messages display with proper formatting, separated by blank lines

### Test Case 4: Existing Errors Still Work
**Precondition:** Normal operation of the application

**Steps:**
1. Test AWS credential errors (line 92-111)
2. Test other validation errors (line 174, 420-433)
3. Verify all existing errors still display correctly

**Expected Result:**
All existing error messages continue to work as before

## Edge Cases

1. **Single Resource Blocked:** Error message with one item in the list
2. **Multiple Resources Blocked:** Error message with many items in the list
3. **Long Resource Names:** Ensure formatting remains correct with long logical IDs or type names
4. **No Resources Selected:** Other error paths should not be affected

## Out of Scope

- Changing error message content or wording
- Adding new validation rules
- Modifying error exit codes
- Internationalizing error messages
- Adding colors or special formatting to errors

## Implementation Constraints

1. Must follow Rust 2021 edition conventions
2. Must pass `cargo fmt` and `cargo clippy` checks
3. Must maintain consistency with existing error handling patterns
4. Should minimize changes to existing code structure
5. Must not introduce new dependencies

## References

- **Affected Code:** `src/main.rs` lines 789-795, 812-818, 823
- **Working Examples:** `src/main.rs` lines 92-111, 174, 420-433
- **Related Documentation:** AGENTS.md error handling section
