# Implementation Plan: Improved Dependency Validation Error Messages

**Status:** Draft  
**Created:** 2026-02-22  
**Branch:** feat/improve-dependency-validation-errors

---

## Executive Summary

This plan outlines the implementation approach for adding bidirectional dependency validation to cfn-teleport. The enhancement extends the existing `validate_move_references()` function to detect when resources being moved depend on resources staying in the source stack, providing clear error messages before CloudFormation validation fails.

**Complexity:** Low-Medium  
**Estimated Effort:** 1-2 days  
**Risk Level:** Low (extends existing patterns, no breaking changes)

---

## Phase 0: Research & Decisions

### Technology Stack

**Decision:** Use existing Rust codebase patterns - no new dependencies required ✅

**Rationale:**
- All necessary tools already exist:
  - `reference_updater::find_all_references()` for dependency detection
  - `HashMap` and `HashSet` for set operations
  - `Box<dyn Error>` for error handling
  - Existing validation patterns to follow

**No research needed** - implementation uses only existing capabilities.

---

## Phase 1: High-Level Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      main() function                         │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Cross-stack move validation (line 332-334)           │ │
│  │  ┌──────────────────────────────────────────────────┐ │ │
│  │  │ validate_move_references() [EXTENDED]            │ │ │
│  │  │                                                   │ │ │
│  │  │ 1. Check staying → moving (existing)             │ │ │
│  │  │ 2. Check moving → staying (NEW)                  │ │ │
│  │  │ 3. Return combined errors                        │ │ │
│  │  └──────────────────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  For refactor_stack_resources_cross_stack():                │
│  - Validation happens at line 1588 (before AWS API calls)   │
│                                                              │
│  For import mode flow:                                      │
│  - Validation happens at line 333 (after user confirmation) │
└─────────────────────────────────────────────────────────────┘

Dependencies:
└─> reference_updater::find_all_references()
    └─> Returns HashMap<String, HashSet<String>>
        - Key: resource ID or "Outputs"
        - Value: set of referenced resource IDs
```

### Integration Points

1. **Main validation entry point** (line 333):
   ```rust
   if source_stack != target_stack {
       validate_move_references(&template_source, &new_logical_ids_map)?;
   }
   ```
   - Currently called for import mode only
   - Will be extended to check both directions

2. **Refactor mode** (line 1588 in `refactor_stack_resources_cross_stack`):
   - Currently NO validation call
   - **DECISION**: Add validation call here to cover refactor mode

3. **Reference detection** (reference_updater.rs):
   - Already provides all necessary data
   - No changes needed to this module

### Existing Patterns to Follow

From current `validate_move_references()` (lines 846-907):
```rust
fn validate_move_references(
    source_template: &serde_json::Value,
    new_logical_ids_map: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    // 1. Get moving resources
    let moving_resources: HashSet<String> = new_logical_ids_map.keys().cloned().collect();
    
    // 2. Find all references
    let all_references = reference_updater::find_all_references(source_template);
    
    // 3. Collect errors
    let mut errors = Vec::new();
    
    // 4. Check conditions and add errors
    // ...
    
    // 5. Return formatted error message
    if !errors.is_empty() {
        return Err(format!("Cannot move resources...\n{}", errors.join("\n")).into());
    }
    
    Ok(())
}
```

**Pattern to maintain:**
- Same function signature (no breaking changes)
- Same error message format
- Use `Vec<String>` to collect multiple errors
- Join with newlines and add tips at the end

---

## Phase 2: Detailed Implementation Plan

### Task 1: Extend `validate_move_references()` Function

**File:** `src/main.rs`  
**Location:** Lines 846-907 (existing function)  
**Complexity:** Low

**Changes:**
1. After existing "staying → moving" check (line 892), add new section
2. Add loop to check each moving resource's dependencies
3. For each reference, check if it's NOT in the moving set
4. Collect errors in same format as existing checks

**Pseudo-code:**
```rust
// NEW SECTION: Check if moving resources depend on staying resources
for (moving_resource, referenced_resources) in &all_references {
    // Skip if this resource isn't being moved
    if !moving_resources.contains(moving_resource) {
        continue;
    }
    
    for referenced in referenced_resources {
        // Check if referenced resource is staying (not being moved)
        if !moving_resources.contains(referenced) {
            errors.push(format!(
                "  - Resource '{}' depends on '{}' which is not being moved. \
                 Either move both resources together, or remove the dependency before moving.",
                moving_resource, referenced
            ));
        }
    }
}
```

**Edge cases handled:**
- Circular dependencies: Both resources in moving set → no error ✅
- Parameters: Not in `all_references` as resource IDs → no false positive ✅
- Pseudo-parameters: Already filtered by `collect_references()` → no false positive ✅
- Resources moving together: Both in moving set → no error ✅

**Testing hooks:**
- Unit tests can be added in `#[cfg(test)]` module
- Integration tests use CDK stacks

