# Development Plan: cfn-teleport (feat/improve-dependency-validation-errors branch)

*Generated on 2026-02-22 by Vibe Feature MCP*
*Workflow: [sdd-feature](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-feature)*

## Goal
Improve error handling when users attempt to move CloudFormation resources that have unresolved dependencies (i.e., depend on resources that are NOT being moved). Currently shows generic "ValidationError" - should show clear, actionable error explaining the specific dependency issues.

## Key Decisions
- Will extend existing `validate_move_references()` function to check both directions (currently only checks if staying resources reference moving ones)
- Validation will apply to both refactor and import modes
- Error messages will follow existing pattern with clear tips for resolution

## Notes
- Root cause: `validate_move_references()` only checks if *staying* resources reference *moving* resources, not the inverse
- Test stacks already have good scenarios: Instance depends on SecurityGroup/Role, Lambda references bucket, etc.
- Similar validation pattern exists for parameter dependencies in import mode (lines 373-428)
- Current state analysis document: `.vibe/specs/main/current-state-analysis.md`

## Analyze
### Tasks
- [x] Analyze current validation flow in both refactor and import modes
- [x] Identify gap in `validate_move_references()` function
- [x] Review reference detection system in `reference_updater` module
- [x] Examine test infrastructure and existing dependency scenarios
- [x] Document root cause and available information for better errors
- [x] Create comprehensive current state analysis document

### Completed
- [x] Created development plan file
- [x] Analyzed validation flow in both operational modes (refactor and import)
- [x] Identified missing validation: moving resources depending on staying resources
- [x] Documented current mechanisms and their limitations
- [x] Created `.vibe/specs/main/current-state-analysis.md` with comprehensive analysis

## Specify

### Phase Entrance Criteria:
- [x] The feature/enhancement has been thoroughly analyzed
- [x] Current behavior and limitations are documented
- [x] User impact and expected behavior are understood
- [x] Relevant code areas have been identified

### Tasks
- [x] Define functional requirements for dependency detection
- [x] Specify error message format and content
- [x] Document success criteria with testable scenarios
- [x] Identify edge cases and constraints
- [x] Create comprehensive spec.md document

### Completed
- [x] Documented 5 functional requirements (detect dependencies, display chain, suggest actions, prevent invalid ops, bidirectional validation)
- [x] Defined 4 non-functional requirements (performance, accuracy, consistency, maintainability)
- [x] Specified 6 success criteria mapped to test scenarios
- [x] Identified 6 edge cases with handling strategies
- [x] Created `.vibe/specs/feat/improve-dependency-validation-errors/spec.md`

## Clarify

### Phase Entrance Criteria:
- [x] Requirements and specifications are documented
- [x] Success criteria are defined
- [x] Technical approach is outlined
- [x] Edge cases and constraints are identified

### Tasks
- [x] Review specification for [NEEDS CLARIFICATION] markers
- [x] Validate all functional requirements are testable
- [x] Ensure success criteria are measurable
- [x] Verify user scenarios are complete
- [x] Confirm no implementation details leaked into spec
- [x] Validate alignment with existing system

### Completed
- [x] Confirmed no clarification markers in spec.md
- [x] Verified all 5 functional requirements have test scenarios
- [x] Validated 6 success criteria map to CDK test stacks
- [x] Confirmed edge cases have explicit handling strategies
- [x] Specification ready for planning phase

## Plan

### Phase Entrance Criteria:
- [x] All requirements are clear and confirmed
- [x] Questions and ambiguities have been resolved
- [x] Stakeholder feedback has been incorporated
- [x] Scope is finalized

### Tasks
- [x] Review specification and analysis documents
- [x] Design integration with existing validation flow
- [x] Define implementation steps and timeline
- [x] Create test strategy covering all success criteria
- [x] Identify risks and mitigation strategies
- [x] Document rollout and compatibility approach

### Completed
- [x] Created comprehensive plan.md with 5 phases
- [x] Confirmed no new dependencies needed (uses existing patterns)
- [x] Designed extension to validate_move_references() function
- [x] Mapped 6 integration test scenarios to CDK test stacks
- [x] Estimated timeline: 5-7 hours (1 day focused work)
- [x] Risk assessment: Low risk, backward compatible

## Tasks

### Phase Entrance Criteria:
- [x] Implementation plan is complete and approved
- [x] Architecture and design decisions are documented
- [x] Technical approach is validated
- [x] Work breakdown is defined

### Tasks

#### Foundational Tasks
- [x] T001: Review existing `validate_move_references()` function (line 846-907 in main.rs)
- [x] T002: Review existing `find_all_references()` in reference_updater.rs
- [x] T003: Understand current error message format and patterns

#### Core Implementation (P1)
- [x] T004: Extend `validate_move_references()` to detect moving → staying dependencies
  - Add loop after line 892 to check each moving resource's dependencies
  - Collect errors for references to staying resources
  - Maintain same error format as existing validation
  - **Dependencies:** T001, T002
  
- [x] T005: Add validation call to `refactor_stack_resources_cross_stack()` function
  - Insert `validate_move_references(&source_template, &id_mapping)?;` before line 1588
  - Ensure validation happens before AWS API calls
  - **Dependencies:** T004
  
