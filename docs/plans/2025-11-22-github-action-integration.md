# GitHub Action Skill Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a meta-dispatcher agent that automatically discovers and routes PR reviews to applicable plugin skills, posting inline comments via claude-code-action.

**Architecture:** The skill-dispatcher agent analyzes PR context, discovers all available skills by scanning `.claude/plugins/*/skills/*/SKILL.md` files, uses LLM reasoning to determine applicability, spawns subagents for each applicable skill, and aggregates results. Subagents post inline comments directly using the MCP `mcp__github_inline_comment__create_inline_comment` tool.

**Tech Stack:**
- claude-code-action (GitHub Action wrapper)
- Task tool (subagent spawning)
- MCP GitHub inline comment tool
- gh CLI (PR context gathering)
- Markdown (agent/skill documentation)

---

## Task 1: Create Dispatcher Agent Skeleton

**Files:**
- Create: `agents/skill-dispatcher.md`

**Step 1: Write the agent header and overview**

Create the agent file with frontmatter and overview:

```markdown
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
```

**Step 2: Commit the skeleton**

```bash
cd /Users/fsaintjacques/src/fsj-claude-tools/.worktrees/github-action-integration
git add agents/skill-dispatcher.md
git commit -m "feat: add skill-dispatcher agent skeleton

Initial structure with overview and prerequisites.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds, agent skeleton created

---

## Task 2: Implement Phase 1 - PR Context Gathering

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add PR context gathering section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Phase 1: PR Context Gathering

**Objective:** Collect comprehensive information about the PR to inform skill matching.

### Step 1.1: Gather PR Metadata

Use `gh` CLI to fetch PR information:

\`\`\`bash
gh pr view <PR_NUMBER> --json files,additions,deletions,title,body,labels --jq '.'
\`\`\`

**Capture:**
- `files`: Array of changed file paths
- `additions`: Number of lines added
- `deletions`: Number of lines removed
- `title`: PR title
- `body`: PR description
- `labels`: PR labels

### Step 1.2: Get PR Diff

Fetch the actual code changes:

\`\`\`bash
gh pr diff <PR_NUMBER>
\`\`\`

**Purpose:** Provides context for pattern-based skill matching (async code, error handling, etc.)

### Step 1.3: Extract File Extensions

Analyze changed files to identify languages:

\`\`\`bash
gh pr view <PR_NUMBER> --json files --jq '.files[].path' | grep -oE '\.[^.]+$' | sort -u
\`\`\`

**Purpose:** Quick filter for language-specific skills (`.rs` → Rust skills)

### Step 1.4: Store Context

Aggregate all PR context into a structured format for skill evaluation:

\`\`\`json
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
\`\`\`

### Error Handling

**If `gh` command fails:**
- Check GitHub CLI authentication: `gh auth status`
- Check PR number validity
- Log error and exit gracefully: "Failed to gather PR context: [error]"

**If PR is too large (>1000 files):**
- Sample first 100 files for skill matching
- Log warning: "Large PR detected, sampling files for skill evaluation"
```

**Step 2: Commit Phase 1**

```bash
git add agents/skill-dispatcher.md
git commit -m "feat: add PR context gathering phase

Implements gh CLI integration for metadata, diff, and file analysis.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 3: Implement Phase 2 - Skill Discovery

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add skill discovery section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Phase 2: Skill Discovery

**Objective:** Find all available skills in installed plugins.

### Step 2.1: Discover Plugin Skills

Use `Glob` tool to find all SKILL.md files:

\`\`\`bash
# Pattern to find all plugin skills
.claude/plugins/*/skills/*/SKILL.md
\`\`\`

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

\`\`\`yaml
---
name: rust-async-design
description: Use when reviewing async Rust code for deadlocks, race conditions, sync locks in async contexts
---

# Rust Async Design Review

## Overview
Reviews async Rust code for common concurrency issues...
\`\`\`

**Extracted data:**
\`\`\`json
{
  "path": ".claude/plugins/rust-toolkit/skills/rust-async-design/SKILL.md",
  "name": "rust-async-design",
  "description": "Use when reviewing async Rust code for deadlocks, race conditions, sync locks in async contexts",
  "overview": "Reviews async Rust code for common concurrency issues..."
}
\`\`\`

### Step 2.3: Build Skill Registry

Aggregate all discovered skills:

\`\`\`json
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
\`\`\`

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
```

**Step 2: Commit Phase 2**

