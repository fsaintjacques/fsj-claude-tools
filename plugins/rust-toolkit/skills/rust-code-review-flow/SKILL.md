---
name: rust-code-review-flow
description: Use when starting a Rust code review to determine which review skills to apply - routes code to appropriate specialized skills based on patterns detected in the implementation
---

# Rust Code Review Flow

## Overview

When you encounter Rust code to review, you need to determine: which skills apply? What are the core issues? What review pattern matches this code?

This skill is a **router** - it helps you identify which specialized review skills to use based on what you see in the code.

**Use when:** You're starting a code review and need to decide which skills to apply.

**Do NOT use this skill for:** Conducting the actual review (use the specific skills identified here).

## The Review Detection Process

### Phase 1: Scan for Issue Categories

First pass: What type of code are you looking at?

**Questions to ask yourself:**
1. Is this async code? (Look for `async`, `await`, `tokio::`, locks)
2. Is this a design proposal? (Is it a design document, not code?)
3. Is this unsafe code? (Look for `unsafe`, raw pointers, FFI)
4. Is this error handling? (Result types, custom error types)
5. Is this struct/trait design? (Struct definitions, trait implementations)

**Quick scan checklist:**
- [ ] Async operations present?
- [ ] Design document or implementation code?
- [ ] Unsafe blocks?
- [ ] Error types or handling patterns?
- [ ] Struct composition or architecture?
- [ ] Generic types or trait bounds?
- [ ] Borrowing/lifetime parameters?

### Phase 2: Identify Specific Patterns

Look deeper at what specific patterns you see:

**Async Patterns:**
- `Mutex` (sync lock) with `.await` nearby → **rust-async-design**
- `tokio::spawn` without observing result → **rust-async-design**
- No timeout on external I/O → **rust-async-design**
- `select!` with error handling issues → **rust-async-design**

**Error Handling Patterns:**
- `Result<T, String>` → **rust-error-handling**
- `.map_err(|_| ...)` (source discarded) → **rust-error-handling**
- Error type with no documentation → **rust-error-handling**
- No distinction between recoverable/fatal → **rust-error-handling**

**Memory Safety Patterns:**
- `unsafe` blocks → **rust-systems-review**
- Raw pointers (`*const`, `*mut`) → **rust-systems-review**
- FFI (extern "C") → **rust-systems-review**
- No SAFETY comment on unsafe → **rust-systems-review**

**Type System Patterns:**
- 4+ type parameters → **rust-type-system**
- Complex trait bounds → **rust-type-system**
- Generic code hard to call → **rust-type-system**
- Trait objects mixed with statics → **rust-type-system**

**Borrowing/Lifetime Patterns:**
- 3+ lifetime parameters → **rust-borrowing-complexity**
- Lifetime appears once → **rust-borrowing-complexity**
- Struct all borrowed fields → **rust-borrowing-complexity**
- Self-referential attempt → **rust-borrowing-complexity**

**Architectural Patterns:**
- Struct with 15+ fields → **rust-architectural-composition-critique**
- 30+ methods on one struct → **rust-architectural-composition-critique**
- 5+ layer processing pipeline → **rust-architectural-composition-critique**
- Single-method traits × 5 → **rust-architectural-composition-critique**
- Concrete type dependencies → **rust-architectural-composition-critique**

**Trait Detection:**
- Method like `as_bytes()` not implementing `AsRef` → **rust-trait-detection**
- `into_iter()` method but no `IntoIterator` impl → **rust-advanced-trait-detection**
- Missing `Clone`, `Copy`, `Default`, `Debug` → **rust-trait-detection**
- Missing `Deref`, `Drop`, operator traits → **rust-advanced-trait-detection**

**Design Document:**
- Pre-implementation architecture → **rust-design-review**
- Unvalidated assumptions → **rust-design-review**
- No error strategy documented → **rust-design-review**

### Phase 3: Route to Appropriate Skills

Use this decision tree to route to the right skill(s):

