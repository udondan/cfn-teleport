# Feature Specification: Resource Dependency Markers

## Overview
Enhance the resource selection UI to display visual markers indicating dependency relationships and output references, helping users make informed decisions and avoid validation errors before selecting resources to move or rename.

## User Story
**As a** CloudFormation stack operator  
**I want to** see which resources have dependencies or are referenced by outputs during the selection process  
**So that** I can make informed decisions and avoid errors before attempting to move resources between stacks.

## Context
Currently, users select resources for migration and only discover dependency issues during validation (after selection). This creates a trial-and-error workflow. The codebase already tracks all resource references internally, but this information isn't surfaced during selection.

## Requirements

### Functional Requirements

#### FR1: Display Dependency Markers
The resource selection list shall display visual markers next to each resource indicating:
- **Has incoming dependencies**: Other resources in the stack reference this resource
- **Referenced by outputs**: Stack outputs reference this resource
- **No markers**: Resource is not referenced by anything (safe to move independently)

**Markers shall use CLI-safe emoji symbols:**
- 🔗 (link) - Resource has incoming dependencies (other resources depend on this)
- 📤 (outbox tray) - Resource is referenced by stack outputs
- Both markers displayed when both conditions apply

#### FR2: Conditional Legend Display
A legend explaining marker meanings shall be displayed:
- **When**: Before the resource selection list
- **Only if**: The source stack contains at least one resource with markers
- **Content**: Only shows legend entries for markers actually present in the list

Legend format:
```
Dependency Markers:
  🔗 - Resource is referenced by other resources in the stack
  📤 - Resource is referenced by stack outputs
```

#### FR3: Context-Aware Selection Validation
Selection behavior shall depend on operation type:

**Cross-Stack Move (source ≠ target):**
- **Block selection**: Resources marked with 📤 (referenced by outputs) cannot be selected
- **Reason**: Outputs cannot be moved between stacks; would create invalid references
- **User feedback**: Display error message explaining why selection is invalid

**Same-Stack Rename (source = target):**
- **Allow selection**: Resources with any markers (🔗 or 📤) can be selected
- **Reason**: Renaming within same stack allows automatic reference updates
- **No restriction**: All resources are selectable

#### FR4: Display Format Integration
Markers shall integrate with existing columnar display format:
- Position: After the ResourceType column, before LogicalID
- Fixed width: 2 characters (1 emoji + 1 space, or 2 spaces if no marker)
- Alignment: Consistent with existing column layout
- No breaking: Existing display features (renaming indicator ►) remain functional

Example display:
```
Dependency Markers:
  🔗 - Resource is referenced by other resources in the stack
  📤 - Resource is referenced by stack outputs

? Select resources to move:
  AWS::S3::Bucket          🔗 MyBucket          my-bucket-physical-id
  AWS::DynamoDB::Table     📤 MyTable           my-table-name
  AWS::Lambda::Function    🔗📤 MyFunction      my-function-name
  AWS::IAM::Role              IndependentRole  role-xyz
```

#### FR5: Pre-Selection Dependency Analysis
Before displaying the resource selection UI:
- Analyze the source stack template to identify all resource references
- Identify resources referenced by outputs
- Identify resources referenced by other resources
- Compute marker display for each resource

### Non-Functional Requirements

#### NFR1: Performance
- Dependency analysis shall complete within 2 seconds for stacks with up to 500 resources
- Analysis shall not block or delay stack listing

#### NFR2: Usability
- Emoji symbols shall render correctly on common terminal emulators (iTerm2, Terminal.app, Windows Terminal, etc.)
- Markers shall be visually distinct and easy to understand
- Legend shall be concise and placed immediately before selection UI

#### NFR3: Compatibility
- Feature shall work with existing validation logic
- Existing error messages for dependency violations remain unchanged
- No changes to non-interactive mode behavior (if applicable)

#### NFR4: Maintainability
- Reuse existing `find_all_references()` function from `reference_updater.rs`
- Extend existing `format_resources()` function rather than replacing it
- Follow existing code style and patterns

## Success Criteria