```bash
git add agents/skill-dispatcher.md
git commit -m "feat: add skill discovery phase

Implements plugin scanning and skill metadata extraction.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 4: Implement Phase 3 - Applicability Evaluation

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add applicability evaluation section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Phase 3: Applicability Evaluation

**Objective:** Use LLM reasoning to determine which skills apply to the PR.

### Step 3.1: Prepare Evaluation Prompt

For each discovered skill, construct an evaluation prompt:

\`\`\`
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
\`\`\`

### Step 3.2: Evaluate Each Skill

For each skill in the registry:
1. Run the evaluation prompt using LLM
2. Parse the response to extract decision and reasoning
3. Record the result

**Example evaluation:**

\`\`\`json
{
  "skill": "rust-async-design",
  "decision": "APPLIES",
  "confidence": "high",
  "reasoning": "PR changes .rs files with async functions and .await calls visible in diff"
}
\`\`\`

### Step 3.3: Build Applicable Skills List

Filter skills based on evaluation:

**Include if:**
- Decision is "APPLIES" (high confidence)
- Decision is "MAYBE" AND PR is small (<100 files) - better to over-review than miss issues

**Exclude if:**
- Decision is "DOES_NOT_APPLY"
- Decision is "MAYBE" AND PR is large (>100 files) - avoid unnecessary reviews on large PRs

**Result:**

\`\`\`json
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
\`\`\`

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
```

**Step 2: Commit Phase 3**

```bash
git add agents/skill-dispatcher.md
git commit -m "feat: add applicability evaluation phase

Implements LLM-based skill matching with prioritization.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 5: Implement Phase 4 - Subagent Orchestration

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add subagent orchestration section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Phase 4: Subagent Orchestration

**Objective:** Spawn fresh subagents to execute each applicable skill in isolation.

### Step 4.1: Prepare Subagent Instructions

For each applicable skill, construct subagent prompt:

\`\`\`markdown
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
\`\`\`

### Step 4.2: Spawn Subagents

For each applicable skill (in priority order):

\`\`\`python
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
\`\`\`

**Execution model:**
- **Sequential** by default (one skill at a time, easier to track progress)
- **Parallel** option for future enhancement (spawn all at once)

### Step 4.3: Monitor Subagent Execution

Track progress of each subagent:

\`\`\`json
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
\`\`\`

If `track_progress: true` is enabled in the workflow, update progress comment on PR.

### Step 4.4: Collect Results

After each subagent completes:

\`\`\`json
{
  "skill": "rust-async-design",
  "status": "completed",
  "findings_count": 3,
  "inline_comments_posted": 3,
  "duration_seconds": 45
}
\`\`\`

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
```

**Step 2: Commit Phase 4**

```bash
git add agents/skill-dispatcher.md
git commit -m "feat: add subagent orchestration phase

Implements Task tool integration for isolated skill execution.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 6: Implement Phase 5 - Result Aggregation

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add result aggregation section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Phase 5: Result Aggregation

**Objective:** Collect subagent results and post summary comment to PR.

### Step 5.1: Aggregate Subagent Results

Combine results from all subagents:

\`\`\`json
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
\`\`\`

### Step 5.2: Format Summary Comment

Construct markdown summary:

\`\`\`markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (3 issues found)
- ✓ rust-error-handling (1 issue found)

**Total:** 4 inline comments posted

**Review Details:**
This PR was automatically reviewed using specialized skills from installed plugins. Each inline comment indicates which skill identified the issue.

---
_Review powered by skill-dispatcher agent_
\`\`\`

### Step 5.3: Post Summary Comment

Use `gh` CLI to post summary:

\`\`\`bash
gh pr comment <PR_NUMBER> --body "<summary_markdown>"
\`\`\`

### Step 5.4: Handle Failures

If any skills failed, include in summary:

\`\`\`markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (3 issues found)
- ✗ rust-error-handling (failed: timeout after 5 minutes)

**Total:** 3 inline comments posted

**Warnings:**
- rust-error-handling skill timed out. Consider manual review for error handling patterns.

---
_Review powered by skill-dispatcher agent_
\`\`\`

### Step 5.5: Return Completion Status

Agent returns final status:

\`\`\`json
{
  "status": "completed",
  "skills_executed": 2,
  "skills_failed": 0,
  "total_findings": 4,
  "summary_posted": true
}
\`\`\`

### Error Handling

**If summary comment post fails:**
- Log error: "Failed to post summary comment: {error}"
- Check GitHub CLI authentication
- Retry once with exponential backoff
- If still fails, log final error but don't fail the entire review

**If no findings from any skill:**
- Post positive summary:
  \`\`\`markdown
  ## PR Review Summary

  **Skills Applied:**
  - ✓ rust-async-design (no issues found)
  - ✓ rust-error-handling (no issues found)

  **Result:** No issues detected by automated skill reviews.

  ---
  _Review powered by skill-dispatcher agent_
  \`\`\`
```

