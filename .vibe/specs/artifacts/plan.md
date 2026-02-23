# Implementation Plan: Parallel Integration Test Infrastructure

**Version**: 1.0  
**Date**: 2026-02-23  
**Status**: Draft

---

## Overview

This plan outlines the implementation strategy for transforming the cfn-teleport integration test infrastructure from a sequential 2-stack design to a parallel 5-stack design. The implementation will be done incrementally to minimize risk and enable testing at each stage.

---

## Architecture Integration

### Current Architecture
- **CDK Stack Definition**: `test/cdk/lib/index.ts` - Single `TestStack` class with conditional resource creation
- **CDK App**: `test/cdk/bin/app.ts` - Creates single CDK stack (Test1)
- **YAML Template**: `test/cdk/stack2-template.yaml` - Minimal CloudFormation template for Test2
- **Deployment**: `test/cdk/Makefile` - Deploys Test1 via CDK, Test2 via AWS CLI
- **CI Workflow**: `.github/workflows/pr-test.yml` - Sequential test stages in single job

### Target Architecture
- **Multiple Stack Types**: Refactor, Import, Rename stacks with different resource sets
- **CDK Stack Variants**: Same `TestStack` class with new parameters to control resource types
- **Multiple YAML Templates**: Separate YAML templates for each target stack
- **Parallel Deployment**: Deploy all 5 stacks (can be sequential deployment, parallel tests)
- **Parallel CI Jobs**: 3 independent test jobs running concurrently

### Integration Points
1. **CDK Stack Class** (`lib/index.ts`): Add parameters to control which resource sets to include
2. **CDK App** (`bin/app.ts`): Instantiate 3 different source stacks (Refactor, Import, Rename)
3. **YAML Templates**: Create 2 new YAML templates (RefactorTest2, ImportTest2)
4. **Makefile**: Update deployment to handle 5 stacks
5. **GitHub Actions**: Restructure to parallel jobs with stack isolation

---

## Technology Stack

### Existing Technologies (No Changes)
- **CDK**: AWS CDK (TypeScript) - v2.x
- **CloudFormation**: AWS CLI for YAML template deployment
- **Node.js**: v24 (for CDK)
- **GitHub Actions**: ubuntu-latest runners
- **Shell Scripting**: Bash for test orchestration
- **JSON/YAML Processing**: jq for dynamic resource ID discovery

### No New Dependencies Required
All implementation uses existing tools and libraries.

---

## Phase 0: Research & Decisions

### Decision 1: CDK Stack Parameterization Strategy

**Question**: How should we control which resources are included in each stack?

**Options**:
1. **Single Stack Class with Resource Set Parameter**: Pass enum/string to control resource groups
2. **Separate Stack Classes**: RefactorStack, ImportStack, RenameStack
3. **Composition Pattern**: Base stack + mixin functions for resource groups

**Decision**: **Option 1 - Single Stack Class with Resource Set Parameter**

**Rationale**:
- Minimal code changes to existing `TestStack` class
- Current pattern already uses `resources: boolean` parameter
- Easy to maintain - single source of truth for resource definitions
- Clear mapping: parameter value → resource set

**Implementation**:
```typescript
type ResourceSet = 'refactor' | 'import' | 'rename' | 'none';

type TestStackProps = StackProps & {
  resourceSet: ResourceSet;
};
```

---

### Decision 2: YAML Template Management

**Question**: How should we create and maintain multiple YAML templates?

**Options**:
1. **Manual YAML Files**: Create 2 new YAML files by hand
2. **CDK Synth**: Generate YAML from empty CDK stacks
3. **Template from String**: Single template file with placeholders

**Decision**: **Option 1 - Manual YAML Files (following existing pattern)**

**Rationale**:
- Current stack2-template.yaml is manually maintained
- YAML templates are minimal (just parameter + placeholder resource)
- Manual maintenance is simple and explicit
- Avoids CDK synth complexity for minimal templates

**Files to Create**:
- `test/cdk/refactor-test2-template.yaml`
- `test/cdk/import-test2-template.yaml`

---

### Decision 3: GitHub Actions Parallelization Strategy

**Question**: How should we structure parallel test jobs?