```
What are you reviewing?

┌─ Design Document (not code)?
│  └─ YES → rust-design-review
│
├─ Async Code?
│  ├─ Sync locks with await?
│  │  └─ YES → rust-async-design
│  ├─ Unbounded spawning?
│  │  └─ YES → rust-async-design
│  ├─ No timeout on I/O?
│  │  └─ YES → rust-async-design
│  └─ Race conditions?
│     └─ YES → rust-async-design
│
├─ Unsafe Code?
│  ├─ Unsafe blocks present?
│  │  └─ YES → rust-systems-review
│  ├─ Raw pointers?
│  │  └─ YES → rust-systems-review
│  └─ FFI code?
│     └─ YES → rust-systems-review
│
├─ Error Handling?
│  ├─ Result<T, String> or Box<dyn Error>?
│  │  └─ YES → rust-error-handling
│  ├─ Context loss in errors?
│  │  └─ YES → rust-error-handling
│  ├─ No recovery strategy?
│  │  └─ YES → rust-error-handling
│  └─ Undocumented error types?
│     └─ YES → rust-error-handling
│
├─ Type System / Generics?
│  ├─ 4+ type parameters?
│  │  └─ YES → rust-type-system
│  ├─ Complex trait bounds?
│  │  └─ YES → rust-type-system
│  ├─ Over/under-constrained generics?
│  │  └─ YES → rust-type-system
│  └─ Trait objects mixed with static dispatch?
│     └─ YES → rust-type-system
│
├─ Borrowing / Lifetimes?
│  ├─ 3+ lifetime parameters?
│  │  └─ YES → rust-borrowing-complexity
│  ├─ Lifetime appears once?
│  │  └─ YES → rust-borrowing-complexity
│  ├─ All fields borrowed?
│  │  └─ YES → rust-borrowing-complexity
│  └─ Self-referential pattern?
│     └─ YES → rust-borrowing-complexity
│
├─ Architecture / Composition?
│  ├─ Struct: 15+ fields?
│  │  └─ YES → rust-architectural-composition-critique
│  ├─ Struct: 30+ methods?
│  │  └─ YES → rust-architectural-composition-critique
│  ├─ 5+ layer pipeline?
│  │  └─ YES → rust-architectural-composition-critique
│  ├─ Multiple single-method traits?
│  │  └─ YES → rust-architectural-composition-critique
│  ├─ Concrete type dependencies?
│  │  └─ YES → rust-architectural-composition-critique
│  └─ Tight coupling?
│     └─ YES → rust-architectural-composition-critique
│
└─ Missing Trait Implementations?
   ├─ `as_*()` method without AsRef/AsMut?
   │  └─ YES → rust-trait-detection
   ├─ `into_iter()` without IntoIterator?
   │  └─ YES → rust-advanced-trait-detection
   ├─ Missing Clone, Copy, Default, Debug?
   │  └─ YES → rust-trait-detection
   └─ Missing Deref, Drop, operators?
      └─ YES → rust-advanced-trait-detection
```

## Real-World Examples

### Example 1: Async Code with Error Handling Issues

```rust
async fn fetch_data(url: &str) -> Result<Vec<u8>, String> {
    let mut cache = CACHE.lock().unwrap();  // Sync lock

    if let Some(data) = cache.get(url) {
        return Ok(data.clone());
    }

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Error: {}", e))?;  // Context loss

    let data = response.bytes().await?.to_vec();
    Ok(data)
}
```

**Detection Process:**
1. Scan: Is this async? YES (has `async`, `.await`)
2. Pattern match: Sync lock with await? YES
3. Pattern match: Error handling issue? YES (String, context loss)
4. Route:
   - **rust-async-design** (primary - sync lock in async)
   - **rust-error-handling** (secondary - error type and context)

### Example 2: Struct with Multiple Issues

```rust
struct Service<T, U, V, E>
where
    T: Trait1,
    U: Trait2,
    V: Trait3,
    E: std::error::Error,
{
    db: Box<PostgresDb>,
    cache: Box<RedisCache>,
    logger: ConsoleLogger,
    handler: Arc<T>,
    repo: U,
    processor: V,
    error: std::marker::PhantomData<E>,
}

impl<T, U, V, E> Service<T, U, V, E> { /* 40+ methods */ }
```

