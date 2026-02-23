# Implementation Plan: Cross-Stack Resource Move with CFN Refactoring

**Date**: 2026-02-21  
**Phase**: Planning  
**Feature**: Replace legacy import/export flow with CFN Refactoring API for cross-stack moves

## Integration Context

### Existing System Architecture

From analysis of current codebase (`src/main.rs`):

**Current Flow Decision Point** (lines 284-298):
```rust
// Same-stack rename: Use CloudFormation Stack Refactoring API
if source_stack == target_stack {
    return refactor_stack_resources(...).await;
}

// Cross-stack move: Use import/export flow
let resource_ids_to_remove: Vec<_> = new_logical_ids_map.keys().cloned().collect();
// ... legacy flow continues
```

**Existing Modules**:
1. **`refactor_stack_resources()`** (lines 832-989): Handles same-stack renames using CFN refactoring
2. **`reference_updater` module**: Updates all template references (Ref, Fn::GetAtt, Fn::Sub, etc.)
3. **`validate_template()`**: Validates CloudFormation templates before operations
4. **User interaction functions**: Prompts, confirmations, spinners
5. **Template manipulation functions**: `get_template()`, `add_resources()`, `remove_resources()`, etc.

### Technology Stack
- **Language**: Rust 2021
- **AWS SDK**: `aws-sdk-cloudformation` with async/await (tokio)
- **Types Used**: `ResourceLocation`, `ResourceMapping`, `StackDefinition` (already imported)
- **Error Handling**: `Result<T, Box<dyn Error>>` for flexibility
- **User Interaction**: `dialoguer` crate for prompts, custom `Spin` for progress

## High-Level Design

### Strategy: Extend Existing `refactor_stack_resources()` Function

**Option A: Single Function for Both Same-Stack and Cross-Stack** (Recommended)
- Extend `refactor_stack_resources()` to handle both cases
- Pros: Code reuse, consistent behavior, single path through CFN API
- Cons: Function becomes more complex

**Option B: Separate Function for Cross-Stack**
- Create `refactor_cross_stack_resources()`
- Pros: Clear separation, simpler individual functions
- Cons: Code duplication for validation/execution logic

**Decision**: **Option A** - The core logic (create refactor, validate, execute, wait) is identical. Only template preparation differs.

### Key Differences: Same-Stack vs Cross-Stack

| Aspect | Same-Stack Rename | Cross-Stack Move |
|--------|-------------------|------------------|
| **StackDefinitions** | 1 (single stack with renamed resources) | 2 (source with removed, target with added) |
| **ResourceMappings** | Source & destination have same stack_name | Source & destination have different stack_names |
| **Template Updates** | Rename resources in single template | Remove from source, add to target |
| **DeletionPolicy** | Not needed (rename in place) | Still needed? (TBD) |

### Integration Points

1. **Entry Point** (lines 189-297 in `main.rs`):
   - **Current**: `if source_stack == target_stack` branches to refactor vs. legacy
   - **Change**: Remove legacy branch entirely, always use refactor

2. **Template Preparation**:
   - **Reuse**: `get_template()`, `reference_updater::update_template_references()`
   - **Modify**: Template manipulation for cross-stack (remove from source, add to target)

3. **Validation**:
   - **Reuse**: `validate_template()` - call for both source and target templates
   - **Keep**: Pre-move validation for dangling references (lines 282-287)

4. **User Interaction**:
   - **Reuse**: Existing prompts and confirmation flow (lines 194-280)
   - **Minimal changes**: Update messaging to reflect cross-stack move

## Implementation Phases

### Phase 0: Research & Unknowns

#### Unknown 1: DeletionPolicy Handling
**Question**: Does CFN refactoring API require `DeletionPolicy: Retain` on source resources, or does it handle resource retention automatically?

**Current Legacy Flow**:
1. Add `DeletionPolicy: Retain` to source resources
2. Update source stack (resources now retained)
3. Remove resources from source template
4. Update source stack again (resources deleted from stack but retained in AWS)

**Hypothesis**: Refactoring API might handle this automatically since it knows resources are moving.

**Research Task**: Test with simple resource (S3 bucket) to see if DeletionPolicy is needed.

#### Unknown 2: Template Structure for Cross-Stack Refactor
**Question**: What should source and target templates contain for refactoring API?

**Options**:
- **A**: Source has resources removed, target has resources added (mirrors final state)
- **B**: Source has resources with retain policy, target has resources added (mirrors legacy)
- **C**: Something else?

**Hypothesis**: Based on API name "Stack Refactoring", it likely expects final desired state (Option A).

**Research Task**: Review AWS SDK examples or test with actual API call.

### Phase 1: Core Refactoring Function Extension

**Task 1.1**: Extend `refactor_stack_resources()` signature
- Add parameter to indicate cross-stack vs same-stack
- Or detect from `source_stack != target_stack` in ResourceMappings

**Task 1.2**: Implement cross-stack template preparation
- For source: Remove selected resources from template
- For target: Add selected resources to template  
- Apply reference updates to both templates
- Validate both templates

**Task 1.3**: Build dual StackDefinitions
- Create `Vec<StackDefinition>` with both source and target
- Each with appropriate template_body

**Task 1.4**: Build cross-stack ResourceMappings
- Source ResourceLocation with source stack_name
- Destination ResourceLocation with target stack_name

