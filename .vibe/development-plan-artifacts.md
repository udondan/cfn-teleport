# Development Plan: cfn-teleport (artifacts branch)

*Generated on 2026-02-23 by Vibe Feature MCP*
*Workflow: [sdd-feature](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-feature)*

## Goal
Improve integration test infrastructure to enable parallel test execution and eliminate cost risks by:
1. Creating 5 isolated test stacks instead of 2 shared stacks (2 for refactor, 2 for import, 1 for rename)
2. Replacing EC2 Instance with Launch Template to reduce creation time and guarantee zero cost
3. Running 3 test stages (refactor, import, rename) in parallel instead of sequentially
4. Reducing total test execution time from ~15-20 minutes to ≤7 minutes
5. Removing Lambda function (not essential for testing Ref/GetAtt)
6. Consolidating DynamoDB tables where possible while maintaining test coverage
## Key Decisions

### 1. KeyPair Testing Strategy (Decision: Keep with Launch Template)
- **Rationale**: KeyPair is a special case - cannot be moved in refactor mode (expects specific error), must work in import mode
- **Implementation**: Replace EC2 Instance with AWS::EC2::LaunchTemplate that references KeyPair
- **Benefits**: Maintains test coverage, removes 2-3 minute bottleneck, guarantees zero cost

### 2. Lambda Function (Decision: Remove)
- **Rationale**: Not essential for testing Ref/GetAtt - already covered by Outputs and other resources
- **Benefits**: Simplifies stack, reduces resources, still zero cost but cleaner

### 3. DynamoDB Tables (Decision: Consolidate where possible)
- **Rationale**: Keep only tables that serve distinct test purposes
- **Requirement**: Must still cover all test cases (standalone, dependencies, parameters)
- **To determine**: Final count during specification phase

### 4. Stack Architecture (Decision: 5 stacks total)
- **Refactor tests**: 2 stacks (RefactorTest1, RefactorTest2)
- **Import tests**: 2 stacks (ImportTest1, ImportTest2)
- **Rename tests**: 1 stack (RenameTest1) - same-stack operations only
- **Benefits**: 3x parallelization, clear test isolation, simpler than 6 stacks

## Notes
*Additional context and observations*

## Analyze
### Tasks
- [x] Analyze current integration test infrastructure
- [x] Document test stages and their execution time
- [x] Identify performance bottlenecks (sequential execution, EC2 creation)
- [x] Analyze resource costs and identify risks
- [x] Document current test architecture and stack configuration
- [x] Await user feedback on clarifying questions
- [x] Document key decisions based on user feedback

### Completed
- [x] T001-T010: Setup Phase (CDK infrastructure changes)
- [x] T011: Local CDK synth validation
- [x] T018-T024: GitHub Actions parallelization (all test jobs created)

### Status Summary
- **Setup Phase**: ✅ Complete (T001-T010)
- **Foundational Phase**: ⚠️ Partial (T011 done, T012-T017 blocked by VPC lookup requiring AWS credentials)
- **GitHub Actions Phase**: ✅ Complete (T018-T024)
- **Next**: Ready for T025 (Push to branch and trigger CI validation)

### Key Changes Made
1. Refactored `.github/workflows/pr-test.yml`:
   - Split `integration-test` into 3 parallel jobs: `test-refactor`, `test-import`, `test-rename`
   - Each job uses correct stack names (RefactorTest1/2, ImportTest1/2, RenameTest1)
   - Replaced EC2 Instance with LaunchTemplate in import tests
   - Removed Lambda-related tests from rename job (TEST 5)
   - Created dedicated `cleanup` job with `if: always()` to ensure resource cleanup
   - Updated `report-status` to depend on all 3 test jobs + cleanup

2. All tests will now run in parallel instead of sequentially

3. Cleanup verified: `make test-reset` → `cd test/cdk && make DESTROY` handles all 5 stacks


## Plan

