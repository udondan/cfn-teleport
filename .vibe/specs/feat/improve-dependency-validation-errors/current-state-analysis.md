# Current State Analysis: Dependency Validation Error Handling

## Problem Statement

When users attempt to move CloudFormation resources between stacks where the moving resources have **unresolved dependencies** (i.e., they depend on resources that are NOT being moved), cfn-teleport shows a generic CloudFormation `ValidationError` instead of a clear, actionable error message explaining the dependency issue.

### Example Error (Current Behavior)
```
The following resources will be moved from stack CfnTeleportTest1 to CfnTeleportTest2:
  AWS::EC2::Instance InstanceC1063A87  i-0c1288c98ff845af2
Please confirm your selection: yes

Target template validation failed: unhandled error (ValidationError)
```

### Desired Behavior
The tool should detect and explain the specific dependency issue, e.g.:
```
Cannot move resources due to unresolved dependencies:

  - Resource 'Instance' depends on 'SecurityGroup' which is not being moved
  - Resource 'Instance' depends on 'Role' which is not being moved

Either move all dependent resources together, or remove dependencies before moving.
```

---

## Current Implementation Analysis

### 1. Validation Flow

The tool has TWO distinct operational modes for cross-stack moves:

#### **Refactor Mode** (default, `--mode=refactor`)
- Uses CloudFormation Stack Refactoring API
- Safer and atomic, but supports fewer resource types
- Flow: `refactor_stack_resources_cross_stack()` (lines 1558-1705)
  1. Removes resources from source template
  2. Adds resources to target template
  3. Updates references in both templates
  4. **Validates both templates** (lines 1589-1595)
  5. Creates stack refactor via API
  6. Waits for completion

#### **Import Mode** (`--mode=import`)
- Legacy import/export flow
- Supports more resource types but can orphan resources on failure
- Flow: main function cross-stack branch (lines 367-518)
  1. Validates parameter dependencies (blocks resources with params in import mode)
  2. Creates multiple template variants (retained, removed, target with/without deletion policy)
  3. **Validates all template variants** (lines 463-477)
  4. Updates source stack (remove resources)
  5. Creates import changeset for target stack
  6. Executes changeset

### 2. Current Validation Mechanisms

#### **Reference Validation** (`validate_move_references()` - lines 846-907)
- **Purpose**: Prevents dangling references when resources move
- **What it catches**: 
  - Resources **staying** in source that reference resources **being moved**
  - Outputs referencing moving resources
- **What it DOESN'T catch**: 
  - Resources **being moved** that depend on resources **staying behind**
  - This is the GAP causing the reported issue!

**Example from code:**
```rust
// Problem: referencing resource stays, but referenced resource moves
if !is_referencing_resource_moving && is_referenced_resource_moving {
    errors.push(format!("Resource '{}' references '{}', but only '{}' is being moved...", ...));
}
```

**Missing check:** The inverse case where a moving resource references a staying resource is NOT validated!

#### **Template Validation** (`validate_template()` - lines 1370-1383)
- Calls CloudFormation's `validate_template` API
- Returns generic errors from CloudFormation
- This is where users hit the cryptic `ValidationError`

#### **Dependency Display** (`compute_dependency_markers()` - lines 947-1030)
- Shows visual indicators (emojis) for resource dependencies
- Displays incoming/outgoing dependencies, output references, parameter dependencies
- **Informational only** - doesn't prevent invalid moves

### 3. Reference Detection System

The `reference_updater` module provides:

**`find_all_references(template)` (reference_updater.rs:39-67)**
- Scans Resources and Outputs sections
- Returns map of `referencing_resource -> Set<referenced_resources>`
- Handles: `Ref`, `Fn::GetAtt`, `Fn::Sub`, `DependsOn`

**`collect_references(value, references)` (reference_updater.rs:74-145)**
- Recursive JSON traversal
- Excludes pseudo-parameters (AWS::Region, AWS::AccountId, etc.)
- Extracts resource names from all intrinsic function forms

### 4. Test Infrastructure

From `test/cdk/lib/index.ts`, test stacks include scenarios with:
- **Instance with dependencies**: EC2 Instance depends on SecurityGroup, Role, KeyPair (lines 122-132)
- **DependsOn relationships**: DependentTable explicitly depends on DependencyBucket (lines 200-204)
- **Lambda referencing bucket**: Environment variables reference bucket (lines 228-230)
- **Output references**: Various outputs reference resources (tested for rename validation)

---

## Root Cause Analysis

### The Gap

**Current validation logic** (line 885):
```rust
if !is_referencing_resource_moving && is_referenced_resource_moving {
    // Error: staying resource references moving resource
}
```

