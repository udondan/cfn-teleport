# Development Plan: cfn-teleport (dependencmarker branch)

*Generated on 2026-02-22 by Vibe Feature MCP*
*Workflow: [sdd-feature](https://mrsimpson.github.io/responsible-vibe-mcp/workflows/sdd-feature)*

## Goal
Add visual markers to the resource selection UI that indicate:
1. Resources that other resources depend on (incoming dependencies)
2. Resources that depend on other resources (outgoing dependencies)
3. Resources that are referenced by stack outputs
4. Resources that depend on stack parameters

This will help users make informed decisions during resource selection and avoid validation errors.

## Key Decisions
1. **Marker Symbols**: Use emoji arrows in TWO groups with max 2 emojis total:
   - **Left/Right**: ⬅️ (incoming), ➡️ (outgoing), ↔️ (bidirectional resource deps)
   - **Up/Down**: ⬆️ (outputs), ⬇️ (parameters), ↕️ (both)
2. **Selection Blocking**: Block output-referenced resources for cross-stack moves only, allow for same-stack renames
3. **Conditional Legend**: Show legend only when markers are present, only show entries for visible markers
4. **Display Position**: Markers go in separate column after ResourceType, 4 chars total (max 2 emojis)
5. **Analysis Scope**: Source stack only (not target stack) - keeps implementation focused on preventing source breakage
6. **Four-way Dependencies**: Track incoming (⬅️), outgoing (➡️), outputs (⬆️), and parameters (⬇️)
7. **Bidirectional Arrows**: Use ↔️ for resource deps, ↕️ for output+parameter
8. **Compact Display**: Maximum 2 emojis per resource keeps it readable and aligned
9. **Parameter Detection**: Extract parameter names from template Parameters section, check if referenced
10. **Outgoing vs Parameter**: ➡️ shows ONLY for resource-to-resource dependencies; parameters show only ⬇️ (mutually exclusive)

## Notes
- The codebase already has comprehensive dependency tracking via `find_all_references()` in `reference_updater.rs`
- Current validation happens AFTER selection; markers will shift some feedback BEFORE selection
- Display uses dialoguer's MultiSelect with fixed-width columnar formatting
- All tasks are ordered sequentially with clear dependencies noted
- Tasks T001-T003 can be done in parallel (marked [P]) as they create independent functions
- Integration tasks (T011-T012) must be done after core functionality is complete
- Testing tasks (T013-T020) should be done after all implementation tasks

## Analyze
### Tasks
- [x] Analyze current resource selection UI implementation
- [x] Review existing dependency detection code
- [x] Identify integration points for adding markers
- [x] Document current display format and constraints
- [x] Create current-state-analysis.md

### Completed
- [x] Created development plan file
- [x] Analyzed format_resources() function and display format
- [x] Reviewed reference_updater.rs dependency tracking
- [x] Identified technical constraints and integration points

## Specify

### Phase Entrance Criteria:
- [x] Current behavior and context have been analyzed and documented
- [x] The problem/opportunity is clearly understood
- [x] User needs and requirements are identified

### Tasks
- [x] Document functional requirements (marker display, legend, validation)
- [x] Define marker symbols and meanings
- [x] Specify context-aware behavior (cross-stack vs same-stack)
- [x] Define success criteria and acceptance tests
- [x] Document out of scope items
- [x] Clarify open question: Should we analyze target stack dependencies? (Decision: Source only)

### Completed
- [x] Created spec.md with complete functional and non-functional requirements
- [x] Defined 5 acceptance test scenarios
- [x] Specified emoji markers: 🔗 (dependencies) and 📤 (outputs)
- [x] Resolved scope question: source stack analysis only

## Clarify

### Phase Entrance Criteria:
- [x] Functional and non-functional requirements are documented
- [x] Success criteria are defined
- [x] Scope boundaries are established

### Tasks
- [x] Review specification for remaining [NEEDS CLARIFICATION] markers (none found)
- [x] Validate all functional requirements are testable
- [x] Verify success criteria are measurable
- [x] Ensure user scenarios are complete (5 acceptance test scenarios documented)
- [x] Confirm no implementation details leaked into spec
- [x] Validate specification alignment with existing system

### Completed
- [x] Specification review complete - no remaining clarifications needed
- [x] All requirements are testable and technology-agnostic
- [x] 5 detailed acceptance test scenarios cover all key use cases
- [x] Specification ready for implementation planning

## Plan

### Phase Entrance Criteria:
- [x] All ambiguities and open questions are resolved
- [x] Technical approach is clarified
- [x] Stakeholder alignment is achieved

### Tasks
- [x] Review specification and analysis documents
- [x] Identify integration points with existing architecture
- [x] Plan data structures (ResourceDependencyInfo)
- [x] Design function signatures and API changes
- [x] Plan implementation phases (0: Prep, 1: Display, 2: Selection, 3: Integration, 4: Testing)
- [x] Document technical decisions and rationale
- [x] Identify dependencies and risks
- [x] Create rollout strategy

### Completed
- [x] Created comprehensive plan.md with 4 implementation phases
- [x] Defined data structures and function signatures
- [x] Documented 5 technical decisions with rationale
- [x] Identified all integration points (no new dependencies needed)
- [x] Planned backward-compatible API extensions

## Tasks

### Phase Entrance Criteria:
- [x] High-level implementation plan is documented
- [x] Technical design decisions are made
- [x] Architecture approach is defined

### Setup Phase: Foundation

#### T001: Define ResourceDependencyInfo struct [P]
**File**: `src/main.rs` (after imports, before main)  
**What**: Create struct to hold dependency analysis results
```rust
struct ResourceDependencyInfo {
    has_incoming_deps: bool,
    referenced_by_outputs: bool,
}
```
**Acceptance**: Struct compiles, follows Rust naming conventions

#### T002: Implement compute_dependency_markers() function [P]
**File**: `src/main.rs`  
**What**: Create function that analyzes template and returns HashMap of dependency info
**Signature**:
```rust
fn compute_dependency_markers(
    resources: &[&cloudformation::types::StackResourceSummary],
    template: &serde_json::Value,
) -> HashMap<String, ResourceDependencyInfo>
```
**Logic**:
- Call `reference_updater::find_all_references(template)`
- For each resource, check if logical_id appears in any reference set
- Check if logical_id appears in "Outputs" references
- Return HashMap mapping logical_id to dependency info
**Acceptance**: Function compiles, handles empty templates, identifies dependencies correctly

#### T003: Implement generate_legend() function [P]
**File**: `src/main.rs`  
**What**: Create function that generates conditional legend text
**Signature**:
```rust
fn generate_legend(dependency_info: &HashMap<String, ResourceDependencyInfo>) -> Option<String>
```
**Logic**:
- Scan dependency_info values to see if any have has_incoming_deps or referenced_by_outputs
- If none, return None
- Build legend string with only present markers
- Format: "Dependency Markers:\n  🔗 - Resource is referenced by other resources in the stack\n  📤 - Resource is referenced by stack outputs"
**Acceptance**: Returns None for empty info, returns correct legend for each marker type

### User Story P1: Display Dependency Markers

#### T004: Extend format_resources() signature
**File**: `src/main.rs:685`  
**What**: Add optional dependency_info parameter to function signature
**New Signature**:
```rust
async fn format_resources(
    resources: &[&cloudformation::types::StackResourceSummary],
    resource_id_map: Option<HashMap<String, String>>,
    dependency_info: Option<&HashMap<String, ResourceDependencyInfo>>,
) -> Result<Vec<String>, io::Error>
```
**Acceptance**: Function compiles with new parameter, backward compatible (None allowed)

#### T005: Add marker column to format_resources() display logic
**File**: `src/main.rs:685` (inside format_resources function)  
**What**: Generate marker string and add column to display format
**Changes**:
- For each resource, lookup dependency info by logical_id
- Generate marker string:
  - Both flags true: "🔗📤"
  - has_incoming_deps only: "🔗 "
  - referenced_by_outputs only: "📤 "
  - Neither: "  "
- Update format string to include marker column after resource_type
- Adjust column widths and padding
**Acceptance**: Display shows markers correctly, maintains alignment, handles all combinations

#### T006: Update format_resources() call at line 283 with None parameter
**File**: `src/main.rs:283`  
**What**: Add `None` for dependency_info parameter to maintain backward compatibility
**Change**: `format_resources(&selected_resources, Some(new_logical_ids_map.clone()), None).await?`
**Acceptance**: Compiles, existing functionality unchanged

### User Story P2: Conditional Legend Display

#### T007: Add legend display to select_resources()
**File**: `src/main.rs:565`  
**What**: Call generate_legend() and print legend if present
**Dependencies**: Requires T002 (compute_dependency_markers) and T003 (generate_legend)  
**Changes**:
- Compute dependency markers from template
- Call generate_legend()
- If Some(legend), println!("{}\n", legend) before MultiSelect
**Acceptance**: Legend displays when markers present, hidden when no markers

### User Story P3: Context-Aware Selection Validation

#### T008: Extend select_resources() signature with new parameters
**File**: `src/main.rs:565`  
**What**: Add template and is_cross_stack parameters
**New Signature**:
```rust
async fn select_resources<'a>(
    prompt: &str,
    resources: &'a [&aws_sdk_cloudformation::types::StackResourceSummary],
    template: &serde_json::Value,
    is_cross_stack: bool,
) -> Result<Vec<&'a aws_sdk_cloudformation::types::StackResourceSummary>, Box<dyn Error>>
```
**Acceptance**: Function compiles with new parameters

#### T009: Integrate dependency analysis into select_resources()
**File**: `src/main.rs:565`  
**What**: Compute markers, generate legend, pass to format_resources
**Dependencies**: Requires T002, T003, T005, T008  
**Changes**:
- Call compute_dependency_markers(resources, template)
- Call generate_legend(&dependency_info)
- Print legend if Some
- Pass Some(&dependency_info) to format_resources()
**Acceptance**: Markers and legend display correctly in selection UI

#### T010: Implement cross-stack selection blocking
**File**: `src/main.rs:565` (after MultiSelect interaction)  
**What**: Validate selected resources and block if output-referenced in cross-stack mode
**Dependencies**: Requires T009  
**Logic**:
- After MultiSelect returns indices
- If is_cross_stack == true:
  - For each selected index, get resource logical_id
  - Check if dependency_info[logical_id].referenced_by_outputs == true
  - If true, collect resource info (type, logical_id)
- If any blocked resources, return Err with descriptive message
- Message format:
  ```
  Cannot select the following resources for cross-stack move because they are referenced by stack outputs:
    - ResourceName (AWS::ResourceType)
  
  Outputs cannot be moved between stacks. Consider:
    - Removing or updating the outputs before moving
    - Using same-stack rename instead (references will be updated automatically)
  ```
**Acceptance**: Cross-stack moves block output-referenced resources, same-stack allows all, error message is clear

### Integration Phase: Connect with Main Flow

#### T011: Update main() to retrieve template before selection
**File**: `src/main.rs` (main function, before select_resources call)  
**What**: Get source template and determine operation type
**Changes**:
- After target stack selection
- Call `let source_template = get_template(&client, &source_stack_name).await?;`
- Calculate `let is_cross_stack = source_stack_name != target_stack_name;`
**Acceptance**: Template retrieved successfully, operation type determined correctly

#### T012: Update select_resources() call site with new parameters
**File**: `src/main.rs` (main function, select_resources call)  
**What**: Pass template and is_cross_stack to select_resources()
**Dependencies**: Requires T008, T011  
**Change**:
```rust
let selected_resources = select_resources(
    "Select resources to move:",
    &resource_refs[..],
    &source_template,
    is_cross_stack,
).await?;
```
**Acceptance**: Compiles, selection flow works with new parameters

### Testing Phase: Validation

#### T013: Manual test - Stack with no dependencies
**What**: Test with a stack containing only independent resources  
**Expected**: No markers displayed, no legend shown, all resources selectable  
**Stack**: Create or use test stack without cross-references

#### T014: Manual test - Stack with resource dependencies
**What**: Test with a stack where resources reference each other  
**Expected**: 🔗 markers shown for referenced resources, legend displays dependency entry

#### T015: Manual test - Stack with output references
**What**: Test with a stack that has outputs referencing resources  
**Expected**: 📤 markers shown for output-referenced resources, legend displays output entry

#### T016: Manual test - Cross-stack move blocking
**What**: Attempt cross-stack move of output-referenced resource  
**Expected**: Selection blocked with clear error message

#### T017: Manual test - Same-stack rename allowing
**What**: Attempt same-stack rename of output-referenced resource  
**Expected**: Selection allowed (no blocking)

#### T018: Manual test - Combined markers
**What**: Test resource that is both referenced by others AND by outputs  
**Expected**: Both markers shown (🔗📤), legend shows both entries

#### T019: Manual test - Emoji rendering
**What**: Verify emoji display on different terminal emulators  
**Terminals**: iTerm2, Terminal.app (macOS), Windows Terminal (if available)  
**Expected**: Emojis render correctly, no character corruption

#### T020: Code quality check
**What**: Run linting and formatting checks  
**Commands**: `make lint` (runs clippy + rustfmt check)  
**Expected**: No warnings or errors, code follows style guidelines

### Completed
- [x] Created 20 detailed implementation tasks with clear acceptance criteria
- [x] Organized tasks by phase: Setup, User Stories (P1-P3), Integration, Testing
- [x] Marked parallelizable tasks (T001-T003)
- [x] Documented dependencies between tasks
- [x] Each task specifies file location, signature, logic, and acceptance criteria

## Implement

### Phase Entrance Criteria:
- [x] Detailed task list is created and prioritized
- [x] Each task has clear acceptance criteria
- [x] Dependencies between tasks are identified

### Tasks
- [x] T001: Define ResourceDependencyInfo struct
- [x] T002: Implement compute_dependency_markers() function
- [x] T003: Implement generate_legend() function
- [x] T004: Extend format_resources() signature
- [x] T005: Add marker column to format_resources() display logic
- [x] T006: Update format_resources() call at line 289 with None parameter
- [x] T007: Add legend display to select_resources()
- [x] T008: Extend select_resources() signature with new parameters
- [x] T009: Integrate dependency analysis into select_resources()
- [x] T010: Implement cross-stack selection blocking
- [x] T011: Update main() to retrieve template before selection
- [x] T012: Update select_resources() call site with new parameters
- [ ] T013-T020: Testing tasks (manual tests and code quality)

### Completed
- [x] All core functionality implemented (T001-T012)
- [x] Added parameter dependency detection (⬆️ marker)
- [x] Updated ResourceDependencyInfo struct with depends_on_parameters field
- [x] Updated compute_dependency_markers() to extract parameter names and detect references
- [x] Fixed outgoing dependency detection: only show ⬅️ for actual resource references, not parameters
- [x] **REFACTORED to 2-emoji system**: Left/Right (➡️⬅️↔️) + Up/Down (⬇️⬆️↕️)
- [x] **Dynamic spacing**: Calculates max emoji count, only adds spacing when markers exist
- [x] **Swapped emoji directions**: ➡️=incoming, ⬅️=outgoing, ⬇️=outputs, ⬆️=parameters
- [x] **Compact legend**: 2 lines max with category labels ("Resource dependencies", "Stack interface")
- [x] Code compiles without errors or warnings (cargo clippy passed)
- [x] Code formatted with cargo fmt
- [x] T020: Code quality check passed (make lint)
- [x] All four dependency types tracked with semantic directional arrows
- [x] Ready for PR and CI testing



---
*This plan is maintained by the LLM. Tool responses provide guidance on which section to focus on and what tasks to work on.*
