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