**Detection Process:**
1. Scan: Type system issue? YES (4 type params, complex)
2. Pattern match: Over-genericized? YES
3. Pattern match: Tight coupling? YES (concrete types like PostgresDb, ConsoleLogger)
4. Pattern match: God object? YES (40+ methods)
5. Route:
   - **rust-architectural-composition-critique** (primary - composition, tight coupling, god object)
   - **rust-type-system** (secondary - over-generics)
   - **rust-design-review** (tertiary - if pre-implementation, design is wrong)

### Example 3: Error Handling Only

```rust
fn load_config(path: &str) -> Result<Config, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Error: {}", e))?;

    serde_json::from_str(&text)
        .map_err(|e| format!("Parse failed: {}", e))
}
```

**Detection Process:**
1. Scan: Error handling? YES (Result, String)
2. Pattern match: Generic error type? YES (String)
3. Pattern match: Context loss? YES (.map_err with format)
4. Route:
   - **rust-error-handling** (primary and only)

### Example 4: Trait Implementation Missing

```rust
struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

// Should implement AsRef<[u8]> but doesn't
```

**Detection Process:**
1. Scan: Trait implementation issue? YES (see `as_*()` method)
2. Pattern match: Missing AsRef/AsMut? YES
3. Route:
   - **rust-trait-detection** (primary and only)

### Example 5: Borrowing Complexity

```rust
struct Document<'a, 'b, 'c> {
    title: &'a str,
    content: &'b str,
    metadata: &'c str,
}

impl<'a, 'b, 'c> Document<'a, 'b, 'c> {
    fn process(&self) -> String {
        format!("{}: {}", self.title, self.content)
    }
}
```

**Detection Process:**
1. Scan: Lifetimes present? YES (three)
2. Pattern match: 3+ lifetimes? YES
3. Pattern match: All used? Questionable (metadata unused)
4. Route:
   - **rust-borrowing-complexity** (primary and only)

## Routing Strategy When Multiple Issues Detected

**Key principle:** Route to skills in order of severity/priority.

### Priority Order

1. **Async + Sync Lock** (deadlock/correctness risk) → **rust-async-design** first
2. **Unsafe without docs** (memory safety) → **rust-systems-review** first
3. **God object** (architectural) → **rust-architectural-composition-critique** first
4. **Error context loss** (debuggability) → **rust-error-handling** first
5. **Type system issues** → **rust-type-system**
6. **Borrowing complexity** → **rust-borrowing-complexity**
7. **Missing traits** → **rust-trait-detection** or **rust-advanced-trait-detection**

### Multi-Skill Reviews

When multiple issues present, return them in order:

**Example: Code with 3 issues**
```
Primary issue: Async code with sync lock across await
  → Use rust-async-design

Secondary issue: Error handling with context loss
  → Use rust-error-handling

Tertiary issue: Overcomplicated generic signatures
  → Use rust-type-system

Review order: 1) async-design, 2) error-handling, 3) type-system
```

## When to Use rust-design-review

Use **rust-design-review** (not implementation skills) when:
- Code is a **design document**, not implementation
- You're reviewing **before implementation** starts
- Code shows **unvalidated assumptions** (performance, availability)
- **Error strategy not documented**
- **Component boundaries unclear**
- **No measurement plan** for metrics/throughput

If implementation already exists, use domain-specific skills instead.

## Quick Reference: Pattern → Skill

| Pattern | Skill |
|---------|-------|
| Sync lock with await | rust-async-design |
| No timeout on I/O | rust-async-design |
| Unbounded spawning | rust-async-design |
| Race condition | rust-async-design |
| Result<T, String> | rust-error-handling |
| .map_err(\|_\| ...) | rust-error-handling |
| Undocumented errors | rust-error-handling |
| unsafe block | rust-systems-review |
| Raw pointer deref | rust-systems-review |
| FFI code | rust-systems-review |
| 4+ type parameters | rust-type-system |
| Over-constrained generics | rust-type-system |
| Trait object confusion | rust-type-system |
| 3+ lifetimes | rust-borrowing-complexity |
| Self-referential | rust-borrowing-complexity |
| All fields borrowed | rust-borrowing-complexity |
| 15+ fields | rust-architectural-composition-critique |
| 30+ methods | rust-architectural-composition-critique |
| 5+ layer pipeline | rust-architectural-composition-critique |
| Trait explosion | rust-architectural-composition-critique |
| Concrete dependencies | rust-architectural-composition-critique |
| as_*() without AsRef | rust-trait-detection |
| into_iter() without IntoIterator | rust-advanced-trait-detection |