**Step 2: Commit Phase 5**

```bash
git add agents/skill-dispatcher.md
git commit -m "feat: add result aggregation phase

Implements summary comment posting and error handling.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 7: Add Examples and Usage Documentation

**Files:**
- Modify: `agents/skill-dispatcher.md`

**Step 1: Add complete examples section**

Add to `agents/skill-dispatcher.md`:

```markdown
## Complete Examples

### Example 1: Rust PR with Async Issues

**PR Context:**
- Files: `src/handlers.rs`, `src/api.rs`
- Languages: Rust (`.rs`)
- Diff shows: `async fn`, `.await`, `Mutex::new()`

**Execution Flow:**

1. **Phase 1: Context Gathering**
   \`\`\`bash
   gh pr view 123 --json files,title
   # Returns: {"files": [{"path": "src/handlers.rs"}, ...], "title": "Add async handlers"}

   gh pr diff 123
   # Returns: diff showing async code and sync mutex
   \`\`\`

2. **Phase 2: Discovery**
   \`\`\`
   Found skills:
   - rust-async-design
   - rust-error-handling
   - python-type-checker
   \`\`\`

3. **Phase 3: Evaluation**
   \`\`\`
   Applicable:
   - rust-async-design (APPLIES: async code in diff)
   - rust-error-handling (APPLIES: Result types visible)

   Not applicable:
   - python-type-checker (no .py files)
   \`\`\`

4. **Phase 4: Execution**
   \`\`\`
   Spawning subagent for rust-async-design...
   → Posted 2 inline comments (sync mutex, no timeout)

   Spawning subagent for rust-error-handling...
   → Posted 1 inline comment (String error type)
   \`\`\`

5. **Phase 5: Summary**
   \`\`\`markdown
   ## PR Review Summary

   **Skills Applied:**
   - ✓ rust-async-design (2 issues found)
   - ✓ rust-error-handling (1 issue found)

   **Total:** 3 inline comments posted
   \`\`\`

### Example 2: Documentation-Only PR

**PR Context:**
- Files: `README.md`, `docs/guide.md`
- Languages: None (markdown only)

**Execution Flow:**

1. **Phase 1-2:** Context gathered, skills discovered
2. **Phase 3: Evaluation**
   \`\`\`
   No skills apply (no code changes)
   \`\`\`
3. **Phase 5: Summary**
   \`\`\`markdown
   ## PR Review Summary

   No specialized skills apply to this PR.
   This appears to be a documentation-only change.
   \`\`\`

### Example 3: Multi-Language PR

**PR Context:**
- Files: `src/main.rs`, `scripts/deploy.py`, `tests/test_api.py`
- Languages: Rust (`.rs`), Python (`.py`)

**Execution Flow:**

1. **Phase 3: Evaluation**
   \`\`\`
   Applicable:
   - rust-async-design (Rust files)
   - rust-error-handling (Rust files)
   - python-type-checker (Python files)
   \`\`\`

2. **Phase 4: Execution**
   \`\`\`
   Executes 3 subagents in priority order
   \`\`\`

3. **Phase 5: Summary**
   \`\`\`markdown
   ## PR Review Summary

   **Skills Applied:**
   - ✓ rust-async-design (1 issue found)
   - ✓ rust-error-handling (0 issues found)
   - ✓ python-type-checker (2 issues found)

   **Total:** 3 inline comments posted
   \`\`\`
```

**Step 2: Commit examples**

```bash
git add agents/skill-dispatcher.md
git commit -m "docs: add usage examples for skill dispatcher

Shows execution flow for Rust, documentation, and multi-language PRs.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 8: Create Example Workflow Configuration

**Files:**
- Create: `docs/examples/github-workflow-example.yml`

**Step 1: Write the example workflow**

Create comprehensive workflow example:

```yaml
# Example GitHub Action workflow for skill-based PR reviews
# Copy this file to .github/workflows/claude-skill-review.yml in your repository

name: Claude Skill-Based PR Review

on:
  pull_request:
    types: [opened, synchronize, ready_for_review, reopened]

