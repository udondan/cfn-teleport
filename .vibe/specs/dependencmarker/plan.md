# Implementation Plan: Resource Dependency Markers

## Overview
This document outlines the implementation strategy for adding visual dependency markers to the resource selection UI in cfn-teleport.

## Integration with Existing Architecture

### Current Architecture Context
- **Language**: Rust 2021
- **CLI Framework**: clap with derive macros
- **UI Library**: dialoguer for interactive prompts
- **Async Runtime**: tokio
- **AWS SDK**: aws-sdk-cloudformation
- **JSON Handling**: serde_json for template manipulation

### Key Integration Points

#### 1. Reference Analysis (Existing)
- **Module**: `src/reference_updater.rs`
- **Function**: `find_all_references(template) -> HashMap<String, HashSet<String>>`
- **Current Use**: Post-selection validation
- **New Use**: Pre-selection marker computation
- **No changes needed**: Function already provides all required data

#### 2. Resource Formatting (Extend)
- **Function**: `format_resources(resources, resource_id_map) -> Vec<String>`
- **Location**: `src/main.rs:685`
- **Current**: Creates columnar display with optional rename indicators
- **Changes**: Add marker column, accept dependency info parameter

#### 3. Resource Selection (Extend)
- **Function**: `select_resources(prompt, resources) -> Vec<StackResourceSummary>`
- **Location**: `src/main.rs:565`
- **Current**: Calls format_resources, displays MultiSelect
- **Changes**: Pass dependency info, add legend, handle blocking

## Implementation Phases

### Phase 0: Preparation & Data Structures

#### Task 0.1: Define Dependency Info Structure
**Goal**: Create data structure to hold dependency analysis results

```rust
// New struct to hold dependency information for a resource
struct ResourceDependencyInfo {
    has_incoming_deps: bool,    // Other resources reference this
    referenced_by_outputs: bool, // Outputs reference this
}
```

**Location**: Add to `src/main.rs` or create new module `src/dependency_markers.rs`  
**Rationale**: Simple struct, no lifetime issues, easy to pass around

#### Task 0.2: Create Marker Generation Function
**Goal**: Function to compute markers for resources

```rust
fn compute_dependency_markers(
    resources: &[StackResourceSummary],
    template: &Value,
) -> HashMap<String, ResourceDependencyInfo>
```

**Logic**:
1. Call `reference_updater::find_all_references(template)`
2. For each resource, check if it appears in any reference set
3. Special handling for "Outputs" key
4. Return map of logical_id -> dependency info

**Location**: `src/main.rs` or `src/dependency_markers.rs`

---

### Phase 1: Display Enhancement

#### Task 1.1: Enhance format_resources()
**Goal**: Add marker column to resource display

**Signature Change**:
```rust
async fn format_resources(
    resources: &[&cloudformation::types::StackResourceSummary],
    resource_id_map: Option<HashMap<String, String>>,
    dependency_info: Option<&HashMap<String, ResourceDependencyInfo>>, // NEW
) -> Result<Vec<String>, io::Error>
```

**Changes**:
1. Add new max_length entry for marker column (fixed at 2)
2. For each resource, lookup dependency info
3. Generate marker string: "", "🔗 ", "📤 ", or "🔗📤"
4. Insert marker column after resource_type, before logical_id
5. Update format string with new width

**Example Output**:
```
AWS::S3::Bucket       🔗 MyBucket          my-bucket-id
AWS::DynamoDB::Table  📤 MyTable           my-table-id
AWS::Lambda::Function    MyFunction       function-id
```

#### Task 1.2: Create Legend Generation Function
**Goal**: Generate conditional legend based on visible markers

```rust
fn generate_legend(dependency_info: &HashMap<String, ResourceDependencyInfo>) -> Option<String>
```

**Logic**:
1. Scan dependency_info to see which markers are present
2. If no markers, return None
3. Build legend string with only present markers
4. Return Some(legend_string)

**Output Format**:
```
Dependency Markers:
  🔗 - Resource is referenced by other resources in the stack
  📤 - Resource is referenced by stack outputs
```

---

### Phase 2: Selection Logic Enhancement

#### Task 2.1: Extend select_resources() Function
**Goal**: Add dependency analysis and legend display

**Changes**:
1. Accept new parameter: `template: &Value`
2. Accept new parameter: `is_cross_stack: bool`
3. Call `compute_dependency_markers(resources, template)`
4. Call `generate_legend(&dependency_info)`
5. If legend exists, print it before MultiSelect
6. Pass dependency_info to format_resources()
7. After selection, validate against blocking rules (Task 2.2)

**Signature**:
```rust
async fn select_resources<'a>(
    prompt: &str,
    resources: &'a [&aws_sdk_cloudformation::types::StackResourceSummary],
    template: &serde_json::Value,  // NEW
    is_cross_stack: bool,          // NEW
) -> Result<Vec<&'a aws_sdk_cloudformation::types::StackResourceSummary>, Box<dyn Error>>
```

#### Task 2.2: Implement Selection Blocking
**Goal**: Prevent selection of output-referenced resources for cross-stack moves

**Logic**:
1. After user makes selection in MultiSelect
2. If `is_cross_stack == true`:
   - For each selected resource
   - Check if `dependency_info[resource].referenced_by_outputs == true`
   - If true, collect error
3. If errors found, return Err with helpful message
4. Otherwise, return Ok with selected resources

**Error Message Format**:
```
Cannot select the following resources for cross-stack move because they are referenced by stack outputs:
  - MyBucket (AWS::S3::Bucket)
  - MyTable (AWS::DynamoDB::Table)

Outputs cannot be moved between stacks. Consider:
  - Removing or updating the outputs before moving
  - Using same-stack rename instead (references will be updated automatically)
```

