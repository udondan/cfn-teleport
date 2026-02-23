# Current State Analysis: Integration Test Infrastructure

**Generated**: 2026-02-23  
**Context**: Enhancement to improve integration test performance and cost efficiency

---

## Executive Summary

The current integration test infrastructure deploys 2 CloudFormation stacks and runs 3 test stages **sequentially**, resulting in very long test execution times. Additionally, some resources (EC2 instances) take significant time to create and may incur costs.

### Key Findings

1. **Sequential Test Execution**: ~15-20 minutes total test time due to sequential stages
2. **Slow Resource Creation**: EC2 Instance takes 2-3 minutes to create
3. **Cost Risk**: EC2 t2.micro instances, Lambda functions, and potentially other resources may incur costs
4. **Test Isolation**: Current 2-stack design limits parallelization opportunities

---

## Current Test Architecture

### Stack Configuration

**Stack 1: CfnTeleportTest1**
- Deployment: CDK (JSON format)
- Contains: All test resources initially
- Purpose: Source stack for testing resource migration

**Stack 2: CfnTeleportTest2**
- Deployment: AWS CLI (YAML format from `stack2-template.yaml`)
- Contains: Minimal placeholder (WaitConditionHandle)
- Purpose: Target stack for testing resource migration and YAML format preservation

### Test Resources (in Stack1)

#### Currently Deployed Resources

| Resource | Type | Purpose | Creation Time | Cost Risk |
|----------|------|---------|---------------|-----------|
| **EC2 Instance** | `AWS::EC2::Instance` (t2.micro) | Test instance migration | **2-3 minutes** | **YES** - charges after free tier |
| KeyPair | `AWS::EC2::KeyPair` | Test KeyPair migration | <10s | FREE |
| SecurityGroup | `AWS::EC2::SecurityGroup` | EC2 dependency | <10s | FREE |
| IAM Role | `AWS::IAM::Role` | EC2 dependency | <10s | FREE |
| Instance Profile | `AWS::IAM::InstanceProfile` | EC2 dependency | <10s | FREE |
| VPC (Lookup) | `AWS::EC2::Vpc` | Default VPC lookup | N/A | FREE |
| S3 Buckets (5x) | `AWS::S3::Bucket` | Test bucket migration | <10s each | FREE (no storage) |
| DynamoDB Tables (4x) | `AWS::DynamoDB::Table` | Test table migration | ~30s each | FREE (on-demand, no data) |
| Lambda Function | `AWS::Lambda::Function` | Test env var references | ~10s | **MAYBE** - compute charges |
| SQS Queue | `AWS::SQS::Queue` | Test Fn::Sub references | <10s | FREE |

**Total Stack Creation Time**: ~4-5 minutes (mostly EC2 instance)

#### Resource Groupings

1. **Standalone Resources** (refactor mode test):
   - `StandaloneBucket`, `StandaloneTable`
   - NO dependencies, NO outputs

2. **Regular Resources** (refactor mode test):
   - `Bucket-1`, `Bucket-2`, `DynamoDbTable`

3. **EC2 Complex** (import mode test):
   - `Instance`, `KeyPair`, `SecurityGroup`, `Role`, `InstanceProfile`
   - Many cross-dependencies

4. **Rename Test Resources** (same-stack rename):
   - `RenameBucket`, `RenameTable`, `RenameQueue`
   - `DependencyBucket`, `DependentTable`
   - `LambdaTargetBucket`, `TestFunction`

5. **Parameter Test Resources**:
   - `ParameterTable` - depends on `ParameterTableName` parameter

---

## Current Test Flow

### Integration Test Stages (Sequential)

#### Stage 1: Refactor Mode Tests (~5-7 minutes)
**Location**: `.github/workflows/pr-test.yml:156-344`

Tests:
1. Move standalone resources (StandaloneBucket, StandaloneTable) → Stack2
2. Move regular resources (Bucket-1, Bucket-2, DynamoDbTable) → Stack2
3. Move all resources back to Stack1
4. Move ParameterTable (parameter dependency test)
5. Verify refactor mode REJECTS KeyPair alone (dangling reference)
6. Verify refactor mode REJECTS KeyPair+dependencies (requires replacement)

**Dependencies**: Stack1 (with all resources), Stack2 (empty)