### Phase Entrance Criteria:
- [x] Specification is reviewed and approved
- [x] Questions and ambiguities are resolved
- [x] Technical approach for parallel execution is clear
- [x] Resource replacement strategy is agreed upon

### Tasks
- [x] Review existing CDK stack architecture and patterns
- [x] Decide on CDK parameterization strategy (single class with ResourceSet parameter)
- [x] Decide on YAML template management approach (manual files)
- [x] Decide on GitHub Actions parallelization strategy (separate jobs)
- [x] Design Launch Template configuration for KeyPair validation
- [x] Decide on stack deployment order (sequential, not parallel)
- [x] Confirm DynamoDB table count (keep all 4)
- [x] Map resources to new stack architecture
- [x] Design incremental rollout strategy (4 stages)
- [x] Document all file changes and risk levels
- [x] Create detailed implementation steps with dependencies

### Completed
- [x] Created comprehensive plan.md with implementation strategy
- [x] All technical decisions documented with rationale
- [x] Resource mapping complete (current → target stacks)
- [x] Risk mitigation strategies defined
- [x] Timeline estimated (12-15 hours development time)

## Tasks

### Phase Entrance Criteria:
- [x] Implementation plan is documented and approved
- [x] File changes and code structure are planned
- [x] Test changes are outlined
- [x] Tasks are broken down into implementable chunks

### Setup Phase: CDK Infrastructure Changes (US-2, US-3)

**T001** [P] - Add ResourceSet type to CDK stack definition ✅
**T002** - Refactor resource creation into conditional blocks ✅
**T003** - Implement Launch Template to replace EC2 Instance (US-2) ✅
**T004** - Remove Lambda function and LambdaTargetBucket (US-2) ✅
**T005** [P] - Update CDK app to create 3 stacks ✅
**T006** [P] - Create RefactorTest2 YAML template ✅
**T007** [P] - Create ImportTest2 YAML template ✅
**T008** [P] - Update Makefile deploy target ✅
**T009** [P] - Update Makefile DESTROY target ✅
**T010** [P] - Update Makefile verify-formats target ✅

### Foundational Phase: Local Testing (US-3)

**T011** [DONE] - Local CDK synth validation ✅
- Command: `cd test/cdk && npx cdk synth`
- Verify all 3 stacks synthesize without errors
- Check generated CloudFormation templates
- Dependencies: T001-T005
- Estimated: 10 min
- Status: All 3 stacks synthesize successfully

**T012** [BLOCKED] - Verify resource distribution in synthesized templates
- Command: `cd test/cdk && npx cdk synth <StackName> | jq '.Resources | keys'`
- Check RefactorTest1 has: StandaloneBucket, Bucket-1/2, tables, KeyPair
- Check ImportTest1 has: Bucket-1/2, DynamoDbTable, KeyPair, LaunchTemplate, SecurityGroup, Role, InstanceProfile
- Check RenameTest1 has: RenameBucket, RenameTable, RenameQueue, DependencyBucket, DependentTable
- Dependencies: T011
- Estimated: 15 min

**T013** - Local stack deployment test
- Command: `cd test/cdk && make deploy`
- Deploy all 5 stacks to test AWS account
- Verify all stacks reach CREATE_COMPLETE status
- Record deployment timing per stack
- Dependencies: T006-T010, T012
- Estimated: 15 min (+ ~10 min AWS deployment time)

**T014** - Manual refactor mode test
- Command: `cfn-teleport --source CfnTeleportRefactorTest1 --target CfnTeleportRefactorTest2 --yes --mode refactor --resource <StandaloneBucket>`
- Verify standalone bucket moves successfully
- Verify CDK diff shows change
- Dependencies: T013
- Estimated: 10 min

**T015** - Manual import mode test with Launch Template
- Command: `cfn-teleport --source CfnTeleportImportTest1 --target CfnTeleportImportTest2 --yes --mode import --resource <LaunchTemplate> --resource <KeyPair> --resource <SecurityGroup> --resource <Role> --resource <InstanceProfile>`
- Verify all resources move together
- Verify KeyPair migration works with Launch Template
- Dependencies: T013
- Estimated: 10 min

