# GitHub Action Skill Integration Design

**Date:** 2025-11-22
**Status:** Approved
**Author:** Francois Saint-Jacques

## Overview

This design document describes how to integrate plugin-based skills (like rust-toolkit) into GitHub PR reviews using the native `anthropics/claude-code-action`. The system uses a meta-dispatcher agent that automatically discovers applicable skills, spawns subagents to execute them, and posts inline code review comments.

## Goals

1. Enable automated, skill-based PR reviews in GitHub Actions
2. Use LLM evaluation to determine which skills apply to each PR
3. Leverage existing skills without requiring routing manifests
4. Post actionable inline comments at specific code locations
5. Support multiple plugins with minimal configuration

## Architecture

### High-Level Flow

```
GitHub PR Event
    ↓
claude-code-action invoked
    ↓
skill-dispatcher agent (agents/skill-dispatcher.md)
    ↓
    ├─ 1. Get PR context (gh pr view, gh pr diff)
    ├─ 2. Discover skills (glob .claude/plugins/*/skills/*/SKILL.md)
    ├─ 3. LLM evaluates applicability (read descriptions, match to PR)
    ├─ 4. Spawn subagents (Task tool, one per skill)
    │     ├─ rust-async-design subagent
    │     │   └─ Posts inline comments via mcp__github_inline_comment__create_inline_comment
    │     ├─ rust-error-handling subagent
    │     │   └─ Posts inline comments via mcp__github_inline_comment__create_inline_comment
    │     └─ ...
    ├─ 5. Collect results from all subagents
    └─ 6. Post summary comment (gh pr comment)
```

### Component Architecture

**1. Meta-Dispatcher Agent** (`agents/skill-dispatcher.md`)

The dispatcher agent orchestrates the entire review process:

- **PR Context Gathering**: Uses `gh` CLI to fetch PR metadata, changed files, and diffs
- **Skill Discovery**: Scans `.claude/plugins/*/skills/*/SKILL.md` for all available skills
- **Applicability Evaluation**: Uses LLM reasoning to match PR context against skill descriptions
- **Subagent Orchestration**: Spawns fresh subagents for each applicable skill using the Task tool
- **Result Aggregation**: Collects findings from all subagents and posts summary

**2. Plugin Skills** (existing, no changes needed)

Skills like `rust-async-design`, `rust-error-handling`, etc. are already self-describing:

```yaml
---
name: rust-async-design
description: Use when reviewing async Rust code for deadlocks, race conditions...
---
```

The dispatcher reads these descriptions to determine applicability.

**3. GitHub Action Workflow** (user-provided)

Target repositories configure a workflow that invokes the dispatcher:

```yaml
name: Claude Skill-Based PR Review

on:
  pull_request:
    types: [opened, synchronize, ready_for_review, reopened]

jobs:
  review:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
      id-token: write

    steps:
      - uses: actions/checkout@v4

      - uses: anthropics/claude-code-action@v1
        with:
          anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}
          track_progress: true
          prompt: |
            Use the skill-dispatcher agent to review this PR.

            PR Number: ${{ github.event.pull_request.number }}
            Repository: ${{ github.repository }}

          claude_args: |
            --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(gh pr *),Read,Glob,Grep,Task,Skill"
```

## Key Design Decisions

### 1. Agent vs Skill for Dispatcher

**Decision**: Use an agent (`agents/skill-dispatcher.md`)
**Rationale**: Agents are designed for autonomous task execution, which matches the dispatcher's responsibilities:
- Independent operation
- Decision-making (which skills apply)
- Orchestrating subagents
- Aggregating results

Skills are for guided workflows followed by the main session. The dispatcher needs autonomy.

**Precedent**: `anthropics/claude-code/plugins/pr-review-toolkit` uses agents for PR reviews.

### 2. No Routing Manifests

**Decision**: Skills are self-describing via frontmatter and documentation
**Rationale**:
- Skills already have `description` fields and "when to use" sections
- LLM evaluation is more flexible than regex patterns
- No duplication or extra configuration files
- Adding new skills requires zero registration

**Alternative Rejected**: Plugin-specific routing manifests (`.claude-plugin/github-routing.json`)
- Would duplicate information already in SKILL.md files
- Harder to maintain (two sources of truth)
- Less flexible than LLM reasoning

### 3. LLM-Based Skill Evaluation

**Decision**: Use LLM to evaluate skill applicability based on PR context + skill descriptions
**Rationale**:
- Skills describe themselves in natural language
- LLM can handle edge cases and nuance better than pattern matching
- Flexible as skills evolve
- No brittle file extension mappings

**Process**:
1. Dispatcher reads all SKILL.md files
2. Extracts frontmatter (name, description) and overview sections
3. LLM analyzes: "Given this PR context (files changed, diff), does skill X apply?"
4. Returns list of applicable skills with confidence/priority

### 4. Subagent-Driven Execution

