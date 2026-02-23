# Development Plan: cfn-teleport (cfn-refactor branch)

*Generated on 2026-02-21 by Vibe Feature MCP*
*Workflow: [sdd-feature](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-feature)*

## Goal
Replace the current destructive cross-stack resource move flow (remove from source, then import to target) with CloudFormation Stack Refactoring API to ensure atomic validation and execution, preventing fucked-up states when imports fail.

## Key Decisions
1. **Use CFN Refactoring API**: Leverage the same API used for same-stack renames, but with cross-stack ResourceMappings
2. **Atomic Operations**: CloudFormation validates BEFORE modifying any stack (unlike current flow that removes from source first)
3. **CONFIRMED**: AWS API explicitly supports multiple stacks - "Creates a refactor across multiple stacks"
4. **Resource Type Support**: ~~Assume refactoring API supports same 1304 types as import API~~ **INCORRECT** - Refactoring API is MORE restrictive
5. **NO Fallback**: Completely replace legacy import/export flow with refactoring API - no dual code paths

### Critical Discovery (T008 Testing)
**Refactoring API has MORE restrictions than import API:**
- Example: `AWS::EC2::KeyPair` is unsupported because it doesn't allow tag updates after creation
- CloudFormation uses tags internally to track resource ownership during refactoring
- Error message: "Stack Refactor does not support AWS::EC2::KeyPair because the resource type does not allow tag updates after creation"
- **Impact**: Need to add pre-validation and clear error messages for unsupported types
- **Question**: Should we maintain a list of known-unsupported types, or rely on API error messages?

## Notes
- Current same-stack rename using CFN refactoring works well (validated before execution)
- Cross-stack move currently uses legacy import/export flow that can leave stacks in bad state
- ResourceLocation supports different stack names in source/destination - this is the key
- **Implementation Strategy**: Extend existing `refactor_stack_resources()` for cross-stack, delete ~200-300 lines of legacy code
- **Research Completed**: DeletionPolicy NOT needed; 2 StackDefinitions showing final state required

### Code Deletion Summary (Phase 2 Complete)
**Deleted ~305 lines of legacy code:**
- Legacy cross-stack import/export flow (~80 lines, lines 302-381)
- 9 unused helper functions (~225 lines):
  - `retain_resources()`, `update_stack()`, `get_stack_status()`
  - `wait_for_stack_update_completion()`, `get_resource_identifier_mapping()`
  - `create_changeset()`, `execute_changeset()`, `get_changeset_status()`, `wait_for_changeset_created()`
- Unused import: `uuid::Uuid`

**Kept and reused:**
- `remove_resources()` - used by cross-stack template preparation
- `add_resources()` - used by cross-stack template preparation
- `set_default_deletion_policy()` - used by add_resources()
- `reference_updater` module - used by both same-stack and cross-stack
- `validate_template()` - used for template validation

## Analyze
### Tasks
- [x] Analyze current implementation of same-stack rename with CFN refactoring
- [x] Analyze current cross-stack move flow and identify failure points
- [x] Document AWS SDK types for refactoring (ResourceLocation, ResourceMapping, StackDefinition)
- [x] Research AWS documentation for cross-stack refactoring API support
- [x] Verify StackDefinition supports multiple stacks in single refactor operation

### Completed
- [x] Created development plan file
- [x] Created current-state-analysis.md documenting both flows
- [x] Confirmed AWS API explicitly supports cross-stack refactoring with multiple StackDefinitions
- [x] Documented API references and key findings in analysis document

## Specify

### Phase Entrance Criteria:
- [x] Current implementation approach has been analyzed and documented
- [x] CFN refactoring capabilities have been researched
- [x] Validation requirements for cross-stack resource moves are understood
- [x] Differences between rename and move operations are clear

### Tasks
- [x] Define functional requirements for cross-stack move with CFN refactoring
- [x] Define non-functional requirements (reliability, validation time, error clarity)
- [x] Document user workflows (primary move workflow, error scenarios)
- [x] Identify technical constraints from AWS API
- [x] Define success metrics
- [x] Resolve resource type support question (assume same as import)
- [x] Decide on fallback strategy (no fallback, refactoring API only)

### Completed
- [x] Created spec.md with comprehensive feature specification
- [x] Documented atomic operation requirements
- [x] Resolved all clarifications - no fallback needed, assume same resource types
- [x] Ready to proceed to Clarify phase

## Clarify

### Phase Entrance Criteria:
- [x] Functional requirements are specified
- [x] Technical approach using CFN refactoring is defined
- [x] Success criteria and constraints are documented
- [x] API/CLI interface changes are outlined

### Tasks
- [x] Review specification for completeness (no [NEEDS CLARIFICATION] markers found)
- [x] Verify all functional requirements are testable
- [x] Validate success criteria are measurable
- [x] Confirm no implementation details in spec
- [x] Verify alignment with existing system (reference_updater, validation modules)