---

### Phase 3: Integration with Main Flow

#### Task 3.1: Update Main Function Call Sites
**Goal**: Pass new parameters to select_resources()

**Locations to Update**:
1. Main flow after stack selection (around line 140-160 in main.rs)

**Changes**:
1. Retrieve source template: `get_template(client, source_stack_name).await?`
2. Determine if cross-stack: `is_cross_stack = source_stack_name != target_stack_name`
3. Update select_resources call with new parameters

**Example**:
```rust
let source_template = get_template(&client, &source_stack_name).await?;
let is_cross_stack = source_stack_name != target_stack_name;

let selected_resources = select_resources(
    "Select resources to move:",
    &resource_refs[..],
    &source_template,
    is_cross_stack,
).await?;
```

#### Task 3.2: Update Format Resources Call Sites
**Goal**: Update other calls to format_resources() with None for dependency_info

**Locations**:
1. Line 283: Display selected resources with new IDs
2. Any other format_resources() calls

**Change**: Add `None` parameter for backward compatibility

---

### Phase 4: Testing & Validation

#### Task 4.1: Manual Testing Scenarios
**Test Cases**:
1. ✅ Stack with no dependencies → no legend, no markers
2. ✅ Stack with resource dependencies → 🔗 markers shown
3. ✅ Stack with output references → 📤 markers shown
4. ✅ Cross-stack move with output refs → selection blocked
5. ✅ Same-stack rename with output refs → selection allowed
6. ✅ Combined markers (🔗📤) display correctly
7. ✅ Legend shows only visible markers

**Test Stacks**: Use existing CDK test stacks in `test/cdk/`

#### Task 4.2: Integration Test Updates
**Goal**: Add test scenarios to CDK integration tests

**Approach**:
1. Add outputs to test stacks that reference resources
2. Add cross-resource dependencies
3. Test blocked selections return appropriate errors
4. Verify marker display in non-interactive mode (if applicable)

---

## Technical Decisions

### Decision 1: Module Organization
**Choice**: Add functions to `src/main.rs` initially  
**Rationale**: 
- Feature is tightly coupled to resource selection flow
- Avoids premature modularization
- Can refactor to `src/dependency_markers.rs` later if needed

**Alternative Considered**: New module `src/dependency_markers.rs`  
**Rejected Because**: Adds complexity for small feature; single file is simpler

### Decision 2: Marker Display Format
**Choice**: Fixed 2-character column with emojis  
**Rationale**:
- Consistent with existing columnar layout
- Emojis are visually distinct and intuitive
- Fixed width maintains alignment

**Emoji Selection**:
- 🔗 (U+1F517 LINK): Represents connections/dependencies
- 📤 (U+1F4E4 OUTBOX TRAY): Represents outputs/exports

### Decision 3: Selection Blocking Strategy
**Choice**: Return error after selection attempt  
**Rationale**:
- dialoguer's MultiSelect doesn't support conditional disabling
- Error message after attempt provides context and alternatives
- Consistent with existing validation patterns

**Alternative Considered**: Custom selection widget  
**Rejected Because**: Too complex, breaks existing UI patterns

### Decision 4: Performance Optimization
**Choice**: Compute markers once before display, cache in HashMap  
**Rationale**:
- `find_all_references()` only called once per selection
- Lookup by logical_id is O(1)
- Acceptable for typical stack sizes (<500 resources)

### Decision 5: Error Handling
**Choice**: Return descriptive errors, no silent failures  
**Rationale**:
- Consistent with existing error handling patterns
- Users need clear guidance on what went wrong
- Errors include actionable suggestions

---

## Dependencies

### Existing Code Dependencies
- ✅ `reference_updater::find_all_references()` - no changes needed
- ✅ `get_template()` - already exists
- ✅ `format_resources()` - extend with backward compatibility
- ✅ `select_resources()` - extend with new parameters

### External Dependencies
- ✅ dialoguer - already used, no version change needed
- ✅ serde_json - already used for templates
- ✅ No new crates required

---

## Rollout Strategy

### Phase 1: Implementation
1. Implement core functions (Phase 0-1)
2. Integrate with selection flow (Phase 2-3)
3. Manual testing with test stacks

### Phase 2: Testing
1. Test all acceptance scenarios manually
2. Update integration tests if needed
3. Verify emoji rendering on different terminals

### Phase 3: Documentation
1. Update README with feature description
2. Add example screenshots (if applicable)
3. Document marker meanings

---

## Risks & Mitigations

### Risk 1: Emoji Rendering Issues
**Impact**: Medium  
**Likelihood**: Low  
**Mitigation**: 
- Use common, well-supported emojis
- Test on major terminal emulators (iTerm2, Terminal.app, Windows Terminal)
- Document requirements in README

### Risk 2: Performance with Large Stacks
**Impact**: Low  
**Likelihood**: Low  
**Mitigation**:
- `find_all_references()` already used in validation
- Tested with typical stack sizes (<500 resources)
- Can optimize later if needed

### Risk 3: Breaking Changes
**Impact**: High  
**Likelihood**: Very Low  
**Mitigation**:
- Add parameters with backward compatibility
- No changes to external API
- Existing tests should continue passing

---

## Success Metrics

### Implementation Complete When:
- ✅ All 5 acceptance test scenarios pass
- ✅ No regression in existing functionality
- ✅ Code follows existing style guidelines (rustfmt, clippy)
- ✅ Manual testing complete on test stacks
- ✅ Emoji rendering verified on major terminals

### Definition of Done:
- ✅ Code implemented and tested
- ✅ Passes `make lint` (rustfmt + clippy)
- ✅ Passes existing integration tests
- ✅ Manual validation of all scenarios
- ✅ Ready for commit with conventional commit message
