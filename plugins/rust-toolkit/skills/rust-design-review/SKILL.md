---
name: rust-design-review
description: Use when reviewing Rust design documents before implementation - identifies architectural assumptions, missing error strategies, unclear boundaries, and unvalidated performance claims through structured questioning
---

# Rust Design Review

## Overview

Review Rust designs *before* implementation to catch architectural issues early, identify missing specifications, and surface unvalidated assumptions.

**Core principle:** A design review catches problems at the lowest cost - when they're still ideas, not code.

**Use when:** You have a design document or architecture proposal before implementation begins. Implementation hasn't started.

**Do NOT use this skill for:**
- Code review (use domain-specific skills like `rust-async-design`, `rust-type-system`)
- Post-implementation architectural analysis
- Refactoring existing codebases

## The Design Review Process

### Phase 1: Understand the Design Intent

Read the design document and identify:

**Core questions to answer:**
1. What problem does this solve?
2. Who are the users (internal code, external API)?
3. What are the success criteria?
4. What are the constraints (performance, maintainability, team capacity)?

**Red flag:** Document lacks clear problem statement or success criteria.

### Phase 2: Check for Architectural Clarity

Does the design clearly specify these aspects?

**Component Identification**
```
Questions to ask:
- Can you draw the component boxes?
- What are the clear boundaries?
- Which components are tightly coupled?
- Are any components over-sized (doing too much)?
```

**Ownership and Dependencies**
```
Example dependency issues:
❌ "Services communicate" (too vague)
✅ "ServiceA calls ServiceB's API, ServiceB never calls ServiceA"

❌ Circular dependencies implied
✅ Clear acyclic dependency graph
```

**Data Flow**
```
Questions to ask:
- Can you trace a request from entry to exit?
- Where does data transform?
- Where can data be lost or duplicated?
- What's the latency path?
```

**Creation and Initialization Order**
```
Questions to ask:
- What's initialized first?
- Which components depend on others existing?
- Is the init order documented?
- Are there optional components?
```

### Phase 3: Examine Error Handling Strategy

A common gap: designs specify happy path but not error paths.

**What's missing:**
```
❌ "Uses Result<T, E> for error handling" (doesn't specify what E is)
❌ "Handles database errors gracefully" (how exactly?)
❌ No mention of error propagation
❌ No recovery strategies documented
```

**What's complete:**
```
✅ Error types clearly defined (parse errors, network errors, validation errors)
✅ Which errors are recoverable vs fatal
✅ Where errors are caught vs propagated
✅ Fallback behavior specified
✅ Logging/observability for each error type
```

**Error design questions:**
1. What are the possible failure modes?
2. For each failure, who should handle it?
3. Can the caller recover from this error?
4. Does the error need context (what operation failed, with what input)?
5. Should some errors be logged, retried, or reported?

**Red flag patterns:**
- Error handling not mentioned at all
- "Errors are handled" without specifics
- Same error type for unrelated failures
- No distinction between retryable/non-retryable errors

### Phase 4: Validate Concurrency Assumptions

If design involves async, threading, or shared state:

**Thread Safety Questions**
```
❌ "Uses Arc<Mutex<T>> for shared state" (is this needed?)
✅ "Data is immutable after creation, shared via Arc<T>"
✅ "Mutable state protected by Mutex, contention expected to be low"
```

**Async Model**
```
Questions to ask:
- Which operations are async?
- Which must be async (I/O, waiting) vs which are async for convenience?
- Is cancellation safe?
- What happens if a task panics?
- Are there race conditions in the state transitions?
```

**Synchronization Points**
```
Questions to ask:
- Where do async operations synchronize?
- What's the latency of synchronization?
- Can deadlock occur?
- What if a synchronization point times out?
```

**Red flags:**
- `Arc<Mutex<T>>` in hot paths without justification
- Async/await for CPU-bound work
- Unspecified cancellation behavior
- No mention of timeouts

### Phase 5: Surface Unvalidated Assumptions

Every design has assumptions. Expose them:

