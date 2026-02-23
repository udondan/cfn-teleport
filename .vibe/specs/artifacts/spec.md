# Feature Specification: Parallel Integration Test Infrastructure

**Version**: 1.0  
**Date**: 2026-02-23  
**Status**: Draft

---

## Overview

Transform the cfn-teleport integration test infrastructure from a sequential 2-stack design to a parallel 5-stack design, reducing test execution time by 66% (from ~15-20 minutes to ≤7 minutes) while guaranteeing zero cost for all test resources.

---

## Background

### Current State
- Integration tests deploy 2 CloudFormation stacks (Test1, Test2)
- 3 test stages run sequentially: refactor mode, import mode, same-stack rename
- Total execution time: 15-20 minutes per test run
- EC2 Instance resource takes 2-3 minutes to create (single largest bottleneck)
- Test stages cannot run in parallel due to shared stack resources

### Problems
1. **Slow CI feedback**: Developers wait 15-20 minutes for test results
2. **Sequential bottleneck**: Test stages share stacks, preventing parallelization
3. **Cost risk**: EC2 instances could exceed free tier with high CI usage
4. **Resource bloat**: Lambda function not essential for test coverage

---

## Goals

### Primary Goals
1. **Performance**: Reduce integration test execution time to ≤7 minutes (66% improvement)
2. **Parallelization**: Enable 3 test stages to run concurrently
3. **Cost**: Guarantee zero cost for all test resources (no paid services)
4. **Maintainability**: Clear separation between test stages with isolated stacks

### Non-Goals
- Changing cfn-teleport CLI functionality or business logic
- Modifying test coverage or test quality
- Altering the types of tests performed (refactor, import, rename)

---

## User Stories

### US-1: Parallel Test Execution
**As a** developer  
**I want** integration tests to run in parallel  
**So that** I receive test feedback in ≤7 minutes instead of 15-20 minutes

**Acceptance Criteria:**
- Refactor mode tests run independently in dedicated stacks
- Import mode tests run independently in dedicated stacks
- Rename mode tests run independently in dedicated stack
- All 3 test stages can execute simultaneously without conflicts
- Total test execution time is ≤7 minutes (measured from GitHub Actions)

### US-2: Zero-Cost Test Resources
**As a** project maintainer  
**I want** all test resources to be free (no paid services)  
**So that** continuous integration does not incur AWS costs

**Acceptance Criteria:**
- No EC2 instances are created during tests
- All resources fall within AWS free tier or are completely free
- Test resource creation does not exceed free tier limits under typical CI usage
- Cost estimate per test run is verified as $0.00

### US-3: Maintain Test Coverage
**As a** developer  
**I want** all existing test scenarios to remain covered  
**So that** refactoring the test infrastructure does not reduce quality

**Acceptance Criteria:**
- All refactor mode test cases pass (standalone resources, dependencies, parameters, error cases)
- All import mode test cases pass (KeyPair + dependencies, error cases)
- All rename mode test cases pass (Ref, GetAtt, Fn::Sub, DependsOn)
- Template format preservation tests pass (JSON vs YAML)
- Parameter dependency tests pass

---

## Functional Requirements

### FR-1: Five-Stack Architecture

**Description**: Create 5 isolated CloudFormation stacks instead of 2 shared stacks.

**Stack Types:**
1. **Refactor Source Stack** (CfnTeleportRefactorTest1)
   - Contains resources for refactor mode source testing
   - Deployed via CDK (JSON format)
   
2. **Refactor Target Stack** (CfnTeleportRefactorTest2)
   - Target stack for refactor mode testing
   - Deployed via AWS CLI (YAML format) for format preservation testing

3. **Import Source Stack** (CfnTeleportImportTest1)
   - Contains resources for import mode source testing
   - Deployed via CDK (JSON format)

4. **Import Target Stack** (CfnTeleportImportTest2)
   - Target stack for import mode testing
   - Deployed via AWS CLI (YAML format)

5. **Rename Stack** (CfnTeleportRenameTest1)
   - Single stack for same-stack rename operations
   - Deployed via CDK (JSON format)

