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

## Phase 4: Subagent Orchestration

**Objective:** Spawn fresh subagents to execute each applicable skill in isolation.

### Step 4.1: Prepare Subagent Instructions

For each applicable skill, construct subagent prompt:

```markdown
You are executing the {skill_name} skill for a GitHub PR review.

**PR Context:**
- Repository: {repository}
- PR Number: {pr_number}
- Files changed: {files_changed}
- Languages: {languages}

**Your Task:**
1. Read and follow the {skill_name} skill at {skill_path}
2. Analyze the PR diff for issues covered by this skill
3. Use the mcp__github_inline_comment__create_inline_comment tool to post findings
4. Post inline comments at specific file:line locations where issues are found
5. Return a summary of findings to the dispatcher

**Tools Available:**
- Read (for reading skill file and PR files)
- Bash (for gh CLI commands)
- mcp__github_inline_comment__create_inline_comment (for posting comments)

**PR Diff:**
{full_diff}

**Instructions:**
Execute the skill checklist/patterns systematically.
Post inline comments for each issue found.
Provide clear, actionable feedback with suggestions.

**Return Format:**
Return a JSON summary:
\`\`\`json
{
  "skill": "{skill_name}",
  "status": "completed" | "failed",
  "findings_count": <number>,
  "inline_comments_posted": <number>,
  "error": "<error message if failed>"
}
\`\`\`
```

### Step 4.2: Spawn Subagents

For each applicable skill (in priority order):

```python
# Pseudo-code for orchestration
for skill in applicable_skills:
    # Spawn subagent using Task tool
    result = spawn_subagent(
        subagent_type="general-purpose",
        prompt=construct_subagent_prompt(skill),
        description=f"Execute {skill.name} on PR #{pr_number}"
    )

    # Collect result
    skill_results.append(result)
```

**Execution model:**
- **Sequential** by default (one skill at a time, easier to track progress)
- **Parallel** option for future enhancement (spawn all at once)

### Step 4.3: Monitor Subagent Execution

Track progress of each subagent:

```json
{
  "subagent_status": [
    {
      "skill": "rust-async-design",
      "status": "running",
      "started_at": "2025-11-22T10:30:00Z"
    },
    {
      "skill": "rust-error-handling",
      "status": "pending",
      "queued_at": "2025-11-22T10:30:05Z"
    }
  ]
}
```

If `track_progress: true` is enabled in the workflow, update progress comment on PR.

### Step 4.4: Collect Results

After each subagent completes:

```json
{
  "skill": "rust-async-design",
  "status": "completed",
  "findings_count": 3,
  "inline_comments_posted": 3,
  "duration_seconds": 45
}
```

Aggregate all results for final summary.

### Error Handling

**If subagent fails:**
- Capture error message
- Log: "Subagent for {skill} failed: {error}"
- Mark skill as failed in results
- Continue with remaining skills (isolation prevents cascade failures)

**If subagent times out (>5 minutes):**
- Log: "Subagent for {skill} timed out"
- Mark as failed with timeout error
- Continue with remaining skills

**If MCP tool unavailable:**
- Log: "MCP inline comment tool not available"
- Fallback: Post findings as regular PR comment
- Note in summary: "Posted findings as comment (inline comments unavailable)"

## Phase 5: Result Aggregation

**Objective:** Collect subagent results and post summary comment to PR.

### Step 5.1: Aggregate Subagent Results

Combine results from all subagents:

```json
{
  "total_skills_executed": 2,
  "successful_skills": 2,
  "failed_skills": 0,
  "total_findings": 4,
  "results": [
    {
      "skill": "rust-async-design",
      "status": "completed",
      "findings": 3,
      "duration": 45
    },
    {
      "skill": "rust-error-handling",
      "status": "completed",
      "findings": 1,
      "duration": 30
    }
  ]
}
```

### Step 5.2: Format Summary Comment

Construct markdown summary:

```markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (3 issues found)
- ✓ rust-error-handling (1 issue found)

**Total:** 4 inline comments posted

**Review Details:**
This PR was automatically reviewed using specialized skills from installed plugins. Each inline comment indicates which skill identified the issue.

---
_Review powered by skill-dispatcher agent_
```

### Step 5.3: Post Summary Comment

Use `gh` CLI to post summary:

```bash
gh pr comment <PR_NUMBER> --body "<summary_markdown>"
```

### Step 5.4: Handle Failures

If any skills failed, include in summary:

```markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (3 issues found)
- ✗ rust-error-handling (failed: timeout after 5 minutes)

**Total:** 3 inline comments posted

**Warnings:**
- rust-error-handling skill timed out. Consider manual review for error handling patterns.

---
_Review powered by skill-dispatcher agent_
```

### Step 5.5: Return Completion Status

Agent returns final status:

```json
{
  "status": "completed",
  "skills_executed": 2,
  "skills_failed": 0,
  "total_findings": 4,
  "summary_posted": true
}
```

### Error Handling

**If summary comment post fails:**
- Log error: "Failed to post summary comment: {error}"
- Check GitHub CLI authentication
- Retry once with exponential backoff
- If still fails, log final error but don't fail the entire review

**If no findings from any skill:**
- Post positive summary:
  ```markdown
  ## PR Review Summary

  **Skills Applied:**
  - ✓ rust-async-design (no issues found)
  - ✓ rust-error-handling (no issues found)

  **Result:** No issues detected by automated skill reviews.

  ---
  _Review powered by skill-dispatcher agent_
  ```