**Decision**: Follow the subagent-driven-development pattern
**Rationale**:
- Fresh subagent per skill = clean context, no pollution
- Parallel execution possible
- Failures isolated (one skill failing doesn't block others)
- Each subagent operates independently with access to MCP tools

**Pattern**: Dispatcher spawns subagents using Task tool, each receives:
- PR context (files, diff, metadata)
- Skill name to execute
- Access to `mcp__github_inline_comment__create_inline_comment`
- Instructions to post findings as inline comments

### 5. MCP Inline Comments for Findings

**Decision**: Use `mcp__github_inline_comment__create_inline_comment` MCP tool
**Rationale**:
- Native integration with claude-code-action
- Actionable feedback at exact code locations
- Developers can reply/resolve individual comments
- Better UX than consolidated comment blocks

**Alternative Rejected**: `gh pr comment` for all feedback
- Less actionable (no file/line context)
- Harder for developers to address specific issues

**Approach**:
- Subagents post inline comments as they find issues
- Dispatcher posts summary comment listing which skills ran

## Data Flow

### Phase 1: Initialization

1. GitHub triggers workflow on PR event
2. claude-code-action starts with prompt to use skill-dispatcher agent
3. Dispatcher agent activates with access to tools: `Bash`, `Read`, `Glob`, `Task`, `Skill`, MCP tools

### Phase 2: Discovery

1. **Get PR Context**:
   ```bash
   gh pr view <PR_NUMBER> --json files,additions,deletions,title,body,labels
   gh pr diff <PR_NUMBER>
   ```

2. **Discover Skills**:
   ```bash
   glob .claude/plugins/*/skills/*/SKILL.md
   ```

3. **Read Skill Metadata**:
   - For each SKILL.md file:
     - Parse frontmatter (name, description)
     - Extract "Overview" or "When to use" sections

### Phase 3: Evaluation

LLM analyzes:
```
PR Context:
- Files changed: src/handlers.rs, src/api.rs
- Languages: Rust
- Diff shows: async functions, Result types, .await calls

Available Skills:
- rust-async-design: "Use when reviewing async Rust code..."
- rust-error-handling: "Use when reviewing error handling..."
- python-linter: "Use when reviewing Python code..."

Question: Which skills apply?
Answer: rust-async-design (high confidence), rust-error-handling (high confidence)
```

### Phase 4: Execution

For each applicable skill, dispatcher spawns subagent:

```
Task tool invocation:
  subagent_type: general-purpose
  prompt: |
    Execute the rust-async-design skill on this PR.

    PR Number: 123
    Repository: owner/repo

    Changed files:
    - src/handlers.rs
    - src/api.rs

    Use mcp__github_inline_comment__create_inline_comment to post
    findings directly on the relevant code lines.

    [PR diff content]
```

Subagent:
1. Reads and follows the rust-async-design skill
2. Analyzes code using skill's checklist/patterns
3. Posts inline comments via MCP tool when issues found
4. Returns summary to dispatcher

### Phase 5: Aggregation

Dispatcher collects results from all subagents:

```json
{
  "skills_executed": [
    {"name": "rust-async-design", "status": "completed", "findings": 3},
    {"name": "rust-error-handling", "status": "completed", "findings": 1}
  ],
  "total_findings": 4
}
```

Posts summary comment:

```markdown
## PR Review Summary

**Skills Applied:**
- ✓ rust-async-design (3 issues found)
- ✓ rust-error-handling (1 issue found)

**Total:** 4 inline comments posted

---
_Review powered by skill-dispatcher agent_
```

## Error Handling

### 1. No Applicable Skills Found

**Scenario**: LLM evaluation determines no skills match the PR context
**Handling**:
- Post comment: "No specialized skills apply to this PR"
- Optionally suggest generic code review
- Don't fail the workflow

### 2. Skill Discovery Failures

**Scenario**: Malformed SKILL.md, invalid frontmatter, missing files
**Handling**:
- Log warning with details
- Continue with successfully loaded skills
- Include in summary: "Warning: Could not load skill X (reason)"

### 3. Subagent Execution Failures

**Scenario**: Subagent times out, errors, or crashes
**Handling**:
- Don't block other subagents (isolated execution)
- Collect partial results from successful subagents
- Report in summary: "✓ skill A completed, ✗ skill B failed: [error message]"
- Post inline comments from successful subagents

### 4. GitHub API Rate Limits

**Scenario**: `gh` commands hit rate limits on large repos/PRs
**Handling**:
- Implement exponential backoff in dispatcher
- Cache PR context to avoid repeated fetches
- Gracefully degrade: skip skill if context unavailable

### 5. Large PRs (100+ files)

**Scenario**: Skill evaluation expensive on massive PRs
**Handling**:
- Strategy 1: Sample representative files for matching
- Strategy 2: Quick file extension check first (`.rs` files → Rust skills)
- Strategy 3: Set timeout on evaluation phase
- Post partial results if timeout occurs

### 6. Multiple Plugins with Overlapping Skills

**Scenario**: Two plugins both provide "code-review" functionality
**Handling**:
- Option 1: Run both, merge findings in aggregation
- Option 2: Deduplicate based on skill name similarity
- Option 3: Let user configure priority in workflow (future enhancement)

## Repository Structure

```
fsj-claude-tools/
├── .claude-plugin/
│   └── plugin.json              # Marketplace metadata
├── agents/
│   └── skill-dispatcher.md      # Meta-dispatcher agent
├── plugins/
│   └── rust-toolkit/
│       ├── .claude-plugin/
│       │   └── plugin.json      # Plugin metadata
│       └── skills/
│           ├── rust-code-review-flow/
│           │   └── SKILL.md
│           ├── rust-async-design/
│           │   └── SKILL.md
│           ├── rust-error-handling/
│           │   └── SKILL.md
│           └── ... (more skills)
├── docs/
│   ├── plans/
│   │   └── 2025-11-22-github-action-skill-integration-design.md
│   └── examples/
│       └── github-workflow-example.yml
└── README.md
```

## User Installation Flow

1. **Install Plugin from Marketplace**:
   - User runs: `/plugins` in Claude Code
   - Searches for and installs `fsj-claude-tools`
   - Plugin installed to `.claude/plugins/fsj-claude-tools/`

2. **Configure GitHub Workflow**:
   - Copy example workflow to `.github/workflows/claude-skill-review.yml`
   - Customize if needed (trigger events, permissions)

3. **Add API Key Secret**:
   - GitHub repo settings → Secrets
   - Add `ANTHROPIC_API_KEY`

4. **Create PR**:
   - Developer creates PR
   - Workflow triggers → skill-dispatcher runs
   - Inline comments appear automatically

## Implementation Components

### 1. Dispatcher Agent (`agents/skill-dispatcher.md`)

**Responsibilities**:
- PR context gathering via `gh` CLI
- Skill discovery via `Glob` tool
- Skill metadata extraction via `Read` tool
- Applicability evaluation via LLM reasoning
- Subagent spawning via `Task` tool
- Result aggregation and summary posting

**Key Sections**:
- Overview and purpose
- Phase 1: PR Context Gathering
- Phase 2: Skill Discovery
- Phase 3: Applicability Evaluation
- Phase 4: Subagent Orchestration
- Phase 5: Result Aggregation
- Error handling procedures
- Examples

### 2. Plugin Metadata (`.claude-plugin/plugin.json`)

```json
{
  "name": "fsj-claude-tools",
  "description": "GitHub Action skill dispatcher and Rust code review toolkit",
  "version": "1.0.0",
  "author": {
    "name": "Francois Saint-Jacques",
    "email": "fsaintjacques@gmail.com"
  }
}
```

### 3. Documentation

**README.md updates**:
- Installation instructions
- GitHub Action setup guide
- Workflow configuration examples
- Troubleshooting section

**Example Workflow** (`docs/examples/github-workflow-example.yml`):
- Complete, copy-pastable workflow file
- Comments explaining each section
- Customization notes

## Testing Strategy

**Manual Testing**:
1. Create test repository with sample Rust code containing known issues
2. Install fsj-claude-tools plugin
3. Configure workflow
4. Create test PRs with various scenarios:
   - Single language (Rust only)
   - Multiple languages
   - Large PRs (many files)
   - PRs with no issues
5. Verify inline comments appear at correct locations
6. Verify summary comment is accurate
7. Test error cases (malformed skills, missing permissions)

## Future Enhancements

1. **Performance Optimization**:
   - Parallel subagent execution (already supported by Task tool)
   - Skill evaluation caching for similar PRs
   - Incremental reviews (only changed lines)

2. **Configuration Options**:
   - Skill priority/ordering in workflow
   - Severity filtering (only post "high" severity comments)
   - Custom skill discovery paths

3. **Additional Plugins**:
   - Python toolkit
   - JavaScript/TypeScript toolkit
   - Security-focused plugin
   - Performance analysis plugin

4. **Integration with Code Owners**:
   - Route specific skills based on CODEOWNERS file
   - Notify skill-relevant reviewers

5. **Metrics and Reporting**:
   - Track skill usage across PRs
   - Measure review quality (issues caught, false positives)
   - Performance metrics (review time, token usage)

## References

- [claude-code-action repository](https://github.com/anthropics/claude-code-action)
- [claude-code-action examples](https://raw.githubusercontent.com/anthropics/claude-code-action/refs/heads/main/examples/pr-review-comprehensive.yml)
- [claude-code-action solutions](https://raw.githubusercontent.com/anthropics/claude-code-action/refs/heads/main/docs/solutions.md)
- [pr-review-toolkit plugin](https://github.com/anthropics/claude-code/tree/main/plugins/pr-review-toolkit)
- [subagent-driven-development pattern](https://raw.githubusercontent.com/obra/superpowers/refs/heads/main/skills/subagent-driven-development/SKILL.md)
