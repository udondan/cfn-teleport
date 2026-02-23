# Current State Analysis: CFN Refactoring for Cross-Stack Resource Moves

**Date**: 2026-02-21  
**Analyzer**: AI Assistant  
**Branch**: cfn-refactor

## Executive Summary

The cfn-teleport tool currently supports TWO approaches for resource operations:
1. **Same-stack renaming**: Uses AWS CloudFormation Stack Refactoring API (NEW, recently implemented)
2. **Cross-stack moves**: Uses legacy import/export changeset flow (OLD, problematic)

The problem: Cross-stack moves use a destructive approach that removes resources from source BEFORE validating they can be imported into target, leading to fucked-up states when imports fail.

## Current Implementation Details

### Same-Stack Rename (Working Well)
**Location**: `src/main.rs:832-989` (`refactor_stack_resources()`)

**Flow**:
1. Update template with renamed resources and update all references
2. Build `ResourceMapping` objects with source/destination `ResourceLocation` (same stack)
3. Create `StackDefinition` with updated template
4. Call `create_stack_refactor()` with single stack definition and resource mappings
5. Wait for validation (status = `CREATE_COMPLETE` or `CREATE_FAILED`)
6. If validation passes, call `execute_stack_refactor()`
7. Wait for execution (status = `EXECUTE_COMPLETE` or `EXECUTE_FAILED`)

**Key Insight**: CloudFormation validates the refactor BEFORE making any changes. If validation fails, nothing is modified.

### Cross-Stack Move (Current Problematic Flow)
**Location**: `src/main.rs:300-379`

**Flow**:
1. Add `DeletionPolicy: Retain` to source resources
2. Update source stack (wait for completion)
3. Remove resources from source template
4. Update source stack again (wait for completion) - **RESOURCES ARE NOW REMOVED**
5. Create import changeset for target stack
6. Execute changeset to import resources
7. Remove `DeletionPolicy` from target

**Problem**: Steps 4-6 are NOT atomic. If step 5 or 6 fails (validation error, resource conflict, etc.), resources are already removed from source stack but not in target. Fucked-up state.

## AWS SDK Types for Refactoring

From `src/main.rs:838`:
```rust
use cloudformation::types::{ResourceLocation, ResourceMapping, StackDefinition};
```

### ResourceLocation
- `stack_name`: Name of the stack
- `logical_resource_id`: Logical ID of the resource

### ResourceMapping
- `source`: ResourceLocation (source stack + logical ID)
- `destination`: ResourceLocation (destination stack + logical ID)

**Critical**: `ResourceLocation.stack_name` can be DIFFERENT between source and destination! This is the key to cross-stack moves.

### StackDefinition
- `stack_name`: Name of the stack
- `template_body`: Updated template JSON string

**Critical**: `create_stack_refactor()` accepts multiple `StackDefinition` objects via `.stack_definitions()` method!

## Integration Points

### Reference Updater
**Location**: `src/reference_updater.rs`
- Already used in refactoring flow (line 863-864)
- Updates all `Ref`, `Fn::GetAtt`, `Fn::Sub`, etc. to use new logical IDs
- Works for same-stack renames, needs verification for cross-stack moves

### Validation Flow
- `validate_template()` called before refactor creation (line 867-869)
- CloudFormation also validates during `create_stack_refactor()` phase
- Double validation is good defensive programming

### User Confirmation
**Location**: `src/main.rs:189-297`
- Detects same-stack operations (line 194-209)
- Shows rename preview (line 261-280)
- Validates target conflicts (line 238-259)
- Same logic should apply to cross-stack moves

## Key Questions to Answer in Specification Phase

1. ✅ **API Support**: Does `create_stack_refactor()` actually support multiple stack definitions for cross-stack moves?
   - **CONFIRMED**: AWS docs state "Creates a refactor across multiple stacks"
   - `StackDefinitions.member.N` accepts array of StackDefinition objects (Type: Array)
   - `ResourceMapping` explicitly has source and destination with different StackNames
   - ResourceMapping docs: "Specifies the current source of the resource and the destination of where it will be moved to"

2. **Template Updates**: For cross-stack moves, what should each template contain?
   - Source template: Resources removed? Or just marked for move?
   - Target template: Resources added with same approach as current import?
   - Reference updates: Only in target? Or both stacks?

3. **DeletionPolicy**: Is it still needed?
   - Current flow adds `Retain` to prevent resource deletion
   - Does refactoring API handle this automatically?
   - Or do we still need to set it in source template?

4. **Validation Scope**: What does CloudFormation validate?
   - Resource conflicts in target stack?
   - References across stacks?
   - Resource type compatibility?
   - IAM permissions?

5. **Rollback Behavior**: What happens if refactor fails?
   - Does CloudFormation roll back both stacks?
   - Or leave them in intermediate state?
   - Better than current approach regardless?

## Assumptions

1. ✅ **CONFIRMED**: AWS CloudFormation Stack Refactoring API supports cross-stack resource moves
   - API explicitly designed for "multiple stacks"
   - ResourceMapping supports source.stack_name != destination.stack_name
2. The API will validate atomic moves (remove from source + add to target) before execution
3. If validation fails, NEITHER stack is modified (unlike current approach)
4. Reference updates still needed in templates we send to CloudFormation
5. Same user confirmation flow applies to cross-stack moves

## Technical Debt to Consider

- Current flow has manual `DeletionPolicy` management - can we eliminate this?
- Validation is done both locally (`validate_template`) and by AWS - optimize?
- Error messages from CFN refactoring API might be different than changeset errors
- Spinner/progress indication might need adjustment for multi-stack operations

## Success Criteria

The implementation is successful if:
1. ✅ Resources are moved from source to target atomically
2. ✅ If validation fails, NO stack is modified (no fucked-up state)
3. ✅ Reference updates work correctly across stacks
4. ✅ User gets clear error messages if move is not possible
5. ✅ Same user experience (prompts, confirmations) as current rename flow

## Next Steps

1. ✅ Research AWS documentation for cross-stack refactoring support - **CONFIRMED**
2. Specify how to structure templates for source and target stacks
3. Determine if DeletionPolicy management is still needed
4. Design error handling and user messaging
5. Plan reference update strategy for cross-stack moves

## AWS API Documentation References

- **CreateStackRefactor**: https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_CreateStackRefactor.html
  - "Creates a refactor across multiple stacks"
  - Accepts array of StackDefinition objects
  - Accepts array of ResourceMapping objects
  
- **ResourceMapping**: https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_ResourceMapping.html
  - "Specifies the current source of the resource and the destination of where it will be moved to"
  - Source: ResourceLocation (stack_name + logical_resource_id)
  - Destination: ResourceLocation (stack_name + logical_resource_id)
  
- **StackDefinition**: https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_StackDefinition.html
  - Contains stack_name and template_body
  - Multiple StackDefinitions describe final state of each affected stack