**Options**:
1. **Matrix Strategy**: Single test job with matrix for test types
2. **Separate Jobs**: 3 explicit jobs (test-refactor, test-import, test-rename)
3. **Reusable Workflow**: Shared workflow called 3 times

**Decision**: **Option 2 - Separate Jobs**

**Rationale**:
- Clearest in GitHub Actions UI (3 distinct job names)
- Easiest to debug (logs separated by job)
- Most explicit (no hidden matrix magic)
- Allows per-job customization if needed

**Job Structure**:
```yaml
jobs:
  test-refactor:
    needs: [build, cdk-deploy]
    env:
      STACK1: CfnTeleportRefactorTest1
      STACK2: CfnTeleportRefactorTest2
  
  test-import:
    needs: [build, cdk-deploy]
    env:
      STACK1: CfnTeleportImportTest1
      STACK2: CfnTeleportImportTest2
  
  test-rename:
    needs: [build, cdk-deploy]
    env:
      STACK1: CfnTeleportRenameTest1
```

---

### Decision 4: Launch Template Configuration

**Question**: What Launch Template configuration validates KeyPair relationships?

**Research Needed**: 
- Minimal Launch Template properties required
- Does Launch Template support KeyPairName property?
- Does Launch Template reference SecurityGroup and InstanceProfile?

**Decision**: **Use Launch Template with UserData and Full Dependencies**

**Configuration**:
```typescript
new aws_ec2.CfnLaunchTemplate(this, 'LaunchTemplate', {
  launchTemplateName: 'cfn-teleport-test-template',
  launchTemplateData: {
    imageId: machineImage.getImage(this).imageId,
    instanceType: 't2.micro',
    keyName: keyPair.keyPairName,
    securityGroupIds: [securityGroup.securityGroupId],
    iamInstanceProfile: {
      arn: instanceProfile.attrArn,
    },
  },
});
```

**Rationale**:
- Validates KeyPair, SecurityGroup, and InstanceProfile references
- Never launches actual instances (zero cost)
- Creates in <10 seconds
- Sufficient for testing cfn-teleport resource migration

---

### Decision 5: Stack Deployment Order

**Question**: Should stacks be deployed in parallel or sequentially?

**Options**:
1. **Parallel Deployment**: All 5 stacks deploy simultaneously
2. **Sequential Deployment**: One after another
3. **Grouped Deployment**: Source stacks parallel, then target stacks

**Decision**: **Sequential Deployment (Keep Current Pattern)**

**Rationale**:
- Deployment time is not the bottleneck (test execution is)
- Sequential deployment is simpler and more reliable
- Avoids AWS API throttling from parallel CloudFormation operations
- ~2 minutes per stack = ~10 minutes total deployment (acceptable)
- Main benefit is parallel TEST execution, not parallel deployment

---

### Decision 6: DynamoDB Table Consolidation

**Question**: Can we reduce the number of DynamoDB tables?

**Current Tables**:
1. `StandaloneTable` - Refactor mode, no dependencies
2. `DynamoDbTable` - Refactor mode, grouped with buckets
3. `ParameterTable` - Parameter dependency testing
4. `RenameTable` - Rename mode GetAtt testing

**Decision**: **Keep All 4 Tables**

**Rationale**:
- Each serves a distinct test purpose
- `StandaloneTable` vs `DynamoDbTable`: Different refactor test scenarios (isolated vs grouped)
- `ParameterTable`: Unique parameter dependency test case
- `RenameTable`: Rename mode testing (different stack)
- DynamoDB tables are fast to create (~30s) and zero cost
- Consolidation would complicate test logic without meaningful benefit

---

## Phase 1: High-Level Design

### Component Changes

#### 1. CDK Stack Definition (`test/cdk/lib/index.ts`)

**Changes**:
- Add `ResourceSet` type: `'refactor' | 'import' | 'rename' | 'none'`
- Modify `TestStackProps` to include `resourceSet: ResourceSet`
- Refactor resource creation logic into conditional blocks based on `resourceSet`
- Replace EC2 Instance with Launch Template in 'import' resource set
- Remove Lambda function and LambdaTargetBucket from all resource sets