### Completed
- [x] Specification review complete - all requirements clear and testable
- [x] No remaining clarifications needed
- [x] Ready to proceed to Plan phase

## Plan

### Phase Entrance Criteria:
- [x] All ambiguities in requirements have been resolved
- [x] Edge cases and error scenarios are identified
- [x] Testing strategy is clear
- [x] Implementation scope is well-defined

### Tasks
- [x] Analyze existing architecture and integration points
- [x] Design high-level implementation strategy (extend refactor_stack_resources)
- [x] Identify code to delete (~200-300 lines of legacy import/export)
- [x] Plan template manipulation approach (remove from source, add to target)
- [x] Define research tasks (DeletionPolicy handling, template structure)
- [x] Break down implementation into 5 phases (Research, Core, Integration, Templates, Testing)
- [x] Document dependencies and critical path
- [x] Identify risks and mitigations

### Completed
- [x] Created plan.md with complete implementation strategy
- [x] Identified 2 research unknowns to resolve in Phase 0
- [x] Mapped all integration points with existing code
- [x] Ready to proceed to Tasks phase (detailed task breakdown)

## Tasks

### Phase Entrance Criteria:
- [x] High-level architecture/approach is documented
- [x] Implementation steps are identified
- [x] Dependencies between components are clear
- [x] Testing approach is defined

### Phase 0: Research & Discovery
**Goal**: Resolve unknowns about CFN refactoring API behavior

- [x] **T001** [P] Research: Test if DeletionPolicy:Retain needed for cross-stack refactor
  - Create minimal test with S3 bucket
  - Try refactor without DeletionPolicy, observe behavior
  - Document findings in plan.md
  - **Dependencies**: None
  - **Result**: DeletionPolicy NOT needed - refactoring API handles retention automatically
  
- [x] **T002** [P] Research: Determine template structure for cross-stack refactor
  - Review AWS SDK CloudFormation examples
  - Test with actual API call (source with removed, target with added)
  - Document expected template format
  - **Dependencies**: None
  - **Result**: 2 StackDefinitions showing final state (source with removed, target with added)

### Phase 1: Core Refactoring Function Extension
**Goal**: Extend `refactor_stack_resources()` to handle cross-stack moves

- [x] **T003** Extend `refactor_stack_resources()` function signature
  - Add parameters for target stack and target template
  - Or detect cross-stack from ResourceMappings
  - Update function documentation
  - **Dependencies**: T001, T002 (research complete)
  - **Completed**: Added `target_stack_name: Option<&str>` and `target_template: Option<serde_json::Value>` parameters
  - Function now detects cross-stack vs same-stack based on presence of target_stack_name
  - Updated function documentation with detailed parameter descriptions

- [x] **T004** Implement cross-stack template preparation logic
  - For source: remove selected resources, apply reference updates
  - For target: add selected resources, apply reference updates  
  - Validate both templates
  - **Dependencies**: T003
  - **Completed**: Implemented template preparation with branching logic for cross-stack vs same-stack
  - Uses existing `remove_resources()` and `add_resources()` functions
  - Applies `reference_updater::update_template_references()` to both source and target
  - Validates both templates before creating refactor

- [x] **T005** Build dual StackDefinitions for cross-stack
  - Create Vec<StackDefinition> with both source and target
  - Each with appropriate template_body and stack_name
  - **Dependencies**: T004
  - **Completed**: Created `stack_definitions` vector that conditionally includes target stack definition
  - Source stack definition always included
  - Target stack definition added only for cross-stack moves

- [x] **T006** Build cross-stack ResourceMappings
  - Source ResourceLocation with source stack_name
  - Destination ResourceLocation with target stack_name  
  - **Dependencies**: T003
  - **Completed**: Updated ResourceMapping builder to use `source_stack_name` for source and `target_stack_for_mapping` for destination
  - `target_stack_for_mapping` is `target_stack_name.unwrap_or(source_stack_name)` to handle both cases

- [x] **T007** Update refactor execution logic for multiple stacks
  - Handle validation/execution for 2 stacks instead of 1
  - Update spinner messaging ("Moving N resources from X to Y")
  - **Dependencies**: T005, T006
  - **Completed**: Updated spinner message to differentiate "Moving" vs "Renaming"
  - Iterates through stack_definitions vector to add all to refactor request
  - Success message also differentiates between move and rename operations