jobs:
  skill-review:
    runs-on: ubuntu-latest

    # Required permissions for PR interaction
    permissions:
      contents: read           # Read repository files
      pull-requests: write     # Post comments and reviews
      id-token: write         # OIDC authentication

    steps:
      # Checkout the PR branch
      - name: Checkout code
        uses: actions/checkout@v4

      # Run Claude skill-based review
      - name: Run skill dispatcher
        uses: anthropics/claude-code-action@v1
        with:
          # Required: Anthropic API key from repository secrets
          anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}

          # Optional: Enable progress tracking with checkboxes
          track_progress: true

          # Main prompt: Invoke skill-dispatcher agent
          prompt: |
            Use the skill-dispatcher agent to review this PR.

            PR Number: ${{ github.event.pull_request.number }}
            Repository: ${{ github.repository }}

          # Tool permissions
          claude_args: |
            --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(gh pr *),Read,Glob,Grep,Task,Skill"

# Installation Instructions:
#
# 1. Install fsj-claude-tools plugin from Claude marketplace
#    - Includes skill-dispatcher agent and rust-toolkit plugin
#
# 2. Add ANTHROPIC_API_KEY to repository secrets:
#    - Go to repository Settings → Secrets and variables → Actions
#    - Click "New repository secret"
#    - Name: ANTHROPIC_API_KEY
#    - Value: your API key from https://console.anthropic.com
#
# 3. Copy this file to .github/workflows/claude-skill-review.yml
#
# 4. Commit and push the workflow file
#
# 5. Create a PR to trigger the review
#
# Customization Options:
#
# - Trigger events: Modify the 'on:' section to change when reviews run
#   Example: Only run on PRs to main branch:
#   on:
#     pull_request:
#       types: [opened, synchronize]
#       branches: [main]
#
# - Skip draft PRs: Remove 'ready_for_review' from types, add:
#   if: github.event.pull_request.draft == false
#
# - Timeout: Add timeout-minutes to the job:
#   timeout-minutes: 10
```

**Step 2: Commit the example workflow**

```bash
git add docs/examples/github-workflow-example.yml
git commit -m "docs: add example GitHub workflow configuration

Provides copy-pastable workflow with installation instructions.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 9: Update Plugin Metadata

**Files:**
- Modify: `.claude-plugin/marketplace.json`

**Step 1: Read current metadata**

Check existing marketplace.json content.

**Step 2: Update with skill-dispatcher agent**

Update the metadata to include the new agent:

```json
{
  "name": "fsj-claude-tools",
  "displayName": "FSJ Claude Tools",
  "description": "GitHub Action skill dispatcher and Rust code review toolkit with 10+ specialized review skills",
  "version": "1.0.0",
  "author": {
    "name": "Francois Saint-Jacques",
    "email": "fsaintjacques@gmail.com"
  },
  "homepage": "https://github.com/fsaintjacques/fsj-claude-tools",
  "repository": {
    "type": "git",
    "url": "https://github.com/fsaintjacques/fsj-claude-tools.git"
  },
  "keywords": [
    "code-review",
    "rust",
    "github-actions",
    "pr-review",
    "async",
    "error-handling",
    "type-system"
  ],
  "agents": [
    {
      "name": "skill-dispatcher",
      "description": "Meta-dispatcher agent for automated PR reviews using plugin skills",
      "path": "agents/skill-dispatcher.md"
    }
  ],
  "plugins": [
    {
      "name": "rust-toolkit",
      "description": "Comprehensive Rust code review skills",
      "path": "plugins/rust-toolkit"
    }
  ]
}
```

**Step 3: Commit metadata update**

```bash
git add .claude-plugin/marketplace.json
git commit -m "feat: update plugin metadata with skill-dispatcher

Adds agent information for marketplace listing.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 10: Update README

**Files:**
- Create: `README.md` (if doesn't exist)
- Modify: `README.md` (if exists)

**Step 1: Write comprehensive README**

Create or update README.md:

```markdown
# FSJ Claude Tools

GitHub Action skill dispatcher and Rust code review toolkit for automated PR reviews.

## Features

### 🤖 Skill Dispatcher Agent

Automatically discovers and routes PR reviews to applicable plugin skills:
- Analyzes PR context (files, diff, metadata)
- Uses LLM reasoning to match skills to PR content
- Spawns isolated subagents for each skill
- Posts inline comments at specific code locations
- Aggregates results into summary comment

### 🦀 Rust Toolkit Plugin

10+ specialized Rust code review skills:
- **rust-async-design** - Async/await patterns, deadlock detection
- **rust-error-handling** - Error types, propagation, context
- **rust-type-system** - Generics, trait bounds, trait objects
- **rust-borrowing-complexity** - Lifetimes, borrowing patterns
- **rust-architectural-composition-critique** - Architecture, composition
- **rust-systems-review** - Unsafe code, FFI, memory safety
- **rust-trait-detection** - Missing standard trait implementations
- **rust-advanced-trait-detection** - Advanced traits (IntoIterator, Deref, etc.)
- **rust-design-review** - Pre-implementation design validation
- **rust-code-review-flow** - Meta-router for Rust reviews