**Requirements:**
- Each stack pair must have matching `ParameterTableName` parameter for cross-stack dependency testing
- Stacks must be independently deployable and destroyable
- Stack names must be unique and descriptive of their test purpose
- All stacks must be tagged with `ApplicationName=cfn-teleport-test`

---

### FR-2: Replace EC2 Instance with Launch Template

**Description**: Replace `AWS::EC2::Instance` resource with `AWS::EC2::LaunchTemplate` to eliminate creation time bottleneck and cost risk.

**Requirements:**
- Launch Template must reference KeyPair resource
- Launch Template must reference SecurityGroup resource
- Launch Template must reference IAM InstanceProfile
- Launch Template must NOT launch actual EC2 instances
- KeyPair refactor mode error validation must still work (expects error about unsupported resource type)
- KeyPair import mode migration must still work (all dependencies move together)

**Rationale:**
- Launch Template creates in <10 seconds vs 2-3 minutes for EC2 Instance
- Launch Template is completely free (no compute charges)
- Still validates KeyPair relationships and IAM role attachments
- KeyPair is a special case resource that cannot use refactor mode (requires replacement)

---

### FR-3: Remove Lambda Function

**Description**: Remove `AWS::Lambda::Function` and `LambdaTargetBucket` from test resources.

**Requirements:**
- Remove Lambda function that tests environment variable references
- Remove associated LambdaTargetBucket
- Maintain test coverage for Ref and GetAtt via other resources (Outputs already test these)

**Rationale:**
- Lambda not essential for testing CloudFormation intrinsic functions
- Ref and GetAtt already validated by Output tests
- Simplifies test stack
- Reduces resource count

---

### FR-4: Optimize DynamoDB Table Count

**Description**: Review and consolidate DynamoDB tables where possible while maintaining test coverage.

**Current Tables:**
1. `StandaloneTable` - Refactor mode standalone resource test (no dependencies)
2. `DynamoDbTable` - Refactor mode regular resource test (with other resources)
3. `ParameterTable` - Parameter dependency test (depends on stack parameter)
4. `RenameTable` - Rename mode GetAtt reference test

**Requirements:**
- Each table must serve a distinct test purpose
- Must cover: standalone resource moves, grouped resource moves, parameter dependencies, rename operations
- Tables cannot be consolidated if they serve different test stages (refactor vs rename)
- Final count: To be determined during design (keep all 4 if each is essential)

---

### FR-5: Parallel GitHub Actions Workflow

**Description**: Restructure GitHub Actions workflow to run test stages in parallel.

**Requirements:**
- Refactor mode tests run as independent job
- Import mode tests run as independent job
- Rename mode tests run as independent job
- All 3 jobs start simultaneously after stack deployment completes
- Stack deployment must create all 5 stacks (can be parallel or sequential)
- Each test job must use its designated stacks only (no cross-job stack access)
- Test jobs must report individual pass/fail status
- Final status aggregates all test job results

**Constraints:**
- Must maintain existing concurrency control (`pr-test-${{ github.ref }}`)
- Must use existing AWS credentials (secrets)
- Must clean up all 5 stacks after tests complete

---

### FR-6: Resource Inventory by Stack

**Description**: Define which resources belong in each stack for optimal test coverage.

#### Refactor Stacks (RefactorTest1 source, RefactorTest2 target)
**Resources in RefactorTest1:**
- Parameter: `ParameterTableName`
- S3 Buckets: `StandaloneBucket`, `Bucket-1`, `Bucket-2`
- DynamoDB Tables: `StandaloneTable`, `DynamoDbTable`, `ParameterTable`
- KeyPair: For error testing (refactor mode should reject)

**Resources in RefactorTest2:**
- Parameter: `ParameterTableName` (must match)
- Minimal placeholder: `WaitConditionHandle`

**Test Coverage:**
- Standalone resources (no dependencies): StandaloneBucket, StandaloneTable
- Grouped resources: Bucket-1, Bucket-2, DynamoDbTable
- Parameter dependency: ParameterTable
- Error case: KeyPair alone (dangling reference error)
- Error case: KeyPair with dependencies (unsupported resource type error)