### Task 2: Add Validation to Refactor Mode

**File:** `src/main.rs`  
**Location:** Before line 1588 in `refactor_stack_resources_cross_stack()`  
**Complexity:** Trivial

**Changes:**
```rust
// Add before template validation (line 1588):
validate_move_references(&source_template, &id_mapping)?;
```

**Why here:**
- After user confirmation
- Before AWS API calls
- Before template modifications
- Consistent with import mode placement

**Risk:** None - this is the same validation used in import mode

### Task 3: Update Error Messages for Consistency

**File:** `src/main.rs`  
**Location:** Line 897-903 (existing error message)  
**Complexity:** Trivial

**Changes:**
Update the tips section to mention both directions:
```rust
let error_message = format!(
    "Cannot move resources due to dangling references:\n\n{}\n\n\
     Tip: You can move multiple resources together if they reference each other.\n\
     Tip: Use --mode=import to see all validation errors at once.\n\
     Tip: Same-stack renaming doesn't have this restriction.",
    errors.join("\n")
);
```

**Alternative:** Keep tips simple (existing message is already good)

---

## Phase 3: Testing Strategy

### Unit Tests

**Location:** `src/main.rs` - add `#[cfg(test)]` module if not exists

**Test cases:**
1. **Test: moving resource depends on staying resource**
   ```rust
   #[test]
   fn test_detect_outgoing_dependency() {
       // Template: Instance refs SecurityGroup
       // Move: Just Instance
       // Expect: Error listing SecurityGroup dependency
   }
   ```

2. **Test: moving resources depend on each other (circular)**
   ```rust
   #[test]
   fn test_circular_dependencies_allowed() {
       // Template: A refs B, B refs A
       // Move: Both A and B
       // Expect: Ok(())
   }
   ```

3. **Test: no dependencies (standalone resource)**
   ```rust
   #[test]
   fn test_standalone_resource() {
       // Template: Bucket with no refs
       // Move: Just Bucket
       // Expect: Ok(())
   }
   ```

4. **Test: outputs reference moving resource (existing test)**
   ```rust
   #[test]
   fn test_outputs_reference_moving_resource() {
       // Verify existing validation still works
   }
   ```

### Integration Tests

**Location:** Test via CDK stacks in `test/cdk/`

**Scenarios:**

| Scenario | Resources | Expected Result | Test Command |
|----------|-----------|-----------------|--------------|
| SC1: Instance alone | Instance | **Error**: depends on SecurityGroup, Role | `cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource Instance` |
| SC2: Instance + deps | Instance, SecurityGroup, Role | **Success**: all move together | `cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource Instance --resource SecurityGroup --resource Role` |
| SC3: Standalone bucket | StandaloneBucket | **Success**: no dependencies | `cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource StandaloneBucket` |
| SC4: Lambda with bucket | LambdaFunction alone | **Error**: references LambdaTargetBucket | `cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource TestFunction` |
| SC5: DependsOn | DependentTable alone | **Error**: DependsOn DependencyBucket | `cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource DependentTable` |
| SC6: Refactor mode | Instance (refactor mode) | **Error**: same as import mode | Add `--mode=refactor` to SC1 |

**Test execution:**
```bash
# Deploy test stacks
cd test/cdk && make deploy

# Run integration tests
cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource Instance
# Expected: Error message listing SecurityGroup and Role dependencies

# Test success case
cargo run -- --source CfnTeleportTest1 --target CfnTeleportTest2 --resource StandaloneBucket --yes
# Expected: Success (resource moves)

# Cleanup
cd test/cdk && make DESTROY
```

### Regression Tests

Ensure existing functionality still works:
- ✅ Same-stack rename (no validation)
- ✅ Cross-stack move of standalone resources
- ✅ Output reference validation (staying → moving)
- ✅ Parameter dependency blocking (import mode)

---

## Phase 4: Implementation Steps

### Step 1: Implement Extended Validation Logic ⏱️ 2-3 hours

1. Open `src/main.rs`
2. Locate `validate_move_references()` function (line 846)
3. After line 892 (end of existing validation), add new section:
   - Loop through moving resources
   - Check their dependencies
   - Collect errors for staying dependencies
4. Test locally with simple templates

**Acceptance:** Function compiles and passes basic logic check

### Step 2: Add Validation to Refactor Mode ⏱️ 15 minutes