#### Stage 2: Import Mode Tests (~5-7 minutes)
**Location**: `.github/workflows/pr-test.yml:345-418`

Tests:
1. Move ALL resources including EC2 complex → Stack2 (import mode)
2. Move ALL resources back to Stack1 (import mode)

**Dependencies**: Stack1 (reset to original state), Stack2 (empty)

#### Stage 3: Same-Stack Rename Tests (~3-5 minutes)
**Location**: `.github/workflows/pr-test.yml:419-628`

Tests:
1. Rename RenameBucket → RenamedBucket (test Ref in Output)
2. Rename RenameTable → RenamedTable (test GetAtt in Output)
3. Rename RenameQueue → RenamedQueue (test Fn::Sub with Ref)
4. Rename DependencyBucket → RenamedDependencyBucket (test DependsOn)
5. Rename LambdaTargetBucket → RenamedLambdaTarget (test Lambda env vars with Ref and GetAtt)

**Dependencies**: Stack1 only (no cross-stack operations)

**Total Sequential Time**: ~15-20 minutes + cleanup

---

## Performance Bottlenecks

### 1. Sequential Test Execution
- **Impact**: 3x longer than necessary
- **Root Cause**: All tests share the same 2 stacks; must run sequentially to avoid conflicts
- **Solution Potential**: With 6 stacks (2 per stage), tests can run in parallel

### 2. EC2 Instance Creation
- **Impact**: Adds 2-3 minutes to stack deployment
- **Usage**: Only used in import mode tests (Stage 2)
- **Problem**: 
  - Takes longest of any resource to create
  - Requires replacement when moved (cannot use refactor mode)
  - Only validates KeyPair relationship and IAM role attachment

### 3. Resource Creation Dependencies
- **Impact**: Stack1 deployment is serial due to dependencies
- **Example**: Instance depends on SecurityGroup, Role, KeyPair
- **Note**: This is inherent to CloudFormation; cannot be optimized

---

## Cost Analysis

### Resources with Potential Costs

#### 1. EC2 Instance (t2.micro)
- **Type**: `AWS::EC2::Instance`
- **Instance Type**: t2.micro
- **Cost**: 
  - Free tier: 750 hours/month
  - Beyond free tier: ~$0.0116/hour (~$8.50/month)
  - **Per test run**: ~5 minutes = **~$0.001** (negligible but accumulates)
- **Risk Level**: LOW (within free tier for typical usage)

#### 2. Lambda Function
- **Type**: `AWS::Lambda::Function`
- **Runtime**: Node.js 20.x
- **Cost**:
  - Free tier: 1M requests/month, 400,000 GB-seconds
  - **Per test run**: Never invoked during tests = **$0.00**
- **Risk Level**: NONE (never invoked)

#### 3. DynamoDB Tables (4x)
- **Type**: `AWS::DynamoDB::Table`
- **Billing Mode**: On-Demand (inferred from CDK defaults)
- **Cost**:
  - Free tier: 25 GB storage, 25 WCU/RCU
  - **Per test run**: No data written = **$0.00**
- **Risk Level**: NONE (no operations)

#### 4. S3 Buckets (5x)
- **Type**: `AWS::S3::Bucket`
- **Cost**:
  - Free tier: 5 GB storage, 20,000 GET requests
  - **Per test run**: No objects stored = **$0.00**
- **Risk Level**: NONE (empty buckets)

#### 5. Other Resources
- KeyPair, SecurityGroup, IAM Role, SQS Queue, VPC: **FREE** (no charges)

### Total Cost Per Test Run
**Current**: ~$0.00 - $0.001 (effectively free within free tier)

**Risk**: Continuous CI runs could exceed free tier limits if many contributors

---

## Technical Constraints

### CloudFormation Limitations
1. **Resource Import**: Some resources cannot be imported (e.g., retained for legacy reasons)
2. **Template Format**: Must preserve JSON vs YAML format between stacks
3. **Parameter Consistency**: Parameters must exist in both stacks for dependent resources

### CDK Constraints
1. **Logical ID Hashing**: CDK generates hashed logical IDs (e.g., `Bucket182C536A1`)
2. **Dynamic Lookup**: Tests use `jq` to find logical IDs dynamically from templates
3. **VPC Lookup**: Uses default VPC (must exist in test account)

