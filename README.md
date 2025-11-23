# FSJ Claude Tools

GitHub Action skill dispatcher and Rust code review toolkit for automated PR reviews.

## Features

### Skill Dispatcher Agent

Automatically discovers and routes PR reviews to applicable plugin skills:
- Analyzes PR context (files, diff, metadata)
- Uses LLM reasoning to match skills to PR content
- Spawns isolated subagents for each skill
- Posts inline comments at specific code locations
- Aggregates results into summary comment

### Rust Toolkit Plugin

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
/plugin marketplace add fsaintjacques/fsj-claude-tools
/plugin install rust-tools@fsj-claude-tools
```


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

## License

MIT

## Author

Francois Saint-Jacques (fsaintjacques@gmail.com)