#### Import Stacks (ImportTest1 source, ImportTest2 target)
**Resources in ImportTest1:**
- Parameter: `ParameterTableName`
- S3 Buckets: `Bucket-1`, `Bucket-2`
- DynamoDB Table: `DynamoDbTable`
- KeyPair: `KeyPair`
- Launch Template: `LaunchTemplate` (replaces Instance)
- SecurityGroup: `SecurityGroup`
- IAM Role: `Role`
- IAM InstanceProfile: `InstanceProfile`

**Resources in ImportTest2:**
- Parameter: `ParameterTableName` (must match)
- Minimal placeholder: `WaitConditionHandle`

**Test Coverage:**
- Import mode with resources requiring replacement (KeyPair)
- Complex dependency chain (LaunchTemplate → KeyPair, SecurityGroup, InstanceProfile → Role)
- All resources move together successfully

#### Rename Stack (RenameTest1)
**Resources in RenameTest1:**
- S3 Bucket: `RenameBucket` - tests Ref in Output
- DynamoDB Table: `RenameTable` - tests GetAtt in Output  
- SQS Queue: `RenameQueue` - tests Fn::Sub with resource reference
- S3 Bucket: `DependencyBucket` - tests DependsOn relationship
- DynamoDB Table: `DependentTable` - depends on DependencyBucket

**Test Coverage:**
- Ref references updated on rename
- GetAtt references updated on rename
- Fn::Sub references updated on rename
- DependsOn relationships updated on rename

---

## Non-Functional Requirements

### NFR-1: Performance
- Stack creation time: ≤2 minutes per stack (down from 4-5 minutes)
- Total test execution time: ≤7 minutes (includes deployment + all 3 test stages)
- Parallel test stages complete within 5 minutes of stack deployment

### NFR-2: Cost
- All resources must be completely free or within AWS free tier
- No compute charges (no running instances, no Lambda invocations)
- No storage charges (no S3 objects, no DynamoDB data)
- No data transfer charges

### NFR-3: Reliability
- Test failure rate must not increase
- AWS API throttling risk must not increase
- Flaky test rate must remain ≤ current baseline

### NFR-4: Maintainability
- Test code must remain readable and well-commented
- Stack purposes must be clearly documented
- Resource naming must be consistent across stacks
- Test stages must be independently debuggable

---

## Constraints

### Technical Constraints
1. **CloudFormation Limitations**
   - KeyPair resources require replacement when moved (cannot use refactor mode)
   - Template format (JSON vs YAML) must be preserved
   - Parameters must exist in both source and target stacks for dependent resources

2. **CDK Constraints**
   - CDK generates hashed logical IDs (e.g., `Bucket182C536A1`)
   - Tests must dynamically discover logical IDs from templates
   - Default VPC must exist in test AWS account

3. **GitHub Actions Constraints**
   - Must use existing AWS credentials (secrets)
   - Must respect concurrency control for same-branch PRs
   - Job logs must remain readable (not exceed log size limits)

### Business Constraints
1. **Test Coverage**: Cannot reduce existing test coverage
2. **Backward Compatibility**: Must validate same cfn-teleport binary behavior
3. **CI Cost**: GitHub Actions minutes are a cost factor (faster = cheaper)

---

## Success Criteria

### Quantitative Metrics
1. **Test Execution Time**: ≤7 minutes (measured from GitHub Actions start to finish)
2. **Stack Creation Time**: ≤2 minutes per stack (down from 4-5 minutes)
3. **Cost Per Test Run**: $0.00 (verified via AWS Cost Explorer)
4. **Test Pass Rate**: ≥ current baseline (no regression)
5. **Stack Count**: Exactly 5 stacks deployed during tests

### Qualitative Criteria
1. **All existing test cases pass** with new infrastructure
2. **CI logs remain clear and debuggable** for each test stage
3. **Stack cleanup completes successfully** after tests (no orphaned resources)
4. **Test stages are independently runnable** (can run one stage without others for debugging)

---

## Out of Scope

The following are explicitly **not** part of this enhancement:

1. **CLI Changes**: No changes to cfn-teleport binary functionality
2. **Test Logic Changes**: No changes to test validation logic (what is tested)
3. **New Test Scenarios**: No new test cases beyond current coverage
4. **Local Testing**: No changes to local development test workflows (only CI)
5. **Multi-Region Testing**: Tests remain in single region (us-east-1)
6. **Concurrent PR Testing**: Same-account concurrent PRs still prevented by concurrency group

---

## Assumptions

1. **AWS Account**: Tests run in single AWS account with default VPC available
2. **Free Tier**: AWS account has access to standard free tier services
3. **GitHub Secrets**: AWS credentials are configured in GitHub repository secrets
4. **CDK Version**: CDK version remains compatible (no major upgrade needed)
5. **Stack Naming**: New stack names do not conflict with any existing stacks in test account
6. **Resource Limits**: AWS account has sufficient service limits for 5 stacks (not hitting limits)

---

## Dependencies

### External Dependencies
- AWS CloudFormation API
- AWS CDK CLI
- GitHub Actions runners (ubuntu-latest)
- Node.js 24 (for CDK)
- jq (for template parsing)

### Internal Dependencies
- cfn-teleport binary (built in parallel job)
- test/cdk/lib/index.ts (CDK stack definition)
- test/cdk/Makefile (deployment scripts)
- .github/workflows/pr-test.yml (CI workflow)

---

## Risks and Mitigations

### Risk 1: Parallel Test Interference
**Description**: Parallel tests might interfere if they accidentally access shared resources.  
**Likelihood**: Low  
**Impact**: High (test failures, false positives)  
**Mitigation**: 
- Use distinct stack names for each test stage
- Environment variables scoped to each test job
- Code review to verify no cross-stack references

### Risk 2: Increased Stack Deployment Time
**Description**: Deploying 5 stacks might take longer than deploying 2 stacks.  
**Likelihood**: Medium  
**Impact**: Medium (reduces parallelization gains)  
**Mitigation**: 
- Deploy stacks in parallel where possible
- Launch Template reduces individual stack creation time
- Accept slight deployment time increase for massive test time reduction

### Risk 3: GitHub Actions Complexity
**Description**: Parallel jobs increase workflow complexity and debugging difficulty.  
**Likelihood**: Medium  
**Impact**: Low (one-time implementation cost)  
**Mitigation**: 
- Clear job names and logging
- Each job independently reports status
- Maintain existing helper functions for consistency

### Risk 4: AWS Service Limits
**Description**: 5 stacks might approach CloudFormation stack limits or resource limits.  
**Likelihood**: Low  
**Impact**: High (tests fail to deploy)  
**Mitigation**: 
- Stack count (5) well below default limit (200 per region)
- Resource count per stack remains low (<50 resources)
- Cleanup job ensures stacks are destroyed after tests

---

## Open Questions

None - all questions resolved during analysis phase with user feedback.

---

## Appendix: Resource Comparison

### Before (2 stacks, sequential)
| Stack | Resources | Creation Time | Test Stages |
|-------|-----------|---------------|-------------|
| Test1 | 20+ resources (inc. EC2) | 4-5 min | Refactor, Import, Rename (sequential) |
| Test2 | 1 resource (placeholder) | <10 sec | Target for all |
| **Total** | **2 stacks** | **~5 min** | **15-20 min** |

### After (5 stacks, parallel)
| Stack | Resources | Creation Time | Test Stage |
|-------|-----------|---------------|------------|
| RefactorTest1 | ~8 resources | ~1 min | Refactor (parallel) |
| RefactorTest2 | 1 resource | <10 sec | Refactor target |
| ImportTest1 | ~10 resources (Launch Template) | ~1.5 min | Import (parallel) |
| ImportTest2 | 1 resource | <10 sec | Import target |
| RenameTest1 | ~5 resources | ~1 min | Rename (parallel) |
| **Total** | **5 stacks** | **~2 min** | **≤7 min** |

**Key Improvements:**
- Deployment: ~5 min → ~2 min (60% faster)
- Testing: 15-20 min → ≤7 min (65% faster)
- Cost: ~$0.001 → $0.00 (100% free)