## Installation

### 1. Install Plugin

In Claude Code:
```
/plugins
```

Search for "fsj-claude-tools" and install.

### 2. Configure GitHub Workflow

Copy the example workflow to your repository:

```bash
cp .claude/plugins/fsj-claude-tools/docs/examples/github-workflow-example.yml \
   .github/workflows/claude-skill-review.yml
```

### 3. Add API Key Secret

1. Go to repository Settings → Secrets and variables → Actions
2. Click "New repository secret"
3. Name: `ANTHROPIC_API_KEY`
4. Value: Your API key from https://console.anthropic.com

### 4. Create a PR

The skill dispatcher will automatically review PRs and post inline comments.

## Usage

### Automated PR Reviews

When a PR is opened or updated:
1. Skill dispatcher analyzes the PR
2. Identifies applicable skills (e.g., Rust skills for `.rs` files)
3. Spawns subagent for each skill
4. Posts inline comments where issues are found
5. Posts summary comment with review results

### Example Review Output

```markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (2 issues found)
- ✓ rust-error-handling (1 issue found)

**Total:** 3 inline comments posted
```

Each inline comment includes:
- Specific issue description
- Why it's a problem
- Suggested fix with code example

## Architecture

```
GitHub PR Event
    ↓
claude-code-action
    ↓
skill-dispatcher agent
    ↓
    ├─ Gather PR context (gh CLI)
    ├─ Discover skills (glob plugins)
    ├─ Evaluate applicability (LLM)
    ├─ Spawn subagents (Task tool)
    │   ├─ rust-async-design
    │   ├─ rust-error-handling
    │   └─ ...
    └─ Aggregate results
        └─ Post summary comment
```

## Configuration

### Workflow Customization

See `docs/examples/github-workflow-example.yml` for configuration options:
- Trigger events (on push, on schedule, etc.)
- Permissions
- Timeout settings
- Tool allowlist

### Skill Configuration

Skills are self-configuring - no manifest needed. To add new skills:
1. Create skill in `.claude/plugins/your-plugin/skills/skill-name/SKILL.md`
2. Add frontmatter with `name` and `description`
3. Skill dispatcher will automatically discover and use it

## Development

### Design Document

See `docs/plans/2025-11-22-github-action-skill-integration-design.md` for:
- Architecture details
- Data flow
- Error handling
- Future enhancements

### Testing

Manual testing workflow:
1. Create test repository with sample code
2. Install fsj-claude-tools plugin
3. Configure workflow
4. Create PR with known issues
5. Verify inline comments appear correctly

## Contributing

Contributions welcome! Areas for improvement:
- Additional language toolkits (Python, JavaScript, etc.)
- Performance optimizations
- Parallel subagent execution
- Custom skill priority configuration

## License

MIT

## Author

Francois Saint-Jacques (fsaintjacques@gmail.com)
```

**Step 2: Commit README**

```bash
git add README.md
git commit -m "docs: add comprehensive README

Includes installation, usage, architecture, and examples.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: Commit succeeds

---

## Task 11: Final Review and Cleanup

**Step 1: Review all files created**

Verify all components are in place:

```bash
ls -R agents/ docs/ .claude-plugin/
```

Expected files:
- `agents/skill-dispatcher.md`
- `docs/plans/2025-11-22-github-action-skill-integration-design.md`
- `docs/plans/2025-11-22-github-action-integration.md` (this plan)
- `docs/examples/github-workflow-example.yml`
- `.claude-plugin/marketplace.json`
- `README.md`

**Step 2: Check git status**

```bash
git status
```

Expected: Working directory clean, all changes committed

**Step 3: Review commit history**

```bash
git log --oneline
```

Expected: ~11 commits showing incremental progress

**Step 4: Create final summary commit if needed**

If any loose ends:

```bash
git add -A
git commit -m "chore: final cleanup and review

Ensures all components are complete and documented.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Implementation Complete

All tasks completed. The GitHub Action skill integration is ready for:
1. Manual testing with a test repository
2. Merge to main branch
3. Publication to Claude marketplace

### Next Steps

1. **Testing:**
   - Create test repository
   - Add intentional Rust code issues
   - Create PR and verify skill dispatcher works

2. **Refinement:**
   - Adjust based on test results
   - Optimize skill matching logic
   - Fine-tune subagent prompts

3. **Deployment:**
   - Merge to main
   - Tag release (v1.0.0)
   - Publish to marketplace