- [x] **T008** Test core refactoring function with simple resource
  - Use test stacks (CfnTeleportTest1, CfnTeleportTest2)
  - Move S3 bucket between stacks
  - Verify atomic behavior (success or rollback)
  - **Dependencies**: T007
  - **Completed**: Successfully tested cross-stack moves
  - **Test results**:
    - ✅ Single S3 bucket move (Bucket182C536A1): SUCCESS
    - ✅ Multiple S3 bucket moves (Bucket21D68F7E8): SUCCESS
    - ✅ Dangling reference detection working correctly
    - ✅ Atomic rollback confirmed (failed moves don't corrupt stacks)
    - ❌ EC2::KeyPair unsupported (tag update restriction)
    - ❌ Some IAM/EC2 resources fail validation (property mismatch - needs investigation)

- [x] **T008a** [NEW] Handle unsupported resource types
  - Improve error message when refactoring API rejects resource type
  - Add pre-validation check before attempting refactor (optional)
  - Document known unsupported types (EC2::KeyPair, others TBD)
  - **Dependencies**: T008
  - **Decision**: Rely on API error messages (already clear and descriptive)
  - **Completed**: 
    - Created comprehensive `research-findings.md` with sources documenting:
      - Why RefactorStacks API is MORE restrictive than import/export
      - Root cause: Tag-based ownership tracking requires tag updates
      - EC2::KeyPair limitation: Tags property requires "Replacement"
      - Trade-off analysis: atomicity vs. resource type coverage
    - Created `api-limitations.md` documenting test results
    - No code changes needed - API errors are user-friendly
    - Pre-validation rejected (adds complexity, AWS API is source of truth)

### Phase 2: Entry Point Integration
**Goal**: Remove legacy flow and integrate new refactoring approach

- [x] **T009** Update main function branching logic
  - Remove `if source_stack == target_stack` distinction
  - Call refactor function for both same-stack and cross-stack
  - **Dependencies**: T008 (core function tested)
  - **Completed**: Replaced legacy cross-stack flow with refactor_stack_resources() call
  - Both same-stack and cross-stack now use unified refactoring API
  - Cross-stack fetches target template and passes to refactor function

- [x] **T010** Delete legacy import/export flow code (lines 300-379)
  - Remove cross-stack import changeset creation
  - Remove DeletionPolicy: Retain logic
  - Remove template_retained, template_removed steps
  - **Dependencies**: T009
  - **Completed**: Deleted entire legacy cross-stack flow (~80 lines of code)
  - Replaced with simple call to refactor_stack_resources() with target stack/template

- [x] **T011** Delete unused helper functions
  - Audit: `create_changeset()`, `wait_for_changeset_created()`, `execute_changeset()`
  - Audit: `retain_resources()`, `remove_resources()`, `add_resources()`
  - Delete if only used by legacy flow
  - **Dependencies**: T010
  - **Completed**: Deleted 9 unused functions (~225 lines):
    - `retain_resources()` - DeletionPolicy:Retain logic no longer needed
    - `update_stack()` - replaced by refactoring API
    - `get_stack_status()` - only used by wait_for_stack_update_completion
    - `wait_for_stack_update_completion()` - refactoring API handles waiting
    - `get_resource_identifier_mapping()` - only used by create_changeset
    - `create_changeset()` - import changeset no longer used
    - `execute_changeset()` - no longer needed
    - `get_changeset_status()` - only used by wait_for_changeset_created
    - `wait_for_changeset_created()` - no longer needed
  - Removed unused `uuid::Uuid` import
  - Kept `remove_resources()` and `add_resources()` - still used by refactoring function
  - Kept `set_default_deletion_policy()` - still used by add_resources()

- [ ] **T012** Update user confirmation prompts (lines 194-280)
  - Differentiate messaging: "rename" vs "move"
  - Show both source and target stack for cross-stack
  - **Dependencies**: T009

### Phase 3: Template Manipulation & References
**Goal**: Ensure template manipulation works correctly for cross-stack

- [ ] **T013** [P] Audit existing template helper functions
  - Identify reusable functions (get_template, validate_template)
  - Identify legacy-only functions
  - **Dependencies**: None (can start early)

- [ ] **T014** Implement/refactor source template manipulation
  - Remove selected resources from source Resources section
  - Keep all other resources and template sections
  - Apply reference updates if needed
  - **Dependencies**: T013

- [ ] **T015** Implement/refactor target template manipulation
  - Add selected resources to target Resources section
  - Ensure no logical ID conflicts
  - Apply reference updates for new IDs
  - **Dependencies**: T013

- [ ] **T016** Test reference updates for cross-stack moves
  - Test Ref, Fn::GetAtt, Fn::Sub with moved resources
  - Verify references within moved resources updated correctly
  - **Dependencies**: T014, T015

### Phase 4: Error Handling & User Experience
**Goal**: Provide clear error messages and good UX

- [ ] **T017** Implement refactoring API error handling
  - Parse CloudFormation refactor status reasons
  - Map to user-friendly error messages
  - **Dependencies**: T008 (need to see real errors)

- [ ] **T018** Handle common error scenarios
  - Resource name conflicts in target stack
  - Unsupported resource types (e.g., EC2::KeyPair - doesn't allow tag updates)
  - Permission errors
  - Validation failures (dangling references)
  - **Dependencies**: T017

- [ ] **T019** Update progress indication
  - Spinner messages for validation phase
  - Spinner messages for execution phase
  - Success confirmation with moved resources listed
  - **Dependencies**: T007

- [ ] **T020** Update CLI help and documentation
  - Update `--help` text if needed
  - Update AGENTS.md with new flow
  - **Dependencies**: T011 (after legacy removed)

### Phase 5: Testing & Validation
**Goal**: Comprehensive testing of cross-stack moves

- [ ] **T021** Update CDK integration test stacks
  - Ensure test stacks (CfnTeleportTest1, CfnTeleportTest2) have appropriate resources
  - Add resources of different types (S3, DynamoDB, EC2, IAM)
  - **Dependencies**: None (can start early)

- [ ] **T022** Test single resource move
  - Move S3 bucket from Test1 to Test2
  - Verify resource exists in Test2, removed from Test1
  - **Dependencies**: T008, T021

- [ ] **T023** Test multiple resource move
  - Move S3 bucket + DynamoDB table together
  - Verify both resources moved atomically
  - **Dependencies**: T022

- [ ] **T024** Test move with logical ID change
  - Move resource and rename it simultaneously
  - Verify new logical ID in target stack
  - **Dependencies**: T022

- [ ] **T025** Test error scenario: conflicting names
  - Attempt move when target has resource with same logical ID
  - Verify validation fails, neither stack modified
  - **Dependencies**: T018

- [ ] **T026** Test error scenario: dangling references
  - Attempt move when source has resources referencing moved resource
  - Verify validation fails or warning shown
  - **Dependencies**: T018

- [ ] **T027** Test different resource types
  - EC2 instance, security group
  - IAM role, policy
  - DynamoDB table
  - Document any unsupported types discovered
  - **Dependencies**: T023

- [ ] **T028** Run full integration test suite
  - `make test` with all CDK stacks
  - Verify no regressions in existing functionality
  - **Dependencies**: T027

### Rollout & Cleanup
**Goal**: Final validation and documentation

- [ ] **T029** Code review and refactoring
  - Review code quality, eliminate duplication
  - Ensure consistent error handling
  - Update comments and documentation
  - **Dependencies**: T028

- [ ] **T030** Final validation
  - Test with real-world scenarios
  - Verify CLI UX is clear and consistent
  - Check for edge cases
  - **Dependencies**: T029

### Summary
- **Total Tasks**: 30
- **Parallelizable**: T001, T002 (research), T013 (audit), T021 (test setup)
- **Critical Path**: T001/T002 → T003 → T004 → T005/T006 → T007 → T008 → T009 → T010 → ...
- **Estimated Effort**: ~2-3 days for experienced Rust developer

### Completed
*Tasks will be marked as completed during implementation*

## Implement

### Phase Entrance Criteria:
- [x] All implementation tasks are broken down and prioritized
- [x] Each task has clear acceptance criteria
- [x] Development environment is ready
- [x] Tests are defined for validation

### Current Focus
**Phase 1-2 Complete!** Core refactoring function extended and legacy code removed.

**Next Steps:**
- **T008**: Test core refactoring function with simple resource (S3 bucket cross-stack move)
- **T012**: Update user confirmation prompts to differentiate "rename" vs "move" messaging
- **Phase 3**: Template manipulation & reference updates (T013-T016)
- **Phase 4**: Error handling & UX improvements (T017-T020)
- **Phase 5**: Comprehensive testing (T021-T030)

### Tasks
*Tasks are tracked in the Tasks section above. Mark with [x] as completed.*

### Completed
- [x] Phase 0: Research & Discovery (T001-T002)
  - DeletionPolicy NOT needed for refactoring API
  - 2 StackDefinitions showing final desired state required for cross-stack
- [x] Phase 1: Core Refactoring Function Extension (T003-T007)
  - Extended refactor_stack_resources() to handle both same-stack and cross-stack
  - Implemented cross-stack template preparation (remove from source, add to target)
  - Built dual StackDefinitions and cross-stack ResourceMappings
  - Updated spinner messaging and success messages
- [x] Phase 2: Entry Point Integration (T009-T011)
  - Unified main function logic to use refactoring API for all cases
  - Deleted ~305 lines of legacy import/export flow code
  - Removed 9 unused helper functions
  - Code compiles and passes clippy with -D warnings
- [~] **T008**: Initial testing revealed API limitation (AWS::EC2::KeyPair unsupported)
  - Error message from API is clear and actionable
  - Need AWS credentials to complete full test with supported resource types



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