**Resource Distribution**:

**Refactor Set**:
- ✅ ParameterTableName parameter
- ✅ StandaloneBucket
- ✅ StandaloneTable
- ✅ Bucket-1, Bucket-2
- ✅ DynamoDbTable
- ✅ ParameterTable
- ✅ KeyPair (for error testing)

**Import Set**:
- ✅ ParameterTableName parameter
- ✅ Bucket-1, Bucket-2
- ✅ DynamoDbTable
- ✅ KeyPair
- ✅ SecurityGroup
- ✅ IAM Role
- ✅ IAM InstanceProfile
- ✅ **LaunchTemplate** (NEW - replaces Instance)
- ❌ Instance (REMOVED)

**Rename Set**:
- ✅ RenameBucket
- ✅ RenameTable
- ✅ RenameQueue
- ✅ DependencyBucket
- ✅ DependentTable
- ❌ LambdaTargetBucket (REMOVED)
- ❌ TestFunction (REMOVED)

---

#### 2. CDK App (`test/cdk/bin/app.ts`)

**Changes**:
- Create 3 stacks instead of 1:
  - `CfnTeleportRefactorTest1` with `resourceSet: 'refactor'`
  - `CfnTeleportImportTest1` with `resourceSet: 'import'`
  - `CfnTeleportRenameTest1` with `resourceSet: 'rename'`

**New Structure**:
```typescript
// Refactor source stack
new TestStack(app, 'CfnTeleportRefactorTest1', {
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: process.env.CDK_DEFAULT_REGION },
  resourceSet: 'refactor',
});

// Import source stack
new TestStack(app, 'CfnTeleportImportTest1', {
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: process.env.CDK_DEFAULT_REGION },
  resourceSet: 'import',
});

// Rename stack
new TestStack(app, 'CfnTeleportRenameTest1', {
  env: { account: process.env.CDK_DEFAULT_ACCOUNT, region: process.env.CDK_DEFAULT_REGION },
  resourceSet: 'rename',
});
```

---

#### 3. YAML Templates (New Files)

**Create**: `test/cdk/refactor-test2-template.yaml`
```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: CfnTeleportRefactorTest2 - YAML target stack for refactor mode tests

Parameters:
  ParameterTableName:
    Type: String
    Default: cfn-teleport-param-test
    Description: Table name controlled by stack parameter

Resources:
  WaitConditionHandle:
    Type: AWS::CloudFormation::WaitConditionHandle

Outputs:
  StackFormat:
    Description: Format of this stack template
    Value: YAML
  ParameterName:
    Description: Name of the parameter
    Value: !Ref ParameterTableName
```

**Create**: `test/cdk/import-test2-template.yaml`
```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: CfnTeleportImportTest2 - YAML target stack for import mode tests

Parameters:
  ParameterTableName:
    Type: String
    Default: cfn-teleport-param-test
    Description: Table name controlled by stack parameter

Resources:
  WaitConditionHandle:
    Type: AWS::CloudFormation::WaitConditionHandle

Outputs:
  StackFormat:
    Description: Format of this stack template
    Value: YAML
  ParameterName:
    Description: Name of the parameter
    Value: !Ref ParameterTableName
```

**Note**: `stack2-template.yaml` can be deleted or kept for reference

---

#### 4. Makefile (`test/cdk/Makefile`)

**Changes**:
- Update `deploy` target to deploy 5 stacks:
  - 3 CDK stacks: RefactorTest1, ImportTest1, RenameTest1
  - 2 YAML stacks: RefactorTest2, ImportTest2
- Update `DESTROY` target to destroy all 5 stacks
- Update `verify-formats` to check all 5 stacks (3 JSON, 2 YAML)

**New Deploy Target**:
```makefile
deploy: build
	@echo "Deploying Refactor Test Stacks..."
	@npx cdk deploy --require-approval never "CfnTeleportRefactorTest1"
	@aws cloudformation deploy \
		--template-file refactor-test2-template.yaml \
		--stack-name CfnTeleportRefactorTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	
	@echo "Deploying Import Test Stacks..."
	@npx cdk deploy --require-approval never "CfnTeleportImportTest1"
	@aws cloudformation deploy \
		--template-file import-test2-template.yaml \
		--stack-name CfnTeleportImportTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	
	@echo "Deploying Rename Test Stack..."
	@npx cdk deploy --require-approval never "CfnTeleportRenameTest1"
	
	@$(MAKE) verify-formats
```