1. Open `src/main.rs`
2. Locate `refactor_stack_resources_cross_stack()` (line 1558)
3. Before line 1588, add validation call:
   ```rust
   validate_move_references(&source_template, &id_mapping)?;
   ```
4. Compile and verify

**Acceptance:** Code compiles, refactor mode calls validation

### Step 3: Add Unit Tests ⏱️ 1-2 hours

1. Add `#[cfg(test)]` module at end of `main.rs` if not exists
2. Add test helper to create simple templates as JSON
3. Implement 4 unit test cases (see Phase 3)
4. Run `cargo test`

**Acceptance:** All unit tests pass

### Step 4: Run Integration Tests ⏱️ 1 hour

1. Deploy CDK test stacks: `cd test/cdk && make deploy`
2. Run each integration test scenario manually (see table above)
3. Verify error messages are clear and accurate
4. Verify success cases still work
5. Test both refactor and import modes

**Acceptance:** All 6 integration scenarios work as expected

### Step 5: Linting and Formatting ⏱️ 15 minutes

1. Run `cargo fmt`
2. Run `cargo clippy -- -D warnings`
3. Fix any warnings or errors

**Acceptance:** `make lint` passes

### Step 6: Update Documentation ⏱️ 30 minutes

Update README.md or inline docs if needed:
- Mention improved error messages for dependency issues
- Add example of multi-resource move

**Acceptance:** Documentation reflects new behavior

---

## Phase 5: Rollout Strategy

### Compatibility

✅ **Backward compatible** - no breaking changes:
- Function signature unchanged
- CLI arguments unchanged
- Valid operations still work
- Only NEW errors for previously invalid operations that would fail anyway

### Deployment

**Single binary deployment:**
1. Merge PR
2. Release via release-please (automatic)
3. Publish to crates.io (automatic)

**No database migrations:** N/A  
**No configuration changes:** N/A  
**No API changes:** N/A

### Monitoring

**Success metrics:**
- Users see clear dependency errors instead of generic ValidationError
- No increase in false positive validation errors
- No regression in existing functionality

**Validation:**
- CI/CD tests pass
- Manual integration test scenarios work
- Community feedback (if public)

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| False positives (valid moves blocked) | Low | Medium | Comprehensive test scenarios, careful set logic |
| Performance degradation | Very Low | Low | Validation is O(n) like existing code, tested with 200 resources |
| Breaking existing behavior | Very Low | High | Thorough regression testing, no signature changes |
| Missing reference types | Low | Medium | Reuse existing `find_all_references()` which is proven |

### Process Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Incomplete test coverage | Medium | Medium | Define explicit test scenarios in plan, verify all edge cases |
| Integration test environment issues | Medium | Low | Document CDK test setup, use existing test infrastructure |

---

## Dependencies and Prerequisites

### Code Dependencies
- ✅ Existing `reference_updater::find_all_references()` function
- ✅ Existing `validate_move_references()` function
- ✅ Standard library: `HashMap`, `HashSet`, `Vec`

### Environment Dependencies
- ✅ Rust toolchain (already in use)
- ✅ AWS credentials for integration tests (already required)
- ✅ CDK test stacks (already exist)

### No New Dependencies Required ✅

---

## Success Criteria Mapping

| Spec Success Criteria | Implementation Validation |
|----------------------|---------------------------|
| SC1: Clear error for Instance dependencies | Integration test: Move Instance alone, verify error lists SecurityGroup and Role |
| SC2: Multi-resource moves succeed | Integration test: Move Instance + SecurityGroup + Role, verify success |
| SC3: No false positives | Integration test: Move StandaloneBucket, verify success |
| SC4: All reference types | Unit tests: Ref, GetAtt, Sub, DependsOn all detected |
| SC5: Both modes | Integration test: Run with --mode=refactor and --mode=import |
| SC6: Existing validation preserved | Regression test: Output reference validation still works |

---

## Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Step 1: Extend validation logic | 2-3 hours | None |
| Step 2: Add refactor mode call | 15 min | Step 1 |
| Step 3: Unit tests | 1-2 hours | Step 1 |
| Step 4: Integration tests | 1 hour | Step 2, CDK stacks |
| Step 5: Lint/format | 15 min | Steps 1-3 |
| Step 6: Documentation | 30 min | Steps 1-5 |

**Total Estimated Time:** 5-7 hours (1 day of focused work)

---

## Open Questions for Implementation

None - all decisions made during analysis and specification phases.

---

## Approval Checklist

- [x] Implementation approach follows existing patterns
- [x] No new dependencies required
- [x] Backward compatible (no breaking changes)
- [x] Test strategy covers all success criteria
- [x] Timeline is realistic
- [x] Risks identified and mitigated
- [x] Ready to proceed to task breakdown