### Test Coverage Requirements
1. **Refactor mode**: Test with standalone resources, dependencies, parameters
2. **Import mode**: Test with resources requiring replacement (KeyPair)
3. **Rename mode**: Test all CloudFormation intrinsic function types (Ref, GetAtt, Fn::Sub, DependsOn)
4. **Format preservation**: JSON (CDK) vs YAML (CLI) templates

---

## Pain Points

### For Developers
1. **Slow feedback**: 15-20 minute wait for integration test results
2. **Cannot iterate quickly**: Must wait for full test suite on every change
3. **Hard to debug**: Sequential execution makes it hard to isolate failures

### For CI/CD
1. **GitHub Actions minutes**: Consuming ~20 minutes per PR (costs money for private repos)
2. **Concurrent PRs**: Cannot run in parallel due to shared stacks (avoided by concurrency group)
3. **Flaky tests**: Long-running tests more prone to AWS API throttling/timeouts

---

## Opportunities for Improvement

### 1. Parallel Test Execution
**Approach**: Create 6 stacks (2 per test stage)
- **Refactor stacks**: `CfnTeleportRefactorTest1`, `CfnTeleportRefactorTest2`
- **Import stacks**: `CfnTeleportImportTest1`, `CfnTeleportImportTest2`
- **Rename stacks**: `CfnTeleportRenameTest1` (only 1 needed)

**Impact**: 3x faster (5-7 minutes total vs 15-20 minutes)

### 2. Replace EC2 Instance
**Options**:
- **Launch Template**: Create `AWS::EC2::LaunchTemplate` referencing KeyPair
  - Validates KeyPair relationship without creating instance
  - Creation time: <10 seconds
  - Cost: FREE (no instances launched)
- **Remove entirely**: If KeyPair testing is not critical

**Impact**: -2-3 minutes from stack creation, guaranteed $0 cost

### 3. Resource Minimization
**Refactor Test**: Only needs standalone resources (2 buckets, 1 table)
**Import Test**: Needs complex dependencies (keep most resources but replace EC2)
**Rename Test**: Keep all reference test resources (critical for validation)

---

## Questions for Clarification

1. **KeyPair Testing**: Is validating KeyPair migration essential? 
   - If YES: Use Launch Template instead of EC2 Instance
   - If NO: Remove KeyPair, SecurityGroup, Role, InstanceProfile entirely

2. **Lambda Testing**: Is testing Lambda environment variable references essential?
   - Currently tests Ref and GetAtt in Lambda env vars
   - Could be covered by other resources (e.g., Outputs)

3. **DynamoDB Count**: Do we need 4 DynamoDB tables?
   - `StandaloneTable` (refactor)
   - `DynamoDbTable` (refactor)
   - `ParameterTable` (parameter test)
   - `RenameTable` (rename test)
   - Could consolidate or reduce

4. **Concurrent PR Testing**: Should different PRs be able to test concurrently?
   - Current: `concurrency.group: pr-test-${{ github.ref }}` prevents same-branch concurrency
   - Stack names include account ID, so different accounts OK
   - Same account, different PRs would conflict with current 2-stack design

---

## Success Criteria for Enhancement

1. **Performance**: Integration tests complete in **≤7 minutes** (currently 15-20 minutes)
2. **Cost**: **Zero cost** for all test resources (no paid services)
3. **Coverage**: Maintain 100% test coverage for all modes (refactor, import, rename)
4. **Reliability**: No increase in test flakiness or AWS API issues
5. **Maintainability**: Clear separation of test stages with independent stacks

---

## Appendix: Resource Creation Time Benchmarks

| Resource Type | Typical Creation Time | Notes |
|---------------|----------------------|-------|
| EC2 Instance | 120-180 seconds | Slowest resource |
| DynamoDB Table | 20-40 seconds | Depends on config |
| S3 Bucket | 5-10 seconds | Fast |
| Lambda Function | 10-15 seconds | Medium |
| IAM Role | 5-10 seconds | Fast |
| SecurityGroup | 5-10 seconds | Fast |
| KeyPair | 5-10 seconds | Fast |
| SQS Queue | 5-10 seconds | Fast |
| Launch Template | 5-10 seconds | Fast alternative to EC2 |

**Source**: Empirical observation from GitHub Actions logs and AWS CloudFormation console