---

#### 5. GitHub Actions Workflow (`.github/workflows/pr-test.yml`)

**Major Changes**:
- Keep existing `build` and `cdk-deploy` jobs (update stack names)
- Replace single `integration-test` job with 3 parallel jobs:
  - `test-refactor`
  - `test-import`
  - `test-rename`
- Update cleanup to handle 5 stacks
- Update `report-status` to depend on all 3 test jobs

**Job Dependencies**:
```
changes → build → integration-test (refactor, import, rename)
       → cdk-deploy ↗
```

**Parallel Test Jobs Structure**:
Each job:
1. Downloads binary artifact
2. Sets stack-specific environment variables
3. Runs test script specific to that mode
4. Reports results independently

---

## Phase 2: Detailed Implementation Steps

### Step 1: Update CDK Stack Definition
**File**: `test/cdk/lib/index.ts`

**Changes**:
1. Add `ResourceSet` type and update `TestStackProps`
2. Replace `resources: boolean` with `resourceSet: ResourceSet`
3. Create resource set conditional blocks
4. Implement Launch Template (replace Instance)
5. Remove Lambda function and LambdaTargetBucket
6. Remove Lambda test outputs

**Affected Lines**: ~240 lines (full rewrite of resource creation logic)

**Risk**: Medium - large refactor, but resources remain similar

---

### Step 2: Update CDK App
**File**: `test/cdk/bin/app.ts`

**Changes**:
1. Replace single stack instantiation with 3 stacks
2. Use new `resourceSet` parameter
3. Remove old Test1/Test2 references

**Affected Lines**: ~20 lines

**Risk**: Low - simple instantiation changes

---

### Step 3: Create YAML Templates
**Files**: 
- `test/cdk/refactor-test2-template.yaml` (NEW)
- `test/cdk/import-test2-template.yaml` (NEW)

**Changes**:
1. Copy `stack2-template.yaml` structure
2. Update descriptions for each stack
3. Maintain parameter consistency

**Risk**: Very Low - static YAML files

---

### Step 4: Update Makefile
**File**: `test/cdk/Makefile`

**Changes**:
1. Update `deploy` target for 5 stacks
2. Update `DESTROY` target for 5 stacks
3. Update `verify-formats` for 3 JSON + 2 YAML stacks

**Affected Lines**: ~30 lines

**Risk**: Low - shell commands, testable locally

---

### Step 5: Update GitHub Actions Workflow (Part 1: Deployment)
**File**: `.github/workflows/pr-test.yml`

**Changes**:
1. Update `cdk-deploy` job to deploy 5 stacks
2. No logic changes, just deploy command

**Affected Lines**: ~5 lines

**Risk**: Low - deployment is same process, just more stacks

---

### Step 6: Update GitHub Actions Workflow (Part 2: Test Jobs)
**File**: `.github/workflows/pr-test.yml`

**Changes**:
1. Rename `integration-test` to `test-refactor`
2. Update test script for refactor-only tests
3. Create `test-import` job with import-only tests
4. Create `test-rename` job with rename-only tests
5. Update cleanup to destroy 5 stacks

**Affected Lines**: ~600 lines (test job is very large)

**Risk**: High - complex test logic, many environment variables

**Mitigation**: 
- Split incrementally
- Test each job independently
- Maintain helper function pattern

---

### Step 7: Update Root Makefile (if needed)
**File**: `Makefile` (root)

**Changes**:
1. Update `test-clean-all` if tagging strategy changes (unlikely)
2. Update `test-reset` if stack names change

**Affected Lines**: ~5 lines

**Risk**: Very Low

---

## Phase 3: Testing Strategy

### Unit Testing (CDK)
1. **CDK Synth**: Verify all 3 stacks synthesize without errors
   ```bash
   cd test/cdk && npx cdk synth
   ```