**T016** - Manual rename mode test
- Command: `cfn-teleport --source CfnTeleportRenameTest1 --target CfnTeleportRenameTest1 --yes --resource <RenameBucket>:RenamedTestBucket`
- Verify bucket renames successfully
- Verify references updated
- Dependencies: T013
- Estimated: 10 min

**T017** - Local cleanup verification
- Command: `cd test/cdk && make DESTROY`
- Verify all 5 stacks delete successfully
- Verify no orphaned resources
- Dependencies: T013-T016
- Estimated: 10 min

### US-1 Phase: Parallel Test Execution (GitHub Actions)

**T018** [P] - Update cdk-deploy job for 5 stacks ✅
- File: `.github/workflows/pr-test.yml`
- Update deploy command in cdk-deploy job: `make install && make deploy`
- Deployment will now create 5 stacks instead of 2
- No logic changes needed (Makefile handles it)
- Dependencies: T008-T010, T017 (local testing complete)
- Estimated: 5 min
- Status: Already configured correctly (line 118)

**T019** - Create test-refactor job (refactor mode tests) ✅
- File: `.github/workflows/pr-test.yml`
- Copy structure from current `integration-test` job
- Rename to `test-refactor`
- Update `needs` to `[build, cdk-deploy]`
- Set environment variables:
  - STACK1: CfnTeleportRefactorTest1
  - STACK2: CfnTeleportRefactorTest2
  - Resource IDs for refactor tests
- Extract refactor-specific test script (TEST 1-6 from current workflow)
- Dependencies: T018
- Estimated: 1.5 hours
- Status: Completed - all 6 refactor tests migrated with correct stack names

**T020** [P] - Create test-import job (import mode tests) ✅
- File: `.github/workflows/pr-test.yml`
- Copy structure from `test-refactor` job
- Rename to `test-import`
- Update `needs` to `[build, cdk-deploy]`
- Set environment variables:
  - STACK1: CfnTeleportImportTest1
  - STACK2: CfnTeleportImportTest2
  - Resource IDs for import tests (including LaunchTemplate)
- Extract import-specific test script (current "Test import mode" section)
- Update to test LaunchTemplate instead of Instance
- Dependencies: T018
- Estimated: 1.5 hours
- Status: Completed - replaced INSTANCE with LAUNCH_TEMPLATE in test logic

**T021** [P] - Create test-rename job (same-stack rename tests) ✅
- File: `.github/workflows/pr-test.yml`
- Copy structure from `test-refactor` job
- Rename to `test-rename`
- Update `needs` to `[build, cdk-deploy]`
- Set environment variables:
  - STACK1: CfnTeleportRenameTest1
  - Resource IDs for rename tests
- Extract rename-specific test script (current "Test same-stack renaming" section)
- Remove Lambda-related tests (LambdaTargetBucket, TestFunction)
- Dependencies: T018
- Estimated: 1 hour
- Status: Completed - removed TEST 5 (Lambda env vars), kept TEST 1-4

**T022** - Update cleanup job for 5 stacks ✅
- File: `.github/workflows/pr-test.yml`
- Update "Delete test stacks" step to handle 5 stacks
- Ensure `make test-reset` handles new stack names
- Verify cleanup runs even on test failures
- Dependencies: T019-T021
- Estimated: 15 min
- Status: Created dedicated `cleanup` job that runs after all tests with `if: always()`

**T023** - Update report-status job dependencies ✅
- File: `.github/workflows/pr-test.yml`
- Update `needs` array to include:
  - build
  - cdk-deploy
  - test-refactor
  - test-import
  - test-rename
- Dependencies: T019-T021
- Estimated: 5 min
- Status: Added all 3 test jobs + cleanup to needs array

