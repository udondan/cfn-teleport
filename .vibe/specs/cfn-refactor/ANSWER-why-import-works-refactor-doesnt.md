# Why Legacy Import/Export CAN Move KeyPairs But RefactorStacks Cannot

## TL;DR

**Import/Export**: Doesn't touch the resource → No tag updates → Works with KeyPairs → **But can orphan resources on failure**

**RefactorStacks**: Updates `aws:cloudformation:*` tags → KeyPair tags require Replacement → **Cannot move KeyPairs** → But operations are atomic and safe

---

## The Key Difference

### Legacy Import/Export API

**What it does**:
1. Removes resource from source stack CloudFormation management (`DeletionPolicy: Retain`)
2. **Resource continues to exist in AWS unchanged**
3. Imports existing resource into target stack by physical ID
4. **No modifications to the resource itself** - just CloudFormation bookkeeping

**Why KeyPairs work**:
- The actual KeyPair resource is **never modified**
- No tag updates
- CloudFormation just "takes ownership" of an existing resource
- Resource keeps all existing tags, properties, etc.

**The danger**:
```
Step 1-4: Remove from source stack ✅ 
          (KeyPair now unmanaged by CloudFormation)
Step 5-6: Import to target stack ❌ FAILS
          (Template validation error, permissions issue, etc.)

Result: KeyPair is ORPHANED
- Source stack no longer manages it
- Target stack never acquired it  
- Resource exists in AWS but NO stack controls it
- Must manually clean up
```

---

### RefactorStacks API

**What it does** (from AWS documentation):
```json
{
    "Action": "MOVE",
    "Entity": "RESOURCE",
    "PhysicalResourceId": "MyFunction",
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
}
```

**The process**:
1. Validates entire refactor operation BEFORE making any changes
2. **Removes old CloudFormation tags** (source stack tracking)
3. **Adds new CloudFormation tags** (target stack tracking)
4. Updates both source and target stack definitions
5. If anything fails, **rolls back completely** - no orphaned resources

**Why KeyPairs fail**:
- RefactorStacks **MUST** update tags to track ownership transfer
- From AWS EC2::KeyPair documentation:
  ```yaml
  Tags:
    Type: Array of Tag
    Update requires: Replacement  # ← THE PROBLEM
  ```
- "Replacement" means CloudFormation must **DELETE and CREATE** to change tags
- Deleting a KeyPair breaks all EC2 instances using it
- AWS correctly rejects the operation:
  ```
  Stack Refactor does not support AWS::EC2::KeyPair because the 
  resource type does not allow tag updates after creation.
  ```

---

## Why EC2::KeyPair Has This Restriction

### The CloudFormation Design Decision

**EC2 API Level**: KeyPairs DO support tag updates
- `CreateTags` and `DeleteTags` APIs work fine
- Source: https://docs.aws.amazon.com/AWSEC2/latest/APIReference/API_CreateTags.html

**CloudFormation Resource Level**: `AWS::EC2::KeyPair` treats Tags as **immutable**
- Why: Likely a security/auditability design decision
- KeyPairs are security-sensitive (SSH access)
- CloudFormation enforces immutability for safety
- Consequence: Any tag update requires full resource replacement

**The impact**:
```
Old KeyPair: my-keypair-abc123 (private key stored in SSM)
              ↓
CloudFormation tries to update tags
              ↓
Replacement required
              ↓
DELETE old KeyPair (loses private key material)
              ↓  
CREATE new KeyPair (generates new private key)
              ↓
Result: EC2 instances can't connect (wrong key)
```

**CloudFormation's solution**: Refuse to allow tag updates on KeyPairs → RefactorStacks cannot use KeyPairs

---

## The Complete Picture

### Supported Resource Types Comparison

**Import/Export API** - Supports ~1304 resource types:
- Any resource that can be "imported" (CloudFormation can manage it)
- No requirement for tag updates
- Full list: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/resource-import-supported-resources.html

**RefactorStacks API** - Supports SUBSET of import-supported types:
- Only resources with `provisioningType = FULLY_MUTABLE`
- Must support tag updates **without replacement**
- Known unsupported types documented at: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/stack-refactoring.html#stack-refactoring-resource-limitations

