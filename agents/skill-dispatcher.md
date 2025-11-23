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