**Performance Assumptions**
```
❌ "Sub-millisecond latency" (not measured, not validated)
✅ "Target: sub-millisecond latency. Validation: benchmark hot path
    with realistic data size. Success criteria: p99 < 1ms"
```

**Scalability Assumptions**
```
❌ "Handles millions of requests" (how do you know?)
✅ "Target: 1M req/sec. Validation: load test with expected distribution.
    Bottleneck analysis identifies lock contention as limiting factor."
```

**Availability Assumptions**
```
❌ "99.9% available" (under what conditions?)
✅ "Target: 99.9% available. Failures: network partition, single
    server crash. Recovery: failover to standby in < 1 second."
```

**Design Complexity Assumptions**
```
❌ "Trait-based design allows flexibility" (flexibility for what?)
✅ "Trait-based handler system supports at least 3 different handler
    types without code duplication. Validated with concrete types."
```

**Questions to ask:**
1. Is this assumption stated explicitly?
2. How will you measure if it's true?
3. What breaks if this assumption is wrong?
4. When should we re-validate?

### Phase 6: Check for Unnecessary Complexity

Every design should answer: "Why is this complex?"

**Trait Usage**
```
Questions to ask:
- How many implementors of this trait will there be?
- What's the concrete list (even if planned for future)?
- Why not just use the concrete type?
- If only one implementor exists, is the trait justified?

Red flag:
- Traits with "flexibility for the future"
- Generic interfaces designed for extensibility not currently needed
```

**Generic Type Parameters**
```
Questions to ask:
- How many type parameters does this have?
- Are all of them used in the public API?
- Could some be config instead of type parameters?
- Would this be simpler with trait objects?

Red flag:
- 4+ type parameters in a struct
- Type parameters that don't appear in the API
```

**Async/Await**
```
Questions to ask:
- Does this operation need to be async?
- Or is it async for "consistency"?
- What's the latency cost?

Red flag:
- Async wrappers around sync operations
- "Everything is async" without justification
```

**Indirection**
```
Questions to ask:
- How many layers between caller and implementation?
- Does each layer add value?
- Could layers be removed?

Red flag:
- More than 3 layers without clear reason
- Layers that don't do anything
```

## The Review Checklist

Before approving a design, verify:

### Architecture & Structure
- [ ] **Clear problem statement** - What is this solving?
- [ ] **Success criteria defined** - How do we know it worked?
- [ ] **Component boxes drawable** - Can you sketch the architecture?
- [ ] **Boundaries clear** - What's in/out of scope for each component?
- [ ] **Dependencies acyclic** - Could you draw a DAG?
- [ ] **Ownership model explicit** - Who owns what?

### Data & Control Flow
- [ ] **Happy path traceable** - Can you follow a request through?
- [ ] **Data transformations clear** - Where does data change?
- [ ] **State transitions documented** - How does system evolve?
- [ ] **Latency path identified** - What's the slow path?
- [ ] **Initialization order specified** - What starts first?

### Error Handling
- [ ] **Failure modes identified** - What can go wrong?
- [ ] **Error types defined** - What are E in Result<T, E>?
- [ ] **Recovery strategy specified** - Who handles what error?
- [ ] **Context included** - Do errors carry useful information?
- [ ] **Logging strategy clear** - What gets logged where?

### Concurrency & Async
- [ ] **Async rationale given** - Why async vs sync?
- [ ] **Synchronization points identified** - Where do things coordinate?
- [ ] **Cancellation behavior specified** - What happens if task is cancelled?
- [ ] **Deadlock analysis done** - Can locks cause deadlock?
- [ ] **Thread safety justified** - Why Arc<Mutex<T>> here?

### Assumptions & Validation
- [ ] **Performance assumptions explicit** - What's the latency target?
- [ ] **Measurement plan exists** - How will you validate?
- [ ] **Failure modes documented** - What breaks assumptions?
- [ ] **Complexity justified** - Why not simpler?
- [ ] **Trade-offs named** - What did you choose NOT to do?