2. **Template Validation**: Check synthesized templates contain expected resources
   ```bash
   cd test/cdk && npx cdk synth CfnTeleportRefactorTest1 | jq '.Resources | keys'
   ```

3. **Resource Count**: Verify each stack has correct number of resources

### Integration Testing (Local)
1. **Deploy Stacks**: Deploy all 5 stacks to test AWS account
   ```bash
   cd test/cdk && make deploy
   ```

2. **Manual Test Execution**: Run each test stage manually
   ```bash
   # Refactor tests
   cfn-teleport --source CfnTeleportRefactorTest1 --target CfnTeleportRefactorTest2 --yes --mode refactor --resource <ID>
   
   # Import tests
   cfn-teleport --source CfnTeleportImportTest1 --target CfnTeleportImportTest2 --yes --mode import --resource <ID>
   
   # Rename tests
   cfn-teleport --source CfnTeleportRenameTest1 --target CfnTeleportRenameTest1 --yes --resource <ID>:<NewID>
   ```

3. **Verify Launch Template**: Confirm Launch Template is created and references dependencies

4. **Cleanup**: Verify all stacks destroy successfully
   ```bash
   cd test/cdk && make DESTROY
   ```

### CI Testing (GitHub Actions)
1. **Branch Deploy**: Push to feature branch, trigger CI
2. **Monitor Parallel Jobs**: Verify 3 test jobs run simultaneously
3. **Check Timing**: Verify total time ≤7 minutes
4. **Verify All Pass**: All 3 test jobs must pass

---

## Phase 4: Rollout Strategy

### Incremental Rollout

#### Stage 1: CDK Changes (Local Testing)
**Goal**: Update CDK stack definition and test locally

**Steps**:
1. Update `lib/index.ts` with resource sets
2. Update `bin/app.ts` with 3 stack instantiations
3. Create YAML template files
4. Update Makefile
5. Test locally: `cd test/cdk && make deploy`
6. Verify all 5 stacks deploy successfully
7. Test one migration manually

**Success Criteria**: All 5 stacks deploy, at least one test migration works

**Rollback**: Delete stacks, revert CDK changes

---

#### Stage 2: GitHub Actions Sequential Testing
**Goal**: Test new stacks with sequential CI workflow (no parallelization yet)

**Steps**:
1. Update `cdk-deploy` job with new stack names
2. Keep single `integration-test` job but update to use new stack names
3. Run all tests sequentially against new 5-stack architecture
4. Push to feature branch, trigger CI

**Success Criteria**: All tests pass in CI with new stacks (even if sequential)

**Rollback**: Revert GitHub Actions changes

---

#### Stage 3: Parallel Test Execution
**Goal**: Split test job into 3 parallel jobs

**Steps**:
1. Create `test-refactor`, `test-import`, `test-rename` jobs
2. Update each job with stage-specific tests
3. Update cleanup to handle 5 stacks
4. Push to feature branch, trigger CI
5. Monitor parallel execution and timing

**Success Criteria**: 
- All 3 jobs run in parallel
- All tests pass
- Total time ≤7 minutes

**Rollback**: Revert to Stage 2 (sequential tests with new stacks)

---

#### Stage 4: Cleanup Old Infrastructure
**Goal**: Remove old stack references and unused code

**Steps**:
1. Delete `stack2-template.yaml` (if not needed)
2. Remove old stack name constants
3. Update documentation/comments
4. Final CI validation

**Success Criteria**: Clean codebase, no old references

---

## Phase 5: Validation & Metrics

### Success Metrics

#### Performance Metrics
- **Stack Creation Time**: Measure deployment time for each stack
  - Target: ≤2 minutes per stack
  - Measure: GitHub Actions logs (time between deploy commands)

- **Test Execution Time**: Measure total CI time from start to finish
  - Target: ≤7 minutes
  - Measure: GitHub Actions workflow duration

- **Parallel Speedup**: Compare sequential vs parallel test duration
  - Target: ~3x faster
  - Measure: Individual job duration vs total workflow duration

#### Cost Metrics
- **AWS Cost Explorer**: Verify $0.00 cost for test resources
  - Check after 1 week of CI runs
  - Filter by tag: `ApplicationName=cfn-teleport-test`

