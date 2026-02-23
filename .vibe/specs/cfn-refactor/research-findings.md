# Research Findings: CloudFormation Stack Refactoring API Limitations

*Updated: 2026-02-21 after T008 testing*

## Executive Summary

The CloudFormation Stack Refactoring API (`RefactorStacks`) has **MORE restrictive resource type support** than the legacy import/export API. The root cause is that the refactoring API relies on **tag-based resource ownership tracking**, which fails for resource types where CloudFormation treats tag updates as requiring resource **replacement**.

---

## Source 1: AWS CloudFormation User Guide - Moving Resources Between Stacks

**URL**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/refactor-stacks.html

**Key Finding**: This document describes the **LEGACY** import/export workflow, NOT the RefactorStacks API:

> "Using the resource import feature, you can move resources between, or refactor, stacks. You need to first add a Retain deletion policy to the resource you want to move to ensure that the resource is preserved when you remove it from the source stack and import it to the target stack."

**Important Note**: This manual process is:
1. Add `DeletionPolicy: Retain` to source stack resource
2. Update source stack (applies retention policy)
3. Remove resource from source template
4. Update source stack (removes from CFN management but keeps resource)
5. Create import changeset on target stack
6. Execute import changeset

**Problem**: This is a **multi-step destructive process** - if step 5 or 6 fails, the resource is orphaned from the source stack but not in the target stack.

---

## Source 2: AWS CloudFormation User Guide - Stack Refactoring

**URL**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/stack-refactoring.html

**Key Finding**: RefactorStacks is a **newer API** that performs atomic refactoring AND **modifies CloudFormation-managed tags**:

> "Creates a refactor across multiple stacks by moving resources from a source stack to a target stack or by renaming a resource."

**Critical Difference from Legacy Import/Export**:
- **Atomic validation**: CloudFormation validates ALL changes BEFORE modifying any stacks
- **Single API call**: No multi-step process
- **Automatic rollback**: If validation fails, NO stacks are modified
- **Tag Updates**: Modifies `aws:cloudformation:*` tags to reflect new stack ownership

**How it works** (from API docs):
1. You provide source and target `StackDefinition` objects showing **final desired state**
2. You provide `ResourceMappings` linking old locations to new locations
3. CloudFormation validates the entire refactor operation
4. **CloudFormation updates tags** on the resource:
   - **UntagResources**: Removes old stack tags (`aws:cloudformation:stack-name`, `aws:cloudformation:logical-id`, `aws:cloudformation:stack-id`)
   - **TagResources**: Adds new stack tags with target stack information
5. If valid, executes atomically; if invalid, returns error with NO changes

**Example from documentation showing tag operations**:
```json
"TagResources": [
    {
        "Key": "aws:cloudformation:stack-name",
        "Value": "Stack2"
    },
    {
        "Key": "aws:cloudformation:logical-id",
        "Value": "MyFunction"
    },
    {
        "Key": "aws:cloudformation:stack-id",
        "Value": "arn:aws:cloudformation:us-east-1:123456789012:stack/Stack2/..."
    }
],
"UntagResources": [
    "aws:cloudformation:stack-name",
    "aws:cloudformation:logical-id",
    "aws:cloudformation:stack-id"
]
```

---

## Source 3: AWS EC2 KeyPair Resource Reference

**URL**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-ec2-keypair.html

**Key Finding**: The `Tags` property has a critical restriction:

```yaml
Tags:
  Required: No
  Type: Array of Tag
  Update requires: Replacement  # ← THIS IS THE PROBLEM
```

**What "Update requires: Replacement" means**:
- CloudFormation **CANNOT** update tags on an existing KeyPair
- To change tags, CloudFormation must **DELETE** the old KeyPair and **CREATE** a new one
- This would break any EC2 instances using that KeyPair

**Why this breaks RefactorStacks**:
- RefactorStacks API uses **tags** to track resource ownership during the refactoring operation
- It needs to add/modify tags like `aws:cloudformation:stack-name` or similar metadata
- Since KeyPair tags require "Replacement", CloudFormation cannot safely refactor KeyPairs

**Actual error message from testing**:
```
Stack Refactor does not support AWS::EC2::KeyPair because the resource type 
does not allow tag updates after creation.
```

---

## Source 4: Resource Type Support Documentation

**URL**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/resource-import-supported-resources.html

