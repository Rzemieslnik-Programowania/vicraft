// Embedded templates written into the project on `vicraft init`.
// These are the source-of-truth for file contents created in .aider/templates/.

pub const ISSUE_TEMPLATE: &str = r#"# <Title: verb + noun, e.g. "Add user authentication">

## Why
<One paragraph: what problem we are solving and why now.>

## What
<Concrete description of the feature from the user's or system's perspective.
Not "how" — that belongs in the spec. Just "what should work".>

## Out of scope
- <Thing 1 we are consciously NOT doing>
- <Thing 2>

## Constraints
- <Technical: "must use existing JWT library", "no new DB tables">
- <Design: "follow existing modal pattern">
- <Other: "must be backward compatible">

## References
- Design: <Figma/Penpot link>
- Related: <Linear issue / PR / ADR link>
- Docs: <link to external documentation>

## Open questions
- <Question that could affect the spec — if any>
"#;

pub const SPEC_TEMPLATE: &str = r#"# Spec: {TASK_TITLE}

**ID:** {TASK_ID}
**Date:** {DATE}
**Status:** draft | review | approved
**Branch:** {BRANCH_NAME}
**Source:** {SOURCE} (.issues/ file or Linear #{ISSUE_ID})

---

## 1. Context and motivation

[Why this task is needed. What problem it solves.]

## 2. Scope

### In scope
- [What we are implementing]

### Out of scope
- [What we are NOT implementing in this task]

## 3. Functional requirements

### 3.1 [Area 1]
- [ ] Requirement 1
- [ ] Requirement 2

### 3.2 [Area 2]
- [ ] Requirement 3

## 4. Non-functional requirements

- **Performance:** [if relevant]
- **Security:** [if relevant]
- **Accessibility (a11y):** [if relevant]
- **Tests:** [required coverage, test types]

## 5. Design

### 5.1 User interface
[Link to Figma/Penpot or UI description — if applicable]

### 5.2 Technical architecture
[Architecture changes, new components, API endpoints]

### 5.3 Data model
[Database schema changes, new entities]

## 6. Acceptance criteria

- [ ] [Specific, measurable condition 1]
- [ ] [Specific, measurable condition 2]

## 7. Dependencies and risks

- **Dependencies on other tasks:** [if any]
- **Risks:** [potential issues]

## 8. Additional context

[Links, notes, design decisions]

## 9. Open questions

> This section is populated by AI during spec generation.
> Answer all questions before proceeding to STEP 2 (plan).
> Delete each question once answered, or fold the answer inline.

<!-- AI: add questions below if anything in the issue is ambiguous,
     underspecified, or could lead to wrong implementation decisions.
     Leave this section empty if everything is clear. -->
"#;

pub const PLAN_TEMPLATE: &str = r#"# Implementation plan: {TASK_TITLE}

**Spec:** {SPEC_FILE}
**Plan date:** {DATE}
**Estimate:** {ESTIMATE}

---

## Approach overview

[Short description of the implementation strategy. Why this approach.]

## Implementation steps

### Step 1: {Name}
**Files to modify:**
- `path/to/file.ext` — [what we change and why]

**Files to create:**
- `path/new_file.ext` — [what it is]

**Description of changes:**
[Detailed description of what and how we implement in this step]

**Tests:**
- [ ] [Specific test to write or verify]

---

### Step 2: {Name}
[same structure]

---

## Step dependencies

[If steps have a required order, explain why]

## Migrations / Breaking changes

[If the plan includes DB migrations, API breaking changes, etc.]

## Rollback strategy

[How to revert changes if something goes wrong]

## Pre-implementation questions

- [ ] [Question to clarify]
"#;

pub const REVIEW_TEMPLATE: &str = r#"# Review: {TASK_TITLE}

**Plan:** {PLAN_FILE}
**Implementation:** {IMPL_FILE}
**Review date:** {DATE}
**Diff base:** {BASE_BRANCH}

---

## Summary of changes

[Short summary of what was implemented]

## Plan verification

### Completed steps
- [x] Step 1: {description} ✓
- [x] Step 2: {description} ✓
- [ ] Step 3: {description} ✗ — [what went wrong or is missing]

## Issues to resolve

### 🔴 Critical (block merge)
- [ ] [Issue 1 — description + suggested fix]

### 🟡 Important (should be fixed)
- [ ] [Issue 2]

### 🟢 Minor (nice to have)
- [ ] [Suggestion]

## Acceptance criteria verification

- [x] [Criterion 1] ✓
- [ ] [Criterion 2] ✗ — [why not met]

## Code notes

[Observations on code quality, conformance with project conventions]

## Status

- [ ] **Requires changes** → go to STEP 5
- [ ] **Approved** → proceed to commit/PR
"#;

pub const CONVENTIONS_SKELETON: &str = r#"# Project Conventions
# File: .aider/CONVENTIONS.md
# Auto-loaded by Aider on every session. Keep this accurate and concise.

## Language and runtime
[e.g. Rust 1.80+, Python 3.12+, Node 22+]

## Framework / libraries
[Main frameworks and libraries used]

## Code style
[Formatting rules, linter config references]

## Naming conventions
[Files, modules, functions, variables — what patterns we follow]

## Directory structure
[Where things live and why]

## Testing
[Test framework, where tests live, coverage requirements]

## Error handling
[Pattern used — Result<T,E>, exceptions, etc.]

## Commit conventions
[Conventional Commits — feat/fix/chore/docs etc.]

## DO NOT
- [Hard rule 1]
- [Hard rule 2]
"#;

pub const SKILL_TEMPLATE: &str = r#"# SKILL: {SKILL_NAME}
# File: .aider/skills/SKILL.{slug}.md
# Updated: {DATE}

## Overview
[Short description relevant to this skill area]

## Conventions
[Rules and patterns specific to this area]

## File locations
[Where related files live in the project]

## Design patterns
[Which patterns we use in this area and why]

## Examples
[Short, concrete code or config examples]

## Forbidden patterns
[What NOT to do in this area]

## References
[Links to docs, ADRs, or related files]
"#;

pub const AIDER_CONF: &str = r#"# .aider.conf.yml — Aider configuration for this project
# Auto-loaded by Aider when run from project root.
model: ollama/qwen3-coder:30b
map-tokens: 4096
no-auto-commits: true
read:
  - .aider/CONVENTIONS.md
  - .aider/context/CODEBASE.md
  - .aider/context/PATTERNS.md
"#;

pub const IMPL_SUMMARY_TEMPLATE: &str = r#"# Implementation summary: {TASK_TITLE}

**Plan:** {PLAN_FILE}
**Date:** {DATE}
**Branch:** {BRANCH}

## Changes made

### Modified files
[List of modified files with a brief description of changes]

### New files
[List of new files with a brief description]

## Deviations from plan

[If there were any deviations from the plan — described here with justification]

## Open issues

[If the implementation encountered problems requiring a decision]
"#;
