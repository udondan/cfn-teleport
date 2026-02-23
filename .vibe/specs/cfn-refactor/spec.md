# Feature Specification: Cross-Stack Resource Move with CFN Refactoring

**Date**: 2026-02-21  
**Status**: Draft  
**Feature Type**: Enhancement

## Overview

Replace the current destructive cross-stack resource move flow with AWS CloudFormation Stack Refactoring API to provide atomic validation and execution, preventing data loss when resource moves fail.

## Problem Statement

Currently, when moving CloudFormation resources from one stack to another:
1. Resources are first removed from the source stack (using DeletionPolicy: Retain)
2. Then an import changeset is created for the target stack
3. If the import fails (e.g., resource name conflict, validation error), the resources are already removed from the source stack but not present in the target stack
4. This creates an unrecoverable state requiring manual intervention

## Goals

1. **Atomic Operations**: Ensure resources are moved atomically - either both stacks are updated successfully, or neither is modified
2. **Validation Before Execution**: CloudFormation validates the entire operation before making any changes
3. **Consistency**: Provide the same reliable user experience as same-stack resource renaming (already implemented with CFN refactoring)
4. **Error Prevention**: Eliminate the risk of resources being lost between stacks due to failed imports

## Non-Goals

- Changing the CLI interface or user interaction flow
- Supporting cross-region or cross-account resource moves (AWS API limitation)
- Modifying the same-stack rename implementation
- Adding new resource type support beyond what CloudFormation refactoring already supports

## Requirements

### Functional Requirements

#### FR1: Cross-Stack Resource Move
The system shall move CloudFormation resources from a source stack to a target stack using the CloudFormation Stack Refactoring API.

**Acceptance Criteria**:
- Resources specified by the user are removed from the source stack
- Same resources are added to the target stack with specified logical IDs
- All internal references within moved resources are updated to use new logical IDs
- Operation completes successfully or no changes are made to either stack

#### FR2: Pre-Move Validation
The system shall validate the entire move operation before modifying any stack.

**Acceptance Criteria**:
- CloudFormation validates both source and target templates
- Validation checks for resource naming conflicts in the target stack
- Validation checks for reference integrity in both stacks
- If validation fails, user receives clear error message and no stacks are modified

#### FR3: Reference Update
The system shall update all resource references (Ref, Fn::GetAtt, Fn::Sub, etc.) to use new logical IDs in the moved resources.

**Acceptance Criteria**:
- References within the moved resources are updated
- References in the target stack (if any) remain valid
- Template validation passes for both stacks after reference updates

#### FR4: User Confirmation
The system shall display a summary of the move operation and request user confirmation before proceeding.

**Acceptance Criteria**:
- User sees list of resources being moved
- User sees source and target stack names
- User sees new logical IDs (if different from source)
- User must explicitly confirm before execution

#### FR5: Operation Status Feedback
The system shall provide real-time feedback on the operation status.

**Acceptance Criteria**:
- User sees progress indicator during validation
- User sees progress indicator during execution
- User receives success confirmation with summary of changes
- User receives detailed error message if operation fails

### Non-Functional Requirements

#### NFR1: Reliability
The operation shall be atomic - either fully succeeds or fully fails without partial state.

**Measurement**: No scenario where resources are present in neither stack or both stacks after operation.

#### NFR2: Validation Time
Validation shall complete within reasonable time based on resource count.

**Measurement**: User receives validation result (success or failure) within time proportional to resource count.

#### NFR3: Error Clarity
Error messages shall clearly indicate why the operation failed and what action user should take.

**Measurement**: Error messages include specific resource names, conflict types, and resolution steps.

## User Workflows

### Primary Workflow: Move Resources Between Existing Stacks

**Preconditions**:
- User has AWS credentials configured
- Source and target stacks exist in the same region and account
- User has permission to modify both stacks
- Resources to be moved are supported by CloudFormation refactoring API

**Steps**:
1. User invokes cfn-teleport with source stack and target stack specified
2. System displays available resources in source stack
3. User selects resources to move
4. User specifies new logical IDs for resources (optional, defaults to original IDs)
5. System displays move summary and requests confirmation
6. User confirms the operation
7. System validates the move operation (source and target templates)
8. If validation succeeds, system executes the move
9. System confirms successful completion and displays moved resources

**Postconditions**:
- Selected resources are present in target stack with specified logical IDs
- Selected resources are removed from source stack
- Both stacks are in UPDATE_COMPLETE status
- All resource references are updated correctly

### Error Workflow: Validation Failure

**Scenario**: Target stack already contains a resource with conflicting logical ID

**Steps**:
1. User follows primary workflow steps 1-6
2. System validates the move operation
3. Validation fails due to naming conflict in target stack
4. System displays error message indicating:
   - Which resource names conflict
   - Current logical IDs in target stack
   - Suggestion to use different logical IDs
5. Neither stack is modified
6. User can retry with different logical IDs

**Postconditions**:
- Both stacks remain unchanged
- User understands why operation failed and how to resolve it

### Error Workflow: Execution Failure

**Scenario**: Execution fails during CloudFormation refactor operation

**Steps**:
1. User follows primary workflow steps 1-7
2. Validation succeeds
3. System begins execution
4. Execution fails (e.g., AWS service error, permission issue)
5. CloudFormation rolls back changes
6. System displays error message from CloudFormation
7. Both stacks return to previous state

**Postconditions**:
- Both stacks are rolled back to state before operation
- No resources are lost or orphaned
- User understands what failed

## Technical Constraints

### AWS API Constraints
1. CloudFormation Stack Refactoring API only supports same-region, same-account operations
2. Both source and target stacks must exist (no stack creation during refactor)
3. Resource types must be supported by CloudFormation refactoring API (subset of all CFN resource types)

### Behavioral Constraints
1. Operation may take significant time for stacks with many resources or complex dependencies
2. During operation, both stacks are in UPDATE_IN_PROGRESS state and cannot be modified by other operations
3. Some resource types may require additional IAM permissions for move operation

## Success Metrics

1. **Zero Data Loss**: No scenarios where resources exist in neither stack after failed operation
2. **Operation Success Rate**: Percentage of move operations that complete successfully after validation passes
3. **Validation Accuracy**: Validation correctly identifies conflicts and issues before execution
4. **User Recovery**: If operation fails, user can retry without manual cleanup

## Dependencies

1. AWS CloudFormation Stack Refactoring API availability in target regions
2. Existing reference updater module (already implemented)
3. Existing template validation module (already implemented)
4. AWS SDK for Rust CloudFormation types (ResourceLocation, ResourceMapping, StackDefinition)

## Out of Scope

1. Cross-region resource moves (AWS API limitation)
2. Cross-account resource moves (AWS API limitation)
3. Creating new stacks as part of move operation (would require EnableStackCreation flag - defer to future)
4. Modifying resource properties during move (only logical ID can change)
5. **Legacy import/export flow fallback** (replaced entirely by refactoring API)

## Resolved Questions

### Resource Type Support
**Decision**: Assume CloudFormation Stack Refactoring API supports the same 1304 resource types as CloudFormation resource import (listed in `src/supported_resource_types.rs`).

**Rationale**: 
- Simplest implementation
- AWS API will return clear errors if resource type unsupported
- Can iterate based on real-world feedback if needed
- No need for fallback to legacy import/export flow

### Fallback Strategy
**Decision**: NO fallback to legacy import/export flow. Fully implement via refactoring API only.

**Rationale**:
- Legacy flow has the atomic validation problem we're trying to solve
- Keeping fallback would maintain risk of fucked-up states
- Clear, predictable behavior for users
- Simpler codebase without dual code paths