### Complexity Management
- [ ] **Traits justified** - How many concrete implementors?
- [ ] **Generics needed** - Why not concrete types?
- [ ] **Indirection necessary** - Does each layer add value?
- [ ] **YAGNI respected** - No features for hypothetical future?
- [ ] **Familiar patterns used** - Not reinventing?

## Red Flags That Require Discussion

**Stop and ask questions if:**

1. **"We might need to support X in the future"**
   - Concrete list of X, or remove the generality?

2. **"This trait design allows flexibility"**
   - Flexibility for what? Concrete use cases?

3. **"We'll optimize later"**
   - Premature if unproven, but validate assumptions now

4. **"Errors are handled gracefully"**
   - What does that mean? Specific strategy needed.

5. **"Service A communicates with Service B"**
   - Sync or async? Request-response or pub-sub? Retry behavior?

6. **"Uses async for scalability"**
   - Is this truly concurrent or just async for style?

7. **"Supports millions of operations"**
   - How measured? What's the bottleneck?

8. **"Simple, flexible architecture"**
   - Simple: good. Flexible: be specific.

9. **"We'll add observability later"**
   - Which metrics are critical for debugging?

10. **Missing error handling section**
    - Every system has failure modes. Where's the strategy?

## Questions to Ask as a Reviewer

Use these questions to guide the review conversation:

### High-level
- "What's the simplest design that could work?"
- "Where can this design fail?"
- "What's the most complex part?"
- "Could we remove a component?"

### Data Flow
- "Can you trace a request from start to finish?"
- "Where does data get lost?"
- "How does state evolve?"

### Errors
- "What error can occur here?"
- "Who should recover from it?"
- "What context do they need?"

### Concurrency
- "Why does this need to be async?"
- "What could deadlock?"
- "What if this lock times out?"

### Assumptions
- "How will you prove this assumption?"
- "What breaks if it's false?"
- "When should we re-validate?"

### Simplicity
- "Could we remove this trait?"
- "Do we need this generic parameter?"
- "What if we just used concrete types?"

## When NOT to Approve a Design

Send back for revision if:

1. **Failure paths not specified** - Errors not addressed at all
2. **Data flow unclear** - Can't trace a request through system
3. **Ownership ambiguous** - Unclear who owns/manages what
4. **Assumptions undocumented** - Performance claims without measurement plan
5. **Complexity unjustified** - Traits/generics without concrete use cases
6. **Initialization order undefined** - Could lead to deadlock or use-before-init
7. **Thread safety not addressed** - Shared state with no synchronization strategy
8. **Success criteria missing** - Can't tell if implementation is correct

## The Approval Bar

A design is ready to implement when:

✅ Architecture can be sketched and explained to someone new
✅ All major components, boundaries, and dependencies are clear
✅ Data flow can be traced for normal and error cases
✅ Error handling strategy specified (not perfect, but clear)
✅ Concurrency model (if any) is explicit
✅ Performance/scalability assumptions are documented with validation plan
✅ Unnecessary complexity has been removed
✅ Reviewers understand what's being built and why

## Discussion Format

When issues arise, use this format:

**Pattern identified:** "This design uses a trait with only one implementor"

**Question:** "Would a concrete type be simpler here?"

**Trade-off explanation:** "You'd gain simplicity but lose future flexibility"

**Suggestion:** "Implement as concrete type now, extract trait only when a second implementor is needed"

## Example: Good Design Review Output

```
✅ Architecture clear - can sketch 4 components and their interactions
✅ Data flow traceable - user request → handler → service → database ✓
✅ Error handling specified - validation errors return 400, DB errors return 500
✅ Async justified - I/O operations are network-bound, not CPU-bound
⚠️ Question: Arc<Mutex<Hashmap>> for request cache - is lock contention acceptable?
   → Proposal: Benchmark expected contention, use sharded locks if needed
⚠️ Question: Handler trait with one implementor - remove trait, use struct directly?
   → Proposal: Keep trait only if extensibility is concrete requirement
✅ Ready for implementation after addressing above questions
```