**The explicit blocklist includes**:
- `AWS::EC2::KeyPair` - Tags require replacement
- `AWS::EC2::LaunchTemplate` - Similar tag restrictions
- 100+ other resource types with various limitations
- Full list in Stack Refactoring documentation

---

## Why cfn-teleport CANNOT Fallback to Import/Export

### The Temptation

User thinks: "If RefactorStacks rejects KeyPair, just use the old import/export method as fallback"

### Why This Is Dangerous

**Scenario**:
```bash
$ cfn-teleport --source Stack1 --target Stack2 --resource KeyPair

[RefactorStacks rejects KeyPair]
[Tool falls back to import/export]

Step 1: Add DeletionPolicy: Retain to KeyPair ✅
Step 2: Update Stack1 ✅
Step 3: Remove KeyPair from Stack1 template ✅
Step 4: Update Stack1 ✅
        → KeyPair is now UNMANAGED by CloudFormation
Step 5: Add KeyPair to Stack2 template ✅
Step 6: Create import changeset on Stack2 ❌ FAILS
        → Template validation error
        → Stack2 tags limit exceeded
        → Permissions issue
        → ANY failure here leaves KeyPair orphaned

Result: KeyPair exists in AWS but NO stack manages it
        EC2 instances still work, but:
        - Can't update KeyPair via CloudFormation
        - Drift detection won't catch it (not in any stack)
        - Manual cleanup required
        - Infrastructure as Code is broken
```

### The Fundamental Problem

Import/Export is **not atomic**:
- 6 separate steps
- Each step can fail independently
- Failure in steps 5-6 leaves resource orphaned
- **No automatic rollback**

RefactorStacks is **atomic**:
- Single API call
- Validates BEFORE modifying anything
- Either completes fully or rolls back completely
- **No possibility of orphaned resources**

**We're replacing import/export specifically to avoid corruption** - falling back defeats the entire purpose.

---

## Decision for cfn-teleport

### Use RefactorStacks API Exclusively

**Reasoning**:
1. **Safety first**: Atomic operations prevent stack corruption
2. **Clear errors**: AWS provides descriptive error messages for unsupported types
3. **No fallback**: Falling back to import/export reintroduces the corruption risk
4. **Document limitations**: Users need to know some resources can't be moved

### User Communication

**README.md should state**:
```markdown
## Limitations

cfn-teleport uses the CloudFormation Stack Refactoring API for safe, 
atomic resource moves. This API supports a subset of AWS resource types.

**Resources that cannot be moved**:
- AWS::EC2::KeyPair (tag updates require resource replacement)
- 100+ other types (see AWS documentation)

**Workaround**: For unsupported resource types, you can:
1. Use the manual import/export workflow (at your own risk)
2. Recreate the resource in the target stack
3. Leave the resource in the current stack and reference it cross-stack
```

### Error Handling

**Current approach is correct**:
```rust
// Let AWS API reject unsupported types
// Error messages are already descriptive
match client.refactor_stacks().send().await {
    Ok(response) => { /* success */ },
    Err(e) => {
        eprintln!("Error: {}", e);
        // AWS error already explains WHY it failed
        process::exit(1);
    }
}
```

**Do NOT add pre-validation** because:
- Maintaining an unsupported-types list is fragile (AWS adds new types constantly)
- AWS API is the source of truth
- API error messages are clear and actionable

---

## Sources

1. **Legacy Import/Export**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/refactor-stacks.html
2. **RefactorStacks API with Tag Operations**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/stack-refactoring.html
3. **EC2 KeyPair Tags Restriction**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-ec2-keypair.html
4. **Unsupported Resource Types**: https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/stack-refactoring.html#stack-refactoring-resource-limitations

---

## Conclusion

**The old import/export API CAN move KeyPairs because it doesn't modify the resource - it just reassigns CloudFormation management without touching tags.**

**The RefactorStacks API CANNOT move KeyPairs because it MUST update `aws:cloudformation:*` tags to track ownership transfer, and KeyPair tags require resource Replacement.**

**This is the correct trade-off**: We gain atomicity and safety by accepting a smaller set of supported resource types. The alternative (import/export) supports more types but risks leaving infrastructure in a corrupted state.

**cfn-teleport's approach is correct**: Use RefactorStacks exclusively, rely on AWS error messages, document limitations. Do NOT fallback to import/export.
