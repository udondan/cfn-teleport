# Feature Specification: Improved Dependency Validation Error Messages

**Status:** Draft  
**Created:** 2026-02-22  
**Branch:** feat/improve-dependency-validation-errors

---

## Overview

### Problem Statement
When users attempt to move CloudFormation resources between stacks, if the selected resources have dependencies on resources that are NOT being moved, the tool currently shows a generic error message from CloudFormation ("ValidationError") instead of clearly explaining what the actual problem is and how to fix it.

### User Impact
- **Current Experience**: Confusing generic error messages that require users to manually inspect templates to understand what went wrong
- **Target Experience**: Clear, actionable error messages that explicitly identify missing dependencies and suggest solutions

### Business Value
- Reduces user frustration and support burden
- Improves tool usability and adoption
- Prevents wasted time debugging CloudFormation errors
- Maintains consistency with existing error handling patterns in the tool

---

## Functional Requirements

### FR1: Detect Unresolved Dependencies
**Requirement**: The tool SHALL detect when a resource being moved depends on resources that are staying in the source stack

**Details**:
- Check applies to all cross-stack move operations (both refactor and import modes)
- Does NOT apply to same-stack renames (all resources stay together)
- Detection covers all CloudFormation reference types:
  - `Ref` intrinsic function
  - `Fn::GetAtt` intrinsic function
  - `Fn::Sub` with resource references
  - `DependsOn` attribute

**Examples of what should be detected**:
- EC2 Instance being moved depends on SecurityGroup staying in source
- Lambda Function being moved references S3 Bucket in environment variables, bucket stays in source
- DynamoDB Table being moved has `DependsOn` relationship with S3 Bucket staying in source

### FR2: Display Dependency Chain
**Requirement**: The tool SHALL display a clear list of unresolved dependencies showing which resources depend on which other resources

**Format**:
```
Cannot move resources due to unresolved dependencies:

  - Resource 'Instance' depends on 'SecurityGroup' which is not being moved
  - Resource 'Instance' depends on 'Role' which is not being moved
  - Resource 'LambdaFunction' references 'TargetBucket' which is not being moved

Either move all dependent resources together, or remove dependencies before moving.
```

**Details**:
- Each dependency clearly identifies:
  - The resource being moved (referencing resource)
  - The resource it depends on (referenced resource)
  - Plain language explanation ("depends on", "references")
- Multiple dependencies for the same resource are shown as separate lines
- Grouped logically by the resource being moved

### FR3: Suggest Resolution Actions
**Requirement**: The tool SHALL provide actionable guidance on how to resolve the dependency issues

**Suggestions include**:
- Move all dependent resources together
- Remove or update dependencies before moving
- Consider same-stack renaming if only IDs need to change

### FR4: Prevent Invalid Operations
**Requirement**: The tool SHALL prevent the move operation from proceeding when unresolved dependencies are detected

**Details**:
- Validation happens BEFORE any AWS API calls are made
- No partial state changes occur
- Error message displayed to user with exit code 1

### FR5: Validate Bidirectional References
**Requirement**: The tool SHALL check BOTH directions of dependencies:
1. Resources staying that reference resources moving (already implemented)
2. Resources moving that depend on resources staying (NEW)

**Details**:
- Builds on existing `validate_move_references()` function
- Uses existing reference detection from `reference_updater` module
- Consistent error message format for both directions

---

## Non-Functional Requirements

### NFR1: Performance
**Requirement**: Dependency validation SHALL complete within 2 seconds for stacks with up to 200 resources

**Rationale**: Validation should not significantly slow down the user experience

### NFR2: Accuracy
**Requirement**: Dependency detection SHALL have zero false negatives (missed dependencies)

**Rationale**: False negatives lead to confusing CloudFormation errors - the exact problem we're solving

**Acceptable**: Low rate of false positives (flagging valid moves) is acceptable if edge cases are rare

### NFR3: Consistency
**Requirement**: Error message format SHALL match existing error patterns in the tool

**Details**:
- Multi-line format with bullet points
- Clear "Cannot move resources due to..." prefix
- Actionable tips at the bottom
- Similar to existing `validate_move_references()` output

### NFR4: Maintainability
**Requirement**: Implementation SHALL reuse existing reference detection logic

**Details**:
- Leverage `reference_updater::find_all_references()`
- Follow existing code patterns and conventions
- Add unit tests alongside existing test infrastructure

---

## Success Criteria

### SC1: Clear Error Messages
**Criteria**: When moving an EC2 Instance that depends on SecurityGroup and Role (both staying), the error message explicitly lists both dependencies

**Test**: Use CDK test stack `CfnTeleportTest1` and attempt to move `Instance` alone

### SC2: Multi-Resource Moves Succeed
**Criteria**: When moving Instance + SecurityGroup + Role together, validation passes and resources move successfully

**Test**: Select all three resources in multi-select prompt

### SC3: No False Positives
**Criteria**: Moving standalone resources (no dependencies) continues to work without errors

**Test**: Move `StandaloneBucket` or `StandaloneTable` from test stack

