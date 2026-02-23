# Current State Analysis: Dependency Markers in Resource Selection

## Overview
This analysis examines the current resource selection UI in cfn-teleport to understand how to add visual markers for resource dependencies and output references.

## Current Implementation

### Resource Selection Flow
1. **User Journey**: User selects source stack → target stack → resources to move
2. **Selection UI**: Uses `dialoguer::MultiSelect` with custom formatting
3. **Location**: `select_resources()` function at line 565 in `src/main.rs`

### Current Display Format
The `format_resources()` function (line 685) creates formatted strings for each resource:

**Without renaming:**
```
<ResourceType>  <LogicalID>  <PhysicalID>
```

**With renaming (when displaying selected resources with new IDs):**
```
<ResourceType>  <LogicalID> ► <NewLogicalID>   <PhysicalID>
```

Example output:
```
AWS::S3::Bucket       MyBucket       my-bucket-physical-id
AWS::DynamoDB::Table  MyTable        my-table-name
```

### Existing Dependency Analysis
The codebase already has comprehensive dependency tracking:

1. **Reference Detection** (`reference_updater.rs`):
   - `find_all_references()` (line 39): Scans template and returns `HashMap<String, HashSet<String>>`
   - Key = resource ID that contains references
   - Value = Set of resource IDs being referenced
   - Detects: `Ref`, `Fn::GetAtt`, `Fn::Sub`, `DependsOn`
   - Special handling for "Outputs" section

2. **Validation** (`validate_move_references()`, line 598):
   - Checks for dangling references before moving resources
   - Validates that resources staying behind don't reference moving resources
   - Validates that outputs don't reference moving resources
   - Currently runs AFTER selection (validation phase)

### Key Data Available
From `find_all_references()`, we can determine:
- **Resources with dependencies**: Resources that reference other resources
- **Resources being depended on**: Resources that are referenced by others
- **Output references**: Resources referenced by stack outputs

## Integration Points

### 1. Format Resources Function
- **Current signature**: `format_resources(resources, resource_id_map) -> Vec<String>`
- **Returns**: Vector of formatted strings for display
- **Uses columnar layout** with dynamic width calculation
- **Already handles optional markers** (► for renaming)

### 2. Select Resources Function
- **Receives**: Slice of `StackResourceSummary` references
- **Calls**: `format_resources()` to create display items
- **Passes to**: `MultiSelect` widget from dialoguer crate

### 3. Template Access
- To analyze dependencies, we need the CloudFormation template
- **Current problem**: `select_resources()` doesn't have access to the template
- **Solution needed**: Pass template to selection function or pre-compute dependency info

## Technical Constraints

1. **Display Format**: 
   - dialoguer's `MultiSelect` expects `Vec<String>`
   - Each string is one selectable item
   - Must use fixed-width formatting or single-line markers

2. **Dependency Analysis Timing**:
   - Currently validation happens AFTER selection
   - To show markers, we need to analyze BEFORE selection
   - Performance consideration: `find_all_references()` scans entire template

3. **Information Direction**:
   - Need to show: "This resource IS depended on by others" (incoming)
   - Need to show: "This resource depends on others" (outgoing)
   - Need to show: "This resource is referenced in outputs"

## Existing Patterns to Follow

1. **Symbol Usage**: Already uses `►` for renaming indicator
2. **Columnar Layout**: Uses fixed-width formatting with padding
3. **Error Handling**: Comprehensive validation with detailed messages
4. **User Experience**: Clear, informative prompts and confirmations

## Current User Pain Points

Without visual markers:
1. Users may select resources that will fail validation
2. No indication which resources are "safe" to move independently
3. Users discover dependency issues only after making selections
4. Need to understand template structure to make informed choices

## Assumptions

1. Source stack template is available when selecting resources (needs verification)
2. Performance of `find_all_references()` is acceptable for pre-selection analysis
3. Single-character markers are sufficient (no multi-line or complex symbols needed)
4. Users will understand marker meanings (needs legend/help text)

## Questions to Clarify

1. Should markers show both incoming and outgoing dependencies, or just incoming?
2. Should we combine markers (e.g., resource both depended on AND in outputs)?
3. Where should marker legend/help be displayed?
4. Should dangerous selections be prevented or just warned?

## Next Steps

This analysis provides the foundation for:
1. Defining specific marker symbols and their meanings
2. Designing the enhanced display format
3. Planning the implementation approach
4. Creating user documentation
