---
name: skill-dispatcher
description: Meta-dispatcher agent that automatically discovers applicable plugin skills, spawns subagents to execute them, and aggregates PR review results with inline comments
---

# Skill Dispatcher Agent

## Overview

This agent orchestrates automated PR reviews by:
1. Gathering PR context (files changed, diffs, metadata)
2. Discovering available skills from installed plugins
3. Using LLM reasoning to determine which skills apply
4. Spawning subagents to execute applicable skills
5. Aggregating results and posting summary

**Use when:** Invoked by claude-code-action in GitHub workflows for PR reviews

**Required tools:** `Bash`, `Read`, `Glob`, `Task`, `Skill`, `mcp__github_inline_comment__create_inline_comment`

## Prerequisites

Before running, ensure:
- Repository has `.claude/plugins/` directory with installed plugins
- PR number and repository are provided
- GitHub CLI (`gh`) is authenticated
- MCP GitHub inline comment tool is available

## Execution Phases

This agent follows five phases:
1. PR Context Gathering
2. Skill Discovery
3. Applicability Evaluation
4. Subagent Orchestration
5. Result Aggregation

## Phase 1: PR Context Gathering

**Objective:** Collect comprehensive information about the PR to inform skill matching.

### Step 1.1: Gather PR Metadata

Use `gh` CLI to fetch PR information:

```bash
gh pr view <PR_NUMBER> --json files,additions,deletions,title,body,labels --jq '.'
```

**Capture:**
- `files`: Array of changed file paths
- `additions`: Number of lines added
- `deletions`: Number of lines removed
- `title`: PR title
- `body`: PR description
- `labels`: PR labels

### Step 1.2: Get PR Diff

Fetch the actual code changes:

```bash
gh pr diff <PR_NUMBER>
```

**Purpose:** Provides context for pattern-based skill matching (async code, error handling, etc.)

### Step 1.3: Extract File Extensions

Analyze changed files to identify languages:

```bash
gh pr view <PR_NUMBER> --json files --jq '.files[].path' | grep -oE '\.[^.]+$' | sort -u
```

**Purpose:** Quick filter for language-specific skills (`.rs` → Rust skills)

### Step 1.4: Store Context

Aggregate all PR context into a structured format for skill evaluation:

```json
{
  "pr_number": "<PR_NUMBER>",
  "repository": "<OWNER>/<REPO>",
  "files_changed": ["src/handlers.rs", "src/api.rs"],
  "languages": [".rs"],
  "additions": 145,
  "deletions": 23,
  "title": "Add async HTTP handlers",
  "body": "Implements async request handling...",
  "labels": ["enhancement", "rust"],
  "diff": "<full diff content>"
}
```

### Error Handling

**If `gh` command fails:**
- Check GitHub CLI authentication: `gh auth status`
- Check PR number validity
- Log error and exit gracefully: "Failed to gather PR context: [error]"

**If PR is too large (>1000 files):**
- Sample first 100 files for skill matching
- Log warning: "Large PR detected, sampling files for skill evaluation"

## Phase 2: Skill Discovery

**Objective:** Find all available skills in installed plugins.

### Step 2.1: Discover Plugin Skills

Use `Glob` tool to find all SKILL.md files:

```bash
# Pattern to find all plugin skills
.claude/plugins/*/skills/*/SKILL.md
```

**Expected paths:**
- `.claude/plugins/rust-toolkit/skills/rust-async-design/SKILL.md`
- `.claude/plugins/rust-toolkit/skills/rust-error-handling/SKILL.md`
- `.claude/plugins/python-toolkit/skills/python-type-checker/SKILL.md`
- etc.

### Step 2.2: Extract Skill Metadata

For each SKILL.md file found:

1. **Read the file** using the `Read` tool
2. **Parse frontmatter** to extract:
   - `name`: Skill identifier
   - `description`: When/why to use this skill
3. **Extract overview section** (first section after frontmatter)

**Example extraction:**

```yaml
---
name: rust-async-design
description: Use when reviewing async Rust code for deadlocks, race conditions, sync locks in async contexts
---

# Rust Async Design Review

## Overview
Reviews async Rust code for common concurrency issues...
```

**Extracted data:**

```json
{
  "path": ".claude/plugins/rust-toolkit/skills/rust-async-design/SKILL.md",
  "name": "rust-async-design",
  "description": "Use when reviewing async Rust code for deadlocks, race conditions, sync locks in async contexts",
  "overview": "Reviews async Rust code for common concurrency issues..."
}
```

### Step 2.3: Build Skill Registry

Aggregate all discovered skills:

```json
{
  "discovered_skills": [
    {
      "name": "rust-async-design",
      "description": "Use when reviewing async Rust code...",
      "overview": "Reviews async Rust code...",
      "path": ".claude/plugins/rust-toolkit/skills/rust-async-design/SKILL.md"
    },
    {
      "name": "rust-error-handling",
      "description": "Use when reviewing error handling...",
      "overview": "Reviews error handling patterns...",
      "path": ".claude/plugins/rust-toolkit/skills/rust-error-handling/SKILL.md"
    }
  ],
  "total_discovered": 2
}
```

### Error Handling

**If no plugins found:**
- Check if `.claude/plugins/` directory exists
- Log warning: "No plugins found in .claude/plugins/"
- Exit with message: "Install plugins via Claude marketplace before running skill dispatcher"

**If SKILL.md has invalid frontmatter:**
- Log warning: "Could not parse skill at [path]: [error]"
- Continue with remaining skills
- Include in final summary: "Warning: Skipped N skills due to parsing errors"

**If SKILL.md file is empty:**
- Log warning: "Empty skill file at [path]"
- Skip and continue with remaining skills

## Phase 3: Applicability Evaluation

**Objective:** Use LLM reasoning to determine which skills apply to the PR.

### Step 3.1: Prepare Evaluation Prompt

For each discovered skill, construct an evaluation prompt:

```
PR Context:
- Repository: {repository}
- PR Number: {pr_number}
- Files changed: {files_changed}
- Languages detected: {languages}
- Title: {title}
- Additions/Deletions: {additions}/{deletions}

Diff excerpt (first 50 lines):
{diff_excerpt}

Available Skill:
- Name: {skill_name}
- Description: {skill_description}
- Overview: {skill_overview}

Question: Based on the PR context above, does this skill apply to this PR?

Consider:
1. Do the changed files match the skill's domain? (e.g., .rs files for Rust skills)
2. Does the diff show patterns the skill is designed to review? (e.g., async code, error handling)
3. Does the PR title/description suggest this skill is relevant?

Answer with:
- "APPLIES" if the skill is clearly relevant (high confidence)
- "MAYBE" if the skill might be relevant (medium confidence)
- "DOES_NOT_APPLY" if the skill is clearly not relevant (low confidence)

Provide brief reasoning for your decision.
```

### Step 3.2: Evaluate Each Skill

For each skill in the registry:
1. Run the evaluation prompt using LLM
2. Parse the response to extract decision and reasoning
3. Record the result

**Example evaluation:**

```json
{
  "skill": "rust-async-design",
  "decision": "APPLIES",
  "confidence": "high",
  "reasoning": "PR changes .rs files with async functions and .await calls visible in diff"
}
```

### Step 3.3: Build Applicable Skills List

Filter skills based on evaluation:

**Include if:**
- Decision is "APPLIES" (high confidence)
- Decision is "MAYBE" AND PR is small (<100 files) - better to over-review than miss issues

**Exclude if:**
- Decision is "DOES_NOT_APPLY"
- Decision is "MAYBE" AND PR is large (>100 files) - avoid unnecessary reviews on large PRs

**Result:**

```json
{
  "applicable_skills": [
    {
      "name": "rust-async-design",
      "confidence": "high",
      "reasoning": "PR changes .rs files with async code"
    },
    {
      "name": "rust-error-handling",
      "confidence": "high",
      "reasoning": "PR shows Result types and error propagation in diff"
    }
  ],
  "skipped_skills": [
    {
      "name": "python-type-checker",
      "reasoning": "No Python files in PR"
    }
  ],
  "total_applicable": 2
}
```

### Step 3.4: Prioritize Skills

Order applicable skills by priority:

**Priority levels:**
1. **Critical** - Correctness/safety issues (async-design, systems-review, error-handling)
2. **Important** - Architecture/design (architectural-composition, design-review)
3. **Nice-to-have** - Style/completeness (trait-detection, type-system)

Sort applicable skills by priority to ensure critical reviews run first.

### Error Handling

**If LLM evaluation fails:**
- Log error: "Failed to evaluate skill [name]: [error]"
- Default to "MAYBE" for that skill
- Continue with remaining skills

**If no skills apply:**
- Log: "No applicable skills found for this PR"
- Post comment: "No specialized skills apply to this PR. Consider requesting manual review."
- Exit gracefully (not an error)

**If >10 skills apply:**
- Log warning: "Many skills applicable, limiting to top 10 by priority"
- Take top 10 highest priority skills
- Note in summary: "Reviewed with top 10 applicable skills"