#### Quality Metrics
- **Test Pass Rate**: Maintain 100% pass rate
- **Flaky Test Rate**: Must not increase
- **Coverage**: All test scenarios from old infrastructure still covered

---

## Risk Mitigation

### Risk 1: Launch Template Doesn't Validate KeyPair
**Mitigation**: Test locally first, verify cfn-teleport can move Launch Template with KeyPair reference

**Fallback**: Use EC2 LaunchConfiguration (deprecated but still works) or keep EC2 Instance

---

### Risk 2: Parallel Jobs Cause AWS API Throttling
**Mitigation**: 
- Stagger test start times by 10 seconds
- Reduce concurrent API calls per job
- Monitor CloudFormation API errors in logs

**Fallback**: Add delays between operations or serialize some tests

---

### Risk 3: Increased Stack Count Hits Service Limits
**Mitigation**: 
- Verify stack limit (default 200 per region)
- 5 stacks is well below limit
- Ensure cleanup always runs (even on failure)

**Fallback**: None needed (5 << 200)

---

### Risk 4: Complex GitHub Actions Changes Introduce Bugs
**Mitigation**: 
- Incremental rollout (Stage 1 → 2 → 3)
- Test each stage in feature branch before merging
- Keep helper functions for consistency

**Fallback**: Revert to previous stage if issues occur

---

## Appendix A: File Change Summary

| File | Change Type | Lines Changed | Risk Level |
|------|-------------|---------------|------------|
| `test/cdk/lib/index.ts` | Major Refactor | ~240 | Medium |
| `test/cdk/bin/app.ts` | Minor Update | ~20 | Low |
| `test/cdk/refactor-test2-template.yaml` | New File | ~25 | Very Low |
| `test/cdk/import-test2-template.yaml` | New File | ~25 | Very Low |
| `test/cdk/Makefile` | Update | ~30 | Low |
| `.github/workflows/pr-test.yml` | Major Refactor | ~600 | High |
| `Makefile` (root) | Minor Update | ~5 | Very Low |
| **Total** | | **~945 lines** | |

---

## Appendix B: Resource Mapping

### Current (2 stacks) → Target (5 stacks)

| Resource | Current Stack | Target Stack(s) |
|----------|---------------|-----------------|
| StandaloneBucket | Test1 | RefactorTest1 |
| StandaloneTable | Test1 | RefactorTest1 |
| Bucket-1 | Test1 | RefactorTest1, ImportTest1 |
| Bucket-2 | Test1 | RefactorTest1, ImportTest1 |
| DynamoDbTable | Test1 | RefactorTest1, ImportTest1 |
| ParameterTable | Test1 | RefactorTest1 |
| KeyPair | Test1 | RefactorTest1, ImportTest1 |
| **Instance** | Test1 | **REMOVED** |
| **→ LaunchTemplate** | - | **ImportTest1 (NEW)** |
| SecurityGroup | Test1 | ImportTest1 |
| Role | Test1 | ImportTest1 |
| InstanceProfile | Test1 | ImportTest1 |
| RenameBucket | Test1 | RenameTest1 |
| RenameTable | Test1 | RenameTest1 |
| RenameQueue | Test1 | RenameTest1 |
| DependencyBucket | Test1 | RenameTest1 |
| DependentTable | Test1 | RenameTest1 |
| **LambdaTargetBucket** | Test1 | **REMOVED** |
| **TestFunction** | Test1 | **REMOVED** |
| WaitConditionHandle | Test2 | RefactorTest2, ImportTest2 |

---

## Appendix C: Timeline Estimate

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 0: Research & Decisions | 1 hour | Analysis complete |
| Phase 1: High-Level Design | 1 hour | Phase 0 |
| Phase 2: CDK Implementation | 3-4 hours | Phase 1 |
| Phase 3: GitHub Actions Refactor | 3-4 hours | Phase 2 |
| Phase 4: Local Testing | 2 hours | Phase 2 |
| Phase 5: CI Testing & Iteration | 2-3 hours | Phase 3, 4 |
| **Total Estimate** | **12-15 hours** | |

**Note**: This is development time estimate, not elapsed time (CI runs add wait time)
