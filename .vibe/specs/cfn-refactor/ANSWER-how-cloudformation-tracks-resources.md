# How CloudFormation Tracks Resource Ownership - The Real Story

## The Answer to Your Question

**CloudFormation tracks resources in TWO ways**:
1. **Internal database** - Maps PhysicalResourceId to StackId (PRIMARY tracking)
2. **Resource tags** - Applies `aws:cloudformation:*` tags to resources (OPTIONAL, varies by resource type)

**Import/Export doesn't update tags** - It only changes the internal database mapping

**RefactorStacks DOES update tags** - It changes BOTH the internal database AND the resource tags

---

## Evidence: What I Just Discovered

### Test 1: Check what CloudFormation thinks

```bash
$ aws cloudformation describe-stack-resources \
    --stack-name CfnTeleportTest2 \
    --logical-resource-id Bucket182C536A1

Result: CloudFormation says bucket belongs to CfnTeleportTest2 ✅
```

### Test 2: Check actual tags on the S3 bucket

```bash
$ aws s3api get-bucket-tagging \
    --bucket 031700846815-cfn-teleport-test-1

Result:
{
    "TagSet": [
        {
            "Key": "aws:cloudformation:stack-name",
            "Value": "CfnTeleportTest1"    ← STILL SAYS TEST1!
        },
        {
            "Key": "aws:cloudformation:stack-id",
            "Value": "arn:...stack/CfnTeleportTest1/..."
        }
    ]
}
```

### Test 3: Check DynamoDB table

```bash
$ aws dynamodb list-tags-of-resource \
    --resource-arn arn:aws:dynamodb:...:table/cfn-teleport-test

Result:
{
    "Tags": [
        {
            "Key": "ApplicationName",
            "Value": "cfn-teleport-test"
        }
    ]
}

NO aws:cloudformation:* tags AT ALL! But CloudFormation still tracks it!
```

---

## What This Means

### CloudFormation's PRIMARY Tracking: Internal Database

CloudFormation maintains an **internal mapping** separate from resource tags:

```
Stack: CfnTeleportTest2
├─ LogicalId: Bucket182C536A1
│  └─ PhysicalId: 031700846815-cfn-teleport-test-1
│     Type: AWS::S3::Bucket
│     Status: IMPORT_COMPLETE
│
└─ LogicalId: DynamoDbTable6316879D  
   └─ PhysicalId: cfn-teleport-test
      Type: AWS::DynamoDB::Table
      Status: IMPORT_COMPLETE
```

This is the **source of truth**. CloudFormation uses this database to:
- Know which resources belong to which stack
- Perform drift detection
- Execute updates/deletes
- Show stack resources in console

**Tags are NOT required for CloudFormation to manage a resource.**

---

### CloudFormation's SECONDARY Tracking: Resource Tags

From AWS documentation:
> "CloudFormation automatically creates the following stack-level tags:
> - `aws:cloudformation:logical-id`
> - `aws:cloudformation:stack-id`  
> - `aws:cloudformation:stack-name`
>
> **The propagation of stack-level tags to resources varies by resource type.**"

**Why tags exist**:
1. **User convenience** - You can filter AWS resources by stack name in AWS Console
2. **Cost allocation** - Track costs by CloudFormation stack
3. **External tooling** - Third-party tools can identify which stack created a resource
4. **NOT for CloudFormation's internal tracking** - CloudFormation doesn't need them

**Why tags vary**:
- Some resource types support tags (S3, EC2, Lambda)
- Some don't support tags at all (DynamoDB tables in older APIs)
- Some support tags but CloudFormation doesn't apply them automatically
- **Tags are a courtesy feature, not a requirement**

---

## Why Import/Export Doesn't Touch Tags

### What Import Operation Does

```
Step 1: User removes resource from Source Stack template
Step 2: CloudFormation updates Source Stack:
        - Internal DB: Remove mapping (Stack1 → Resource)
        - Resource: NO CHANGES (DeletionPolicy: Retain)
        
Step 3: User adds resource to Target Stack template  
Step 4: CloudFormation imports to Target Stack:
        - Internal DB: Add mapping (Stack2 → Resource)
        - Resource: NO CHANGES (just reads PhysicalId to verify it exists)
```

**The resource is NEVER modified** - CloudFormation only:
1. Removes the database entry from Stack1
2. Adds a database entry to Stack2
3. Reads the resource's current state to validate it matches the template

**Tags stay exactly as they were** because:
- CloudFormation doesn't update the resource
- The internal database is sufficient for tracking
- Tags are optional metadata, not required for management

---

## Why RefactorStacks DOES Touch Tags

### What RefactorStacks Operation Does

From the AWS CLI documentation output we fetched:

```json
{
    "Action": "MOVE",
    "Entity": "RESOURCE",
    "PhysicalResourceId": "MyTestFunction",
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
            "Value": "arn:aws:cloudformation:...stack/Stack2/..."
        }
    ],
    "UntagResources": [
        "aws:cloudformation:stack-name",
        "aws:cloudformation:logical-id",
        "aws:cloudformation:stack-id"
    ]
}
```

**RefactorStacks performs BOTH operations**:
1. **Internal DB**: Updates the mapping (same as import)
2. **Resource Tags**: Removes old tags, adds new tags

**Why does RefactorStacks update tags?**

This is the key question. Let me think through possible reasons:

### Theory 1: Consistency (Most Likely)

RefactorStacks is a **newer, more complete API**. AWS decided that when you refactor:
- The internal database should reflect the new stack ✅
- The resource tags should ALSO reflect the new stack ✅
- Everything should be consistent