## Decision Checklist

Before starting a code review:

```
☐ Is this code or a design document?
  ☐ If design: Use rust-design-review

☐ What patterns do I see?
  ☐ Async operations?
  ☐ Error handling?
  ☐ Unsafe code?
  ☐ Generics/type system?
  ☐ Borrows/lifetimes?
  ☐ Struct composition?
  ☐ Missing traits?

☐ Which skills apply?
  ☐ List all applicable skills

☐ What's the priority?
  ☐ Order skills by severity (correctness > design > style)

☐ Start review with highest priority skill
  ☐ Complete that skill
  ☐ Move to next skill
  ☐ Integrate findings

☐ Are findings overlapping?
  ☐ Check if one skill's findings affect another
  ☐ Coordinate feedback
```

## Common Combinations

**Async + Errors:**
1. rust-async-design (correctness)
2. rust-error-handling (propagation)

**Architecture + Traits:**
1. rust-architectural-composition-critique (structure)
2. rust-type-system (design)
3. rust-trait-detection (missing impls)

**Unsafe + Design:**
1. rust-systems-review (safety)
2. rust-design-review (if pre-impl)

**Generics + Borrowing:**
1. rust-type-system (type params)
2. rust-borrowing-complexity (lifetimes)

**Composition + Dependencies:**
1. rust-architectural-composition-critique (structure)
2. rust-design-review (if issues are pre-implementation)

## When Skills Overlap

Sometimes multiple skills apply to same code. Order by impact:

**Correctness issues** (affects functionality):
- rust-async-design
- rust-systems-review
- rust-error-handling

**Design issues** (affects maintainability):
- rust-architectural-composition-critique
- rust-design-review
- rust-borrowing-complexity

**Style/completeness** (affects polish):
- rust-trait-detection
- rust-type-system

If correctness + design issues both present: address correctness first, then design.

## Example: Multi-Skill Review

**Code Submitted:**
```rust
async fn api_handler() -> Result<Response, String> {
    let db = DB.lock().unwrap();
    let data = db.query().await?;
    Ok(Response(data))
}

struct Handler {
    db: Box<PostgresDb>,
    cache: Box<RedisCache>,
    logger: ConsoleLogger,
    // + 20 more fields
}
```

**Review Flow:**
1. **Detection:** Async + errors + god object + tight coupling
2. **Route Priority:**
   - Primary: **rust-async-design** (sync lock in async = correctness bug)
   - Secondary: **rust-error-handling** (String error type = debuggability)
   - Tertiary: **rust-architectural-composition-critique** (god object + tight coupling = design)
3. **Review Order:**
   - First: Use rust-async-design to identify sync lock issue
   - Second: Use rust-error-handling for proper error strategy
   - Third: Use rust-architectural-composition-critique for Handler redesign
4. **Feedback Integration:**
   - Fix sync lock + error handling
   - Redesign Handler with dependency injection
   - Final code passes all three skill reviews

## When in Doubt

If unsure which skill applies:

1. **Ask:** What would break this code in production?
   - Async/concurrency issue? → **rust-async-design**
   - Memory safety issue? → **rust-systems-review**
   - Impossible to debug? → **rust-error-handling**
   - Can't test/reuse? → **rust-architectural-composition-critique**

2. **Ask:** What would confuse a new developer?
   - Complex generics? → **rust-type-system**
   - Confusing lifetimes? → **rust-borrowing-complexity**
   - Too many traits/methods? → **rust-architectural-composition-critique**

3. **Ask:** What's incomplete?
   - Missing trait implementation? → **rust-trait-detection**
   - No error documentation? → **rust-error-handling**

4. **Ask:** Is this pre-implementation?
   - Design document? → **rust-design-review**