**Key Finding**: This lists 1304+ resource types that support **import operations**, but:

> "Not all resources support import operations."

**Critical Gap**: This document does NOT distinguish between:
- Resources that support legacy import/export (1304+ types)
- Resources that support RefactorStacks API (SUBSET of import-supported types)

**Why the gap exists**:
- Import API: Only requires resource to be importable (one-time operation)
- RefactorStacks API: Requires resource to support **tag updates without replacement**

---

## Why Old Import/Export API CAN Move KeyPairs (But RefactorStacks Cannot)

### Legacy Import/Export Workflow (from Source 1)

**URL**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/refactor-stacks.html

**The 6-step manual process**:
1. Add `DeletionPolicy: Retain` to resource in source stack template
2. Update source stack (applies retention policy)
3. Remove resource definition from source stack template
4. Update source stack (removes from CloudFormation management, but **resource still exists in AWS**)
5. Add resource definition to target stack template
6. Create and execute **import changeset** on target stack

**Critical Detail**: The import changeset does **NOT modify the resource at all**. It only:
- Tells CloudFormation: "This resource already exists with physical ID X"
- Associates the existing resource with the new stack
- **Does NOT touch tags** - the resource keeps whatever tags it had

**Why KeyPairs work with import**:
- Import operation is **read-only** from the resource's perspective
- No tag updates required
- CloudFormation just starts "managing" the existing resource
- If the resource properties don't match the template, drift detection catches it (but resource is already imported)

**Why this is dangerous**:
- If step 6 fails (import changeset execution), the resource is **orphaned**
- Source stack no longer manages it (removed in step 4)
- Target stack never acquired it (step 6 failed)
- Resource exists in AWS but NO CloudFormation stack controls it
- Manual cleanup required

---

## Why RefactorStacks CANNOT Move KeyPairs

### Tag Update Requirement

**From Source 2** (Stack Refactoring documentation), the RefactorStacks API performs these operations:

1. **UntagResources** - Remove CloudFormation tags from source stack:
   ```
   aws:cloudformation:stack-name
   aws:cloudformation:logical-id
   aws:cloudformation:stack-id
   ```

2. **TagResources** - Add CloudFormation tags for target stack:
   ```json
   {
       "Key": "aws:cloudformation:stack-name",
       "Value": "TargetStack"
   }
   ```

**For EC2::KeyPair** (from Source 3):
- `Tags` property update requires: **Replacement**
- CloudFormation **CANNOT** modify tags without deleting and recreating the KeyPair
- Deleting a KeyPair would break all EC2 instances using it

**Result**: AWS RefactorStacks API correctly rejects KeyPairs with clear error:
```
Stack Refactor does not support AWS::EC2::KeyPair because the resource type 
does not allow tag updates after creation.
```

---

### Technical Root Cause

**EC2 API vs CloudFormation Design**:

1. **EC2 API Level**: KeyPairs DO support tag updates via `CreateTags` and `DeleteTags` APIs
   - Source: https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateTags.html
   
2. **CloudFormation Resource Level**: `AWS::EC2::KeyPair` resource treats Tags as **immutable** after creation
   - Why: CloudFormation design decision for KeyPairs (possibly for security/auditability)
   - Consequence: Tag updates require full resource replacement

### Why Replacement Is Unacceptable