**Task 1.5**: Update spinner messaging
- Change from "Renaming N resources in stack X" to "Moving N resources from X to Y"

### Phase 2: Entry Point Integration

**Task 2.1**: Remove legacy flow code
- Delete lines 300-379 (cross-stack import/export flow)
- Remove helper functions if no longer used:
  - `retain_resources()`
  - `remove_resources()` (if only used by legacy flow)
  - `add_resources()` (if only used by legacy flow)
  - `create_changeset()`, `wait_for_changeset_created()`, `execute_changeset()` (if only for import)

**Task 2.2**: Update branching logic
- Change `if source_stack == target_stack` to pass flag or derive from context
- Single call to refactored function for both cases

**Task 2.3**: Update user messaging
- Lines 194-280: Update prompts to differentiate rename vs move
- "The following resources will be renamed" vs "The following resources will be moved"

### Phase 3: Template Manipulation Functions

**Task 3.1**: Audit template helper functions
- Determine which are reusable (keep)
- Determine which are legacy-only (delete)
- Create new helpers if needed for cross-stack template prep

**Task 3.2**: Implement/reuse template preparation for cross-stack
- **For source template**:
  - Remove selected resources from Resources section
  - Keep everything else unchanged
  - Apply reference updates (in case remaining resources reference each other)
  
- **For target template**:
  - Add selected resources to Resources section
  - Apply reference updates for new logical IDs
  - Ensure no conflicts with existing resources

### Phase 4: Error Handling & User Experience

**Task 4.1**: Handle refactoring API errors
- Parse CloudFormation refactor status reasons
- Provide clear error messages for common scenarios:
  - Resource name conflicts in target
  - Unsupported resource types
  - Permission issues
  - Validation failures

**Task 4.2**: Progress indication
- Reuse existing spinner pattern
- Update messages for cross-stack context

**Task 4.3**: Success confirmation
- Update output to show resources moved (not renamed)
- Display both source and target stack names

### Phase 5: Testing Strategy

**Task 5.1**: Update integration tests
- Modify `test/cdk/` stacks if needed
- Test cross-stack moves with existing test resources (S3, DynamoDB, EC2)

**Task 5.2**: Add specific test scenarios
- Move single resource
- Move multiple resources
- Move with logical ID change
- Error scenario: target stack has conflicting name
- Error scenario: references between staying/moving resources

**Task 5.3**: Test resource types
- Verify common types work (S3 bucket, DynamoDB table, IAM role)
- Document any types that fail

## Dependencies & Order

### Critical Path
1. **Research** (Phase 0) → MUST complete before Phase 1
   - DeletionPolicy handling determines template structure
   - Template structure determines implementation approach

2. **Core Function** (Phase 1) → Foundation for everything else
   - Must work before removing legacy code

3. **Integration** (Phase 2) → Depends on Phase 1
   - Can't remove legacy until new code works

4. **Template Functions** (Phase 3) → Can happen in parallel with Phase 1
   - Template prep logic needed by Phase 1, but can be developed alongside

5. **Error Handling** (Phase 4) → After Phase 1-3
   - Need working implementation to test error scenarios

6. **Testing** (Phase 5) → Continuous throughout
   - Start with Phase 1, expand as features added

### Parallel Work Opportunities
- Phase 3 (template functions) can start early
- Phase 4 (error messages) can be drafted while implementing Phase 1
- Phase 5 (test planning) can happen upfront

## Code Deletion Estimate

**Functions to likely DELETE** (legacy import/export):
- `create_changeset()` - Creates import changeset
- `wait_for_changeset_created()` - Waits for changeset
- `execute_changeset()` - Executes changeset  
- `retain_resources()` - Adds DeletionPolicy: Retain
- `remove_resources()` - Removes resources from template
- `add_resources()` - Adds resources to template (unless reusable)

**Estimated LOC to remove**: ~200-300 lines (lines 300-600 in current `main.rs`)

## Risks & Mitigations

### Risk 1: DeletionPolicy Still Needed
**Impact**: If refactoring API doesn't handle retention, we need to add DeletionPolicy step
**Mitigation**: Phase 0 research will determine this early

### Risk 2: Unsupported Resource Types
**Impact**: Some resource types might not work with refactoring API
**Probability**: Low (we assume same as import)
**Mitigation**: Clear error messages, document unsupported types as discovered

### Risk 3: Reference Update Complexity
**Impact**: Cross-stack references might be more complex than same-stack
**Mitigation**: Reuse existing `reference_updater` module - already handles complex cases

### Risk 4: AWS API Rate Limits / Timeouts
**Impact**: Multi-stack refactoring might take longer than single-stack
**Mitigation**: Reuse existing wait/poll logic with appropriate timeouts

## Success Criteria

1. ✅ Cross-stack resource moves complete successfully via refactoring API
2. ✅ Legacy import/export code completely removed
3. ✅ No more risk of resources orphaned between stacks
4. ✅ Same user experience as same-stack rename (prompts, confirmations, output)
5. ✅ Integration tests pass for cross-stack moves
6. ✅ Code is simpler (single path instead of dual paths)

## Next Steps

1. Complete Phase 0 research (DeletionPolicy and template structure)
2. Create detailed task breakdown for Phase 1 (core function)
3. Prototype minimal cross-stack move with simple resource type
4. Iterate based on findings