Import/Export was the old way - it only updated the database because:
- It was a manual multi-step process
- Each step was independent
- Tag updates weren't considered critical

RefactorStacks is the modern way - it's atomic and complete:
- Single operation
- Updates everything
- Leaves resources in a consistent state

### Theory 2: Auditability

When you look at a resource in AWS Console:
- With Import/Export: Tags say "Stack1" but CloudFormation says "Stack2" → Confusing
- With RefactorStacks: Tags say "Stack2" and CloudFormation says "Stack2" → Clear

For compliance/auditing, having correct tags matters:
- Cost allocation reports use tags
- Security scanning tools use tags
- Tag-based IAM policies use tags

### Theory 3: Future-Proofing

AWS might be planning features that rely on tags being correct:
- Cross-account resource management
- Tag-based drift detection
- Integration with other AWS services

---

## The KeyPair Problem - Now It Makes Sense

### Why Import Can Handle KeyPairs

```
Import Operation:
1. Internal DB: Remove KeyPair from Stack1 ✅
2. Resource: DO NOTHING ✅ (no tag updates needed)
3. Internal DB: Add KeyPair to Stack2 ✅
4. Resource: DO NOTHING ✅ (no tag updates needed)

Result: Success! KeyPair never touched, only database updated
```

### Why RefactorStacks Cannot Handle KeyPairs

```
RefactorStacks Operation:
1. Internal DB: Update mapping Stack1 → Stack2 ✅
2. Resource: UntagResources ["aws:cloudformation:stack-name"] ❌
   
   ERROR: AWS::EC2::KeyPair Tags property requires "Replacement"
   
   To update tags, CloudFormation must:
   - DELETE KeyPair (loses private key)
   - CREATE new KeyPair (generates new key)
   - Result: EC2 instances can't connect
   
   CloudFormation: "NOPE, rejecting this operation"
```

**RefactorStacks is MORE strict because it's MORE complete** - it wants to update tags for consistency, but KeyPairs don't allow tag updates without replacement.

---

## The Full Picture

```
CloudFormation Resource Management:

┌─────────────────────────────────────────────────────────┐
│                  CloudFormation Service                  │
│                                                          │
│  ┌──────────────────────────────────────────┐          │
│  │     Internal Database (Source of Truth)   │          │
│  │                                           │          │
│  │  Stack1:                                  │          │
│  │    - Bucket1 → s3://mybucket1            │          │
│  │    - Table1  → dynamo://mytable          │          │
│  │                                           │          │
│  │  Stack2:                                  │          │
│  │    - Lambda1 → function/myfunc           │          │
│  │                                           │          │
│  └──────────────────────────────────────────┘          │
│                      ▲                                   │
│                      │                                   │
│              PRIMARY TRACKING                            │
│              (Always accurate)                           │
│                                                          │
└─────────────────────────────────────────────────────────┘
                       │
                       │ CloudFormation applies tags
                       │ (when resource type supports it)
                       ▼
┌─────────────────────────────────────────────────────────┐
│                    AWS Resources                         │
│                                                          │
│  S3 Bucket: mybucket1                                    │
│    Tags:                                                 │
│      ✅ aws:cloudformation:stack-name = Stack1          │
│      ✅ aws:cloudformation:stack-id = arn:...           │
│                                                          │
│  DynamoDB Table: mytable                                 │
│    Tags:                                                 │
│      ❌ (no aws:cloudformation:* tags)                  │
│    → CloudFormation still manages it via internal DB!   │
│                                                          │
│  EC2 KeyPair: mykeypair                                  │
│    Tags:                                                 │
│      ⚠️  CANNOT BE UPDATED without Replacement          │
│    → RefactorStacks cannot change tags                   │
│    → Import/Export works (doesn't try to change tags)   │
│                                                          │
└─────────────────────────────────────────────────────────┘
              ▲
              │
        SECONDARY TRACKING
        (Optional, varies by resource)
        (Used for: cost allocation, filtering, external tools)
```

---

## Summary - Answers to Your Questions

### 1. "Why are we talking about tag updates in the first place?"

Because **RefactorStacks updates both the internal database AND resource tags**, while Import/Export only updates the internal database.

### 2. "What tags need updating when we move resources?"

The `aws:cloudformation:*` tags:
- `aws:cloudformation:stack-name` 
- `aws:cloudformation:stack-id`
- `aws:cloudformation:logical-id`

These tags are **optional metadata** that CloudFormation applies to help users filter/track resources, NOT for CloudFormation's own tracking (internal DB handles that).

### 3. "Why does tag change happen automatically?"

**With RefactorStacks**: AWS decided the modern API should update everything for consistency
**With Import/Export**: Tags DON'T change automatically - they stay as-is

### 4. "Why does the tag not get updated when we import shit in the target stack?"

Because **Import is a read-only operation from the resource's perspective** - CloudFormation only updates its internal database, it doesn't touch the actual resource or its tags.

---

## The Real Difference

**Import/Export** = "Let me take over managing this existing resource without touching it"
- Pro: Works with any resource (no modifications needed)
- Con: Tags stay stale, can fail mid-operation and orphan resources

**RefactorStacks** = "Let me properly move this resource with full consistency"  
- Pro: Updates everything, atomic operation, no orphaned resources
- Con: Requires ability to update tags → fails on resources where tags need Replacement

---

## Why This Matters for cfn-teleport

We're replacing Import/Export with RefactorStacks to gain **atomicity and safety**, but we lose support for resources like KeyPairs where tags require Replacement.

**This is the correct trade-off** - better to fail fast with a clear error than to risk orphaning resources.