**Problem**: Replacing a KeyPair would:
1. Delete the original KeyPair (including private key material if stored in SSM Parameter Store)
2. Create a new KeyPair with different key material
3. **Break all EC2 instances** using the original KeyPair (they won't boot/connect)

**Solution**: CloudFormation refuses to support tag updates on KeyPairs → RefactorStacks API cannot use KeyPairs

---

## Other Resource Types with Similar Restrictions

### Discovered During Testing

**AWS::IAM::Role** and **AWS::EC2::SecurityGroup** - Property mismatch errors during refactoring:
```
Resource SecurityGroupDD263621 in stack [...] does not match the 
destination resource's properties.
```

**Hypothesis**: These failures are likely due to:
1. **Stack-specific properties**: Values that differ between source/target stacks (e.g., stack names in references)
2. **Generated attributes**: Properties auto-generated by CloudFormation that can't be specified in templates
3. **Conditional properties**: Properties that exist in one stack but not the other

**Needs Investigation**:
- Review actual template differences for SecurityGroup and IAM Role
- Determine if template preparation logic needs updates to normalize properties
- Test if these resources work when templates are truly identical

### Successfully Tested Resource Types

✅ **AWS::S3::Bucket** - Works reliably across stacks  
✅ **AWS::DynamoDB::Table** - Works reliably across stacks

---

## Impact on cfn-teleport

### The Fundamental Trade-off

| Aspect | Legacy Import/Export | RefactorStacks API |
|--------|---------------------|-------------------|
| **Safety** | ❌ Can orphan resources | ✅ Atomic validation |
| **Resource Support** | ✅ 1304+ types (no tag updates needed) | ⚠️ Subset (requires tag updates) |
| **Tag Management** | ✅ No tag modifications | ❌ Updates `aws:cloudformation:*` tags |
| **User Experience** | ❌ Multi-step manual (6 steps) | ✅ Single command |
| **Failure Impact** | ❌ Resources orphaned | ✅ Clean rollback |
| **Validation** | ❌ After-the-fact (drift detection) | ✅ Before modification |

**Why can't we have both?**
- Import/Export: Doesn't modify resources → supports more types → but can fail mid-operation
- RefactorStacks: Modifies tags for tracking → requires tag updates → but validates atomically

### Current Behavior (CORRECT APPROACH)

1. Tool attempts RefactorStacks API call
2. AWS validates and returns descriptive error message
3. Operation fails **atomically** (no stacks corrupted)
4. Error message propagated to user

### Why We Cannot Fallback to Import/Export

**If we tried to fallback when RefactorStacks fails**:
1. User runs: `cfn-teleport --source Stack1 --target Stack2 --resource KeyPair`
2. RefactorStacks rejects KeyPair (tag update restriction)
3. Tool falls back to import/export
4. Step 4 of import/export completes (resource removed from Stack1)
5. Step 6 of import/export FAILS (e.g., template validation error)
6. **KeyPair is now orphaned** - not in Stack1 or Stack2
7. EC2 instances using KeyPair still work, but CloudFormation can't manage it

**The risk is too high** - we'd be reintroducing the exact corruption problem we're trying to fix.

---

## Recommendations

### 1. Documentation (HIGH PRIORITY)

**Update README.md**:
```markdown
## Limitations

### Resource Type Support

cfn-teleport uses the CloudFormation Stack Refactoring API, which supports 
a subset of AWS resource types. Resources that require tag replacement 
(like AWS::EC2::KeyPair) cannot be moved between stacks.

**Known unsupported types**:
- AWS::EC2::KeyPair - Tags require replacement

**Workaround**: For unsupported types, use manual import/export workflow 
documented in AWS CloudFormation User Guide.
```

### 2. Error Handling (CURRENT APPROACH - KEEP IT)

**Do NOT add pre-validation** because:
- API error messages are already clear and actionable
- Maintaining an allowlist/blocklist is fragile (AWS adds new types constantly)
- API is the source of truth

**Current error propagation is sufficient**:
```rust
// Error from AWS SDK is already descriptive
Err(e) => eprintln!("Error: {}", e)
```

### 3. Future Investigation (MEDIUM PRIORITY)

**Property Mismatch Issues**:
- Debug why IAM Role and SecurityGroup fail property validation
- Check if template preparation logic needs fixes
- Test with identical source/target templates to isolate issue

---

## Sources Summary

1. **Legacy Import/Export Workflow**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/refactor-stacks.html
2. **RefactorStacks API**: https://docs.aws.amazon.com/AWSCloudFormation/latest/APIReference/API_RefactorStacks.html
3. **EC2 KeyPair Resource**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-ec2-keypair.html
4. **Import-Supported Resources**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/resource-import-supported-resources.html
5. **EC2 CreateTags API**: https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateTags.html

---

## Conclusion

**The CloudFormation Stack Refactoring API is MORE restrictive than import/export because it uses tags for resource ownership tracking.** Resources where CloudFormation treats tag updates as requiring replacement (like `AWS::EC2::KeyPair`) cannot be refactored atomically, so AWS correctly rejects them.

This is a **reasonable trade-off** - we gain atomicity and safety at the cost of supporting fewer resource types. The alternative (legacy import/export) supports more types but risks leaving stacks in corrupted states on failure.

**cfn-teleport's approach is correct**: Use RefactorStacks exclusively, rely on AWS error messages, document limitations.