**Missing validation** (needed):
```rust
if is_referencing_resource_moving && !is_referenced_resource_moving {
    // Error: moving resource references staying resource
}
```

### Why This Happens

1. **User selects** EC2 Instance to move
2. **Instance depends on** SecurityGroup, Role (both staying in source stack)
3. **validate_move_references()** only checks if *staying* resources reference *moving* resources ❌
4. **Tool proceeds** to create target template with Instance
5. **Target template** references SecurityGroup/Role that don't exist in target stack
6. **CloudFormation validation** fails with generic "ValidationError"
7. **User sees** cryptic error message

### When CloudFormation Validates

- **Refactor mode**: Line 1593-1595 (`validate_template(client, target_final.clone())`)
- **Import mode**: Line 469 in loop validating all template variants

At this point, the template has unresolved references, triggering CloudFormation's validation error.

---

## Information Available for Better Error Messages

When validation fails, we have access to:

1. **All resource references** via `find_all_references()`
2. **Which resources are moving** via `id_mapping.keys()`
3. **Which resources exist in target stack** via target template
4. **Resource types and IDs** from stack summaries
5. **CloudFormation error details** via `ProvideErrorMetadata` trait (line 3)

### What Can Be Extracted

From CloudFormation's error response:
- Error code: `e.code()` 
- Error message: `e.message()`
- May include resource-specific details (depends on error type)

From our internal analysis (pre-validation):
- Exact dependency chain: "Instance → SecurityGroup, Role"
- Missing resources in target
- Resource types for context

---

## Existing Similar Validation Patterns

### Parameter Dependency Check (Import Mode Only)
Lines 373-428 show how the tool blocks resources with parameter dependencies in import mode:

```rust
let depends_on_params: Vec<String> = resource_references
    .iter()
    .filter(|ref_name| parameter_names.contains(*ref_name))
    .map(|s| s.to_string())
    .collect();

if !depends_on_params.is_empty() {
    blocked_resources_with_params.push(format!(
        "- {} (parameters: {})",
        old_id,
        depends_on_params.join(", ")
    ));
}
```

This is a good pattern to follow for the outgoing dependency check!

---

## Constraints and Considerations

### Mode-Specific Behavior

1. **Same-stack rename**: No dependency validation needed (all resources stay in same stack)
2. **Cross-stack refactor mode**: Need outgoing dependency check
3. **Cross-stack import mode**: Need outgoing dependency check (already has parameter check)

### Edge Cases

1. **Circular references**: Moving multiple resources that reference each other (allowed)
2. **Parameters**: Target stack may or may not have matching parameters
3. **Cross-stack references**: Outputs/exports from other stacks (pseudo-valid references)
4. **Pseudo-parameters**: AWS::Region, AWS::AccountId, etc. (always valid)

### False Positives to Avoid

- References to AWS pseudo-parameters (already filtered in `collect_references`)
- References to resources that ARE being moved together
- References to parameters that exist in target stack (hard to detect in advance)

---

## Recommendations for Implementation

### 1. Where to Add Validation

Add outgoing dependency check in **two places**:

- **Refactor mode**: Before line 1588 in `refactor_stack_resources_cross_stack()`
- **Import mode**: After line 333 in main flow (alongside existing `validate_move_references()`)

Alternatively, extend `validate_move_references()` to check both directions.

### 2. What to Validate

For each resource being moved:
- Check what it references (via `find_all_references`)
- Verify all referenced resources are EITHER:
  - Also being moved (in `id_mapping`)
  - Pseudo-parameters (already filtered)
  - Parameters (may need special handling)

### 3. Error Message Format

Follow existing pattern from `validate_move_references()` (lines 897-903):
```rust
let error_message = format!(
    "Cannot move resources due to unresolved dependencies:\n\n{}\n\n\
     Tip: You can move multiple resources together if they reference each other.",
    errors.join("\n")
);
```

### 4. Testing Strategy

Test scenarios (use CDK test stacks):
- Moving Instance alone (should fail: depends on SecurityGroup, Role)
- Moving Instance + SecurityGroup + Role together (should succeed)
- Moving DependentTable alone (should fail: DependsOn DependencyBucket)
- Moving Lambda alone (should fail: references LambdaTargetBucket in environment)

---

## Success Criteria

1. ✅ User gets clear error message identifying exact missing dependencies
2. ✅ Error message suggests moving dependent resources together
3. ✅ Works in both refactor and import modes
4. ✅ Doesn't break existing valid scenarios (multi-resource moves)
5. ✅ Handles all reference types (Ref, GetAtt, Sub, DependsOn)
6. ✅ No false positives from pseudo-parameters or circular refs