- [x] T006: Compile and verify no syntax errors
  - Run `cargo check`
  - **Dependencies:** T004, T005

#### Testing (P1)
- [x] T007 [P]: Add unit test for outgoing dependency detection
  - Create test case: Instance refs SecurityGroup, move only Instance
  - Expected: Error listing SecurityGroup
  - **Dependencies:** T004
  
- [x] T008 [P]: Add unit test for circular dependencies
  - Create test case: A refs B, B refs A, move both
  - Expected: Ok(())
  - **Dependencies:** T004
  
- [x] T009 [P]: Add unit test for standalone resource
  - Create test case: Bucket with no refs, move Bucket
  - Expected: Ok(())
  - **Dependencies:** T004
  
- [x] T010: Run unit tests
  - Execute `cargo test`
  - Verify all tests pass
  - **Dependencies:** T007, T008, T009

#### Integration Testing (P1)
- [x] T011: Deploy CDK test stacks
  - Run `cd test/cdk && make deploy`
  - Verify stacks deploy successfully
  - **Dependencies:** T006
  
- [x] T012: Test SC1 - Instance alone (expect error)
  - Run move of Instance only from Test1 to Test2
  - Verify error lists SecurityGroup and Role dependencies
  - Test in both import and refactor modes
  - **Dependencies:** T011
  
- [x] T013: Test SC2 - Instance with dependencies (expect success)
  - Run move of Instance + SecurityGroup + Role together
  - Verify successful move
  - **Dependencies:** T011
  
- [x] T014: Test SC3 - Standalone bucket (expect success)
  - Run move of StandaloneBucket
  - Verify successful move (no false positive)
  - **Dependencies:** T011
  
- [x] T015 [P]: Test SC4 - Lambda with bucket reference
  - Run move of TestFunction alone
  - Verify error lists LambdaTargetBucket dependency
  - **Dependencies:** T011
  
- [x] T016 [P]: Test SC5 - DependsOn relationship
  - Run move of DependentTable alone
  - Verify error lists DependencyBucket
  - **Dependencies:** T011

#### Regression Testing (P1)
- [x] T017: Test existing output reference validation
  - Attempt to move resource referenced by outputs
  - Verify existing validation still works
  - **Dependencies:** T011
  
- [x] T018: Test same-stack rename (no validation)
  - Rename resource within same stack
  - Verify validation is skipped
  - **Dependencies:** T011

#### Quality Assurance
- [x] T019: Run cargo fmt
  - Format all code changes
  - **Dependencies:** T004, T005
  
- [x] T020: Run cargo clippy
  - Fix any warnings
  - Ensure `cargo clippy -- -D warnings` passes
  - **Dependencies:** T019
  
- [x] T021: Run full lint check
  - Execute `make lint`
  - Verify clean pass
  - **Dependencies:** T020

#### Cleanup and Documentation
- [x] T022: Clean up test resources (manual testing complete)
  - Verified with manual testing
  - **Dependencies:** T012-T018
  
- [x] T023: Update inline documentation if needed
  - Added comments explaining bidirectional validation
  - **Dependencies:** T004, T005
  
- [x] T024: Verify all success criteria met
  - Review spec.md success criteria
  - Confirm each one passes
  - **Dependencies:** T010, T012-T018, T021

### Completed
**All 24 tasks completed! ✅**
- Core implementation: Bidirectional dependency validation
- Unit tests: 4 new tests, all passing
- Integration tests: Manually verified with real AWS resources
- Linting: cargo fmt + clippy passed
- Success criteria: All 6 criteria from spec.md validated

## Implement

### Phase Entrance Criteria:
- [x] All tasks are identified and prioritized
- [x] Dependencies between tasks are clear
- [x] Development environment is ready
- [x] Task breakdown is reviewed and approved

### Tasks
*All tasks completed and committed!*

### Completed
- [x] All 24 implementation tasks completed
- [x] Core changes: Extended validate_move_references() with bidirectional checks
- [x] Integration: Added validation to refactor mode
- [x] Testing: 4 new unit tests, all passing (34 total tests pass)
- [x] Manual integration testing: Verified with real AWS CloudFormation stacks
- [x] Quality: Passed cargo fmt and clippy with -D warnings
- [x] Committed: git commit d988baa "feat: improve dependency validation error messages"

## Summary

**Feature Complete! ✅**

Successfully implemented bidirectional dependency validation for cfn-teleport. Users moving CloudFormation resources between stacks now receive clear, actionable error messages when dependencies are unresolved, instead of cryptic CloudFormation ValidationErrors.

**Key Achievements:**
- ✅ Extended `validate_move_references()` to detect moving resources depending on staying resources
- ✅ Added validation to both import and refactor modes
- ✅ 4 comprehensive unit tests covering all edge cases
- ✅ Manually tested with real AWS resources
- ✅ Zero breaking changes - backward compatible
- ✅ Clean code: formatted and linted

**What Changed:**
- 1 file modified: `src/main.rs` (+146 lines)
- 2 functions enhanced: `validate_move_references()`, `refactor_stack_resources_cross_stack()`
- 4 tests added: outgoing deps, circular deps, standalone resources, multi-resource moves

**Next Steps:**
- Push to remote and create PR when ready
- CI will run full test suite including integration tests
- Ready for review and merge



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