### SC1: Correct Marker Display
- ✅ Resources with incoming dependencies show 🔗
- ✅ Resources referenced by outputs show 📤
- ✅ Resources with both conditions show both markers
- ✅ Independent resources show no markers

### SC2: Context-Aware Validation
- ✅ Cross-stack moves block selection of 📤 marked resources
- ✅ Same-stack renames allow selection of all resources regardless of markers
- ✅ Clear error messages when selection is blocked

### SC3: Conditional Legend
- ✅ Legend displays only when markers are present
- ✅ Legend shows only entries for markers visible in the list
- ✅ Legend does not appear when no resources have markers

### SC4: User Experience
- ✅ Users can identify risky selections before attempting them
- ✅ Reduction in validation errors after selection
- ✅ No disruption to existing workflows

## Out of Scope

- **Preventing selection of resources with outgoing dependencies**: Resources that depend on others can still be selected (existing validation will catch issues)
- **Dependency graph visualization**: Only simple markers, no complex dependency trees
- **Automatic dependency resolution**: No automatic selection of dependent resources
- **Configuration options**: Marker symbols are fixed (no user customization)
- **Marker colors**: Uses default terminal emoji colors only
- **Multi-level dependency tracking**: Only direct references, not transitive dependencies

## Assumptions

1. Source stack template is available before resource selection
2. `find_all_references()` performance is acceptable for typical stack sizes
3. Users' terminal emulators support emoji rendering
4. Existing validation logic remains unchanged
5. Operation type (cross-stack vs same-stack) can be determined from source and target stack names

## Dependencies

- Existing: `reference_updater::find_all_references()`
- Existing: `format_resources()` function
- Existing: `select_resources()` function
- Existing: Template retrieval mechanism

## Scope Clarifications

### Dependency Analysis Scope
**Decision**: Analyze source stack only, not target stack.

**Rationale**: 
- Primary goal is preventing source stack breakage
- Target stack conflict detection can be a future enhancement
- Keeps initial implementation simpler and focused
- Existing validation may already catch target stack issues during change set application

## Acceptance Test Scenarios

### Scenario 1: Resource with Incoming Dependencies (Cross-Stack)
**Given**: Stack A has resources R1 (S3 bucket) and R2 (Lambda) where R2 references R1  
**When**: User selects Stack A as source, Stack B as target, and views resource list  
**Then**: R1 displays 🔗 marker, R2 displays no marker  
**And**: Legend shows: "🔗 - Resource is referenced by other resources in the stack"  
**And**: R1 can be selected (dependency warning is for R2 staying behind)

### Scenario 2: Resource Referenced by Output (Cross-Stack)
**Given**: Stack A has resource R1 (S3 bucket) referenced by an output  
**When**: User selects Stack A as source, Stack B as target, and views resource list  
**Then**: R1 displays 📤 marker  
**And**: Legend shows: "📤 - Resource is referenced by stack outputs"  
**And**: R1 cannot be selected (selection is blocked)  
**And**: Error message explains outputs cannot be moved

### Scenario 3: Same-Stack Rename
**Given**: Stack A has resource R1 referenced by output  
**When**: User selects Stack A as both source and target, and views resource list  
**Then**: R1 displays 📤 marker  
**And**: R1 can be selected (same-stack rename is allowed)

### Scenario 4: No Dependencies
**Given**: Stack A has only independent resources  
**When**: User views resource selection list  
**Then**: No markers are displayed  
**And**: No legend is displayed  
**And**: All resources can be selected

### Scenario 5: Combined Markers
**Given**: Stack A has resource R1 referenced by both another resource and an output  
**When**: User views resource selection list for cross-stack move  
**Then**: R1 displays both 🔗📤 markers  
**And**: Legend shows both entries  
**And**: R1 cannot be selected (blocked due to output reference)

## Glossary

- **Incoming dependency**: Another resource references this resource (e.g., via Ref, GetAtt, Sub)
- **Outgoing dependency**: This resource references another resource
- **Output reference**: Stack output section references this resource
- **Cross-stack move**: Moving resources from Stack A to Stack B (A ≠ B)
- **Same-stack rename**: Renaming resources within Stack A (source = target = A)