### SC4: All Reference Types Detected
**Criteria**: Dependencies are detected for all CloudFormation reference patterns:
- `Ref` - direct resource reference
- `Fn::GetAtt` - resource attribute reference
- `Fn::Sub` - resource reference in substitution string
- `DependsOn` - explicit dependency declaration

**Test**: Use Lambda+Bucket, Table with DependsOn, and other test scenarios

### SC5: Both Modes Covered
**Criteria**: Validation works identically in both refactor mode and import mode

**Test**: Run validation scenarios with `--mode=refactor` and `--mode=import`

### SC6: Existing Validation Preserved
**Criteria**: Existing validation for "staying resources referencing moving resources" continues to work

**Test**: Regression test against Output references and other existing scenarios

---

## Scope

### In Scope
- Detection of unresolved outgoing dependencies (moving resources depend on staying resources)
- Clear error messages listing all dependency issues
- Works in both refactor and import operational modes
- All CloudFormation intrinsic function reference types
- Integration with existing validation flow

### Out of Scope
- Detection of cross-stack references via CloudFormation exports (already handled by CloudFormation)
- Validation of parameter availability in target stack (separate concern, partially handled in import mode)
- Automatic resolution of dependencies (e.g., automatically selecting dependent resources)
- Detection of implicit dependencies not expressed in template (e.g., S3 bucket referenced by name string)
- Dependency visualization or graph display
- Warning for complex dependency chains (multiple levels deep)

---

## Edge Cases and Constraints

### EC1: Circular Dependencies
**Scenario**: Resource A references Resource B, Resource B references Resource A

**Handling**: Both resources MUST be moved together - validation should pass if both are selected

**Implementation Note**: Existing logic already handles this correctly by checking set membership

### EC2: Pseudo-Parameters
**Scenario**: Resource references AWS::Region, AWS::AccountId, or other pseudo-parameters

**Handling**: NOT considered dependencies - these are always valid in any stack

**Implementation Note**: Already filtered in `reference_updater::collect_references()`

### EC3: Parameters
**Scenario**: Resource depends on stack parameters

**Handling**: 
- Import mode: Already validates and blocks (existing code)
- Refactor mode: Parameters may or may not exist in target stack - this is a separate validation concern

**Decision**: Do NOT flag parameter references as blocking dependencies in this feature

### EC4: Same-Stack Rename
**Scenario**: Source and target stack are the same (renaming resources in place)

**Handling**: Skip outgoing dependency validation entirely - all resources stay together

**Implementation Note**: Matches existing pattern (line 332-334 in main.rs)

### EC5: Resources Moving Together
**Scenario**: Instance depends on SecurityGroup, both are being moved

**Handling**: Validation should pass - this is a valid operation

**Logic**: Referenced resource ID is in the `id_mapping` keyset

### EC6: Empty or Missing References
**Scenario**: Resource definition has no references to other resources

**Handling**: Validation passes (no dependencies to check)

---

## Assumptions

1. **Reference detection is complete**: The existing `reference_updater::find_all_references()` function correctly identifies all resource references in CloudFormation templates
2. **CloudFormation semantics**: All meaningful dependencies are expressed through Ref, GetAtt, Sub, or DependsOn
3. **User intent**: Users want to be prevented from making invalid moves, even if it requires extra steps
4. **Error visibility**: Users will read error messages and understand the guidance provided
5. **Test coverage**: CDK test stacks represent realistic scenarios users will encounter

---

## Constraints

1. **No API changes**: Cannot modify CloudFormation APIs - must work with existing validation APIs
2. **No breaking changes**: Existing valid operations must continue to work exactly as before
3. **Performance**: Validation must complete quickly (under 2 seconds for typical stacks)
4. **Rust ecosystem**: Implementation limited to capabilities of AWS SDK for Rust and serde_json
5. **CLI interface**: Error messages displayed via stderr/stdout (no GUI)

---

## Dependencies

### Internal Dependencies
- `reference_updater::find_all_references()` - existing function for detecting references
- `validate_move_references()` - existing validation function (will be extended)
- `id_mapping: HashMap<String, String>` - map of resources being moved

### External Dependencies
- CloudFormation template structure and semantics
- AWS SDK for Rust
- serde_json for template parsing

### Test Dependencies
- CDK test stacks in `test/cdk/` with existing dependency scenarios
- AWS credentials for integration testing

---

## Open Questions

None - all requirements are clear based on analysis phase findings.

---

## Acceptance Criteria Summary

✅ **Must Have**:
1. Detect when moving resources depend on staying resources
2. Display clear error messages listing each dependency
3. Prevent invalid move operations before AWS API calls
4. Work in both refactor and import modes
5. Handle all reference types (Ref, GetAtt, Sub, DependsOn)
6. No regression in existing validation

✅ **Should Have**:
1. Consistent error format with existing messages
2. Actionable resolution suggestions
3. Zero false negatives

✅ **Won't Have (this iteration)**:
1. Automatic dependency resolution
2. Dependency graph visualization
3. Detection of implicit string-based dependencies