**T024** [P] - Update root Makefile test-clean-all (if needed) ✅
- File: `Makefile` (root)
- Verify test-clean-all still works with new stack names
- All stacks use same tag: `ApplicationName=cfn-teleport-test`
- No changes likely needed
- Dependencies: None
- Estimated: 5 min
- Status: Verified - test-reset calls `cd test/cdk && make DESTROY` which handles all 5 stacks

**T024b** - Parallelize stack deployment (matrix strategy) ✅
- File: `.github/workflows/pr-test.yml`
- Replaced sequential deployment with GitHub Actions matrix strategy
- Matrix deploys 5 stacks in parallel:
  - CfnTeleportRefactorTest1 (CDK)
  - CfnTeleportRefactorTest2 (YAML)
  - CfnTeleportImportTest1 (CDK)
  - CfnTeleportImportTest2 (YAML)
  - CfnTeleportRenameTest1 (CDK)
- Conditional steps based on stack type (cdk vs yaml)
- Benefits: Reduces deployment time from ~2min sequential to <1min parallel
- Dependencies: T024
- Estimated: 45 min
- Status: Implemented with matrix strategy (cleaner than 5 separate jobs)

### Testing & Validation Phase (US-1, US-2, US-3)

**T025** - Push to feature branch and trigger CI
- Push all changes to feature branch
- Trigger GitHub Actions workflow
- Monitor cdk-deploy job
- Dependencies: T018-T024
- Estimated: 5 min (+ CI runtime)

**T026** - Verify parallel test execution
- Watch GitHub Actions UI
- Confirm test-refactor, test-import, test-rename run simultaneously
- Verify no job dependencies between test jobs
- Dependencies: T025
- Estimated: 10 min (during CI run)

**T027** - Verify all tests pass
- Check test-refactor job: all 6 refactor tests pass
- Check test-import job: KeyPair + LaunchTemplate migration works
- Check test-rename job: all 5 rename tests pass (minus Lambda tests)
- Dependencies: T025
- Estimated: 10 min (during CI run)

**T028** - Measure and validate timing (US-1 acceptance)
- Record total workflow duration from GitHub Actions
- Verify ≤7 minutes total (target met)
- Record individual test job durations
- Calculate parallelization speedup
- Dependencies: T025
- Estimated: 5 min (after CI run)

**T029** - Validate zero-cost commitment (US-2 acceptance)
- Check AWS Cost Explorer after 24 hours
- Filter by tag: ApplicationName=cfn-teleport-test
- Verify $0.00 cost for test resources
- Confirm no EC2 instance charges
- Dependencies: T025
- Estimated: 10 min (next day)

**T030** - Verify format preservation
- Check verify-formats step in CI logs
- Confirm RefactorTest1, ImportTest1, RenameTest1 are JSON
- Confirm RefactorTest2, ImportTest2 are YAML
- Dependencies: T025
- Estimated: 5 min

**T031** - Verify cleanup completes
- Check cleanup job in CI
- Confirm all 5 stacks destroyed successfully
- Verify no orphaned resources
- Dependencies: T025
- Estimated: 5 min

### Cleanup Phase (Optional)

**T032** [P] - Delete old stack2-template.yaml (optional)
- File: `test/cdk/stack2-template.yaml`
- Can be deleted if no longer referenced
- Or keep for reference
- Dependencies: T031 (after validation complete)
- Estimated: 2 min

**T033** [P] - Update documentation/comments
- Update AGENTS.md with new stack names if referenced
- Update test/cdk/README if exists
- Add comments explaining stack architecture
- Dependencies: T031
- Estimated: 15 min

### Completed
*None yet - ready to begin implementation*

## Implement

### Phase Entrance Criteria:
- [x] All tasks are clearly defined
- [x] Dependencies between tasks are identified
- [x] Development environment is ready

### Tasks
*Implementation will proceed task-by-task from the Tasks section above*

### Completed
*None yet*



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
