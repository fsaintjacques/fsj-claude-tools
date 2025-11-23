---
name: rust-type-system
description: Review Rust code for type system, generics, traits, and composition patterns - identifies over/under-constrained generics, trait object misuse, lifetime complexity, and API ergonomics issues
---

# Rust Type System and Composition Review

## Overview

Review Rust code for issues in generics, traits, composition patterns, and lifetime complexity. Identifies designs that work but violate idiomatic Rust principles.

**Core principle:** A type system issue is idiomatic Rust missing the opportunity to express intent clearly. This skill finds those opportunities.

**Use when:** Reviewing code with complex generic types, trait bounds, lifetime annotations, or questioning if a design pattern is idiomatic.

## When to Use This Skill

**Strong signals to use this skill:**
- Code has `<T, U, V>` with complex trait bounds
- Multiple lifetime parameters (`'a`, `'b`, `'c`)
- Trait objects (`Box<dyn Trait>`) mixed with static types
- Generic functions with bounds so complex they're hard to read
- Code using composition but feels over-engineered
- API generic signatures that are hard to call

**Do NOT use this skill for:**
- Async/await patterns (use `rust-async-design`)
- Memory safety and unsafe code (use `rust-systems-review`)
- Error handling strategy (use `rust-error-handling`)

## The Review Process

### Phase 1: Identify Generic Constraints

Look for these patterns:

**Over-constrained generics** (too many bounds):
```rust
// ❌ Too many bounds
fn process<T: Serialize + Deserialize + Clone + Debug + PartialEq>(
    item: T
) -> String {
    format!("{:?}", item)
}

// ✅ Just what's needed
fn process<T: Debug>(item: T) -> String {
    format!("{:?}", item)
}
```

**Questions to ask:**
- Does every bound actually get used in the function body?
- Could this be split into two functions with fewer bounds each?
- Is the bound needed or just being "safe"?

**Under-constrained generics** (too generic, unclear intent):
```rust
// ❌ Unclear - what can T do?
fn transform<T>(input: T) -> T {
    input
}

// ✅ Clear - needs to be cloned
fn transform<T: Clone>(input: T) -> T {
    input.clone()
}
```

**Questions to ask:**
- What does the function actually need from T?
- Are bounds missing that should be stated?
- Does the signature accurately reflect what the function does?

### Phase 2: Examine Trait Usage

**Trait object misuse:**
```rust
// ❌ Trait object when static dispatch available
fn process(handler: Box<dyn Handler>) {
    handler.handle();
}

// Better with generics when possible
fn process<H: Handler>(handler: H) {
    handler.handle();
}
```

**Red flags:**
- `Box<dyn Trait>` in function arguments (usually use generics instead)
- Trait objects for one-off use cases
- Dynamic dispatch when static dispatch would work
- Trait objects without documenting why dynamic dispatch is needed

**When trait objects ARE appropriate:**
- Heterogeneous collections: `Vec<Box<dyn Handler>>`
- Returning different concrete types from a function
- Plugin systems needing runtime dispatch
- Public APIs hiding implementation

**Trait bound complexity:**
```rust
// ❌ Unreadable where clause
fn complex<T, U>(t: T, u: U)
where
    T: Into<String> + Clone + Debug + Display,
    U: AsRef<str> + PartialEq + Eq + Hash,
{
    // ...
}

// Questions to ask:
// - Are all these bounds actually used?
// - Could bounds be satisfied by fewer trait objects?
// - Is this the right abstraction level?
```

### Phase 3: Analyze Composition vs Traits

**Composition pattern issues:**

```rust
// ❌ Using trait when concrete type suffices
trait Logger {
    fn log(&self, msg: &str);
}

struct ConsoleLogger;
impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) { println!("{}", msg); }
}

// Use it
let logger: Box<dyn Logger> = Box::new(ConsoleLogger);

// ✅ If only one concrete type, use the type directly
struct ConsoleLogger;
impl ConsoleLogger {
    fn log(&self, msg: &str) { println!("{}", msg); }
}
let logger = ConsoleLogger;
```

**Design questions:**
- Are there multiple implementors of this trait, or just one?
- Is the trait interface stable or does it change frequently?
- Would dependency injection work without a trait?
- Is the abstraction justifying its complexity?

**Composition ergonomics:**
```rust
// ❌ Composition chain becomes unreadable
struct App {
    logger: Box<dyn Logger>,
    database: Box<dyn Database>,
    cache: Box<dyn Cache>,
    metrics: Box<dyn Metrics>,
}

// Often better: concrete struct with optional components
struct App {
    logger: ConsoleLogger,
    database: PostgresDatabase,
    cache: RedisCache,
    metrics: Option<PrometheusMetrics>,
}
```

### Phase 4: Lifetime Complexity

**Unnecessary lifetime parameters:**

```rust
// ❌ Lifetime not needed
fn process<'a>(input: &'a str) -> &'a str {
    input
}

// ✅ Rust can infer this
fn process(input: &str) -> &str {
    input
}

// ❌ Multiple lifetimes often signal design issues
fn complex<'a, 'b, 'c>(x: &'a str, y: &'b str) -> &'c str {
    // How does 'c relate to 'a and 'b? Unclear.
}
```

**Lifetime warning signs:**
- More than 2 lifetime parameters (design may be overcomplicated)
- Lifetimes that don't connect to the return type
- Lifetimes in struct fields that could be owned instead
- Comments explaining what lifetimes do (they shouldn't need explaining)

**Better lifetime patterns:**
```rust
// Instead of complex lifetimes, consider:
// 1. Owned data
struct Message {
    content: String,  // Owned, simple lifetime story
}

// 2. Single lifetime reference
fn process<'a>(items: &'a [Item]) -> &'a Item {
    &items[0]
}

// 3. Trait objects for complex relationships
struct Handler {
    callbacks: Vec<Box<dyn Fn(&Item)>>,
}
```

### Phase 5: API Ergonomics

**Generic APIs that are hard to use:**

```rust
// ❌ Caller must specify types and bounds
fn execute<T, E, F>(f: F) -> Result<T, E>
where
    F: Fn() -> Result<T, E>,
    T: Send + Sync + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    f()
}

// Hard to call - types unclear
let result = execute(|| Ok(42))?;

// ✅ Helper with simpler type signature
fn execute_simple<F>(f: F) -> Result<i32, Box<dyn std::error::Error>>
where
    F: Fn() -> Result<i32, Box<dyn std::error::Error>>,
{
    f()
}

// Or use turbofish with clearer intent
let result = execute::<i32, MyError, _>(|| Ok(42))?;
```

**Questions for API review:**
- Can a caller write `function_call(...)` or do they need turbofish `::<>`?
- Do bounds make sense from caller perspective?
- Could trait objects reduce type parameter explosion?
- Is there a simpler overload for common cases?

## Red Flags - Investigation Checklist

When you see these patterns, dig deeper:

- [ ] **Generic function with 4+ type parameters** → Are they all necessary? Can some be grouped into a trait?
- [ ] **3+ where clauses** → Is this abstraction worth the complexity? Could it be simpler?
- [ ] **Multiple lifetime parameters** → Do they all serve a purpose? Could owned data work instead?
- [ ] **Trait object in function argument** → Why dynamic dispatch? Would generics work?
- [ ] **Trait with single implementor** → Is a trait needed, or is the concrete type sufficient?
- [ ] **Generic function hard to call** → Does the caller need to specify types with `::<>`? Why?
- [ ] **Composition with many `Box<dyn Trait>`** → Are all traits necessary or over-engineered?
- [ ] **Lifetime in struct field** → Could the data be owned instead?

## Common Patterns and Questions

### When is a trait necessary?

| Pattern | Use Trait? | Why? |
|---------|-----------|------|
| Concrete struct with one implementation | No | Use struct directly |
| Multiple implementations needed | Yes | Trait provides abstraction |
| Plugin system / runtime dispatch | Yes | Trait objects for heterogeneity |
| Dependency injection | Maybe | Concrete type often simpler |
| Behavior variation | Yes | Trait encapsulates variation |

### Generic vs Trait Object Decision

**Use generics when:**
- Concrete type known at compile time
- Performance critical (no vtable indirection)
- Single implementor expected
- Type checking at compile time matters

**Use trait objects when:**
- Multiple types in same collection
- Type decided at runtime
- Returning different concrete types
- Plugin system or extensibility needed

## Verification Checklist

Before approving type system design:

- [ ] Every generic type parameter is used in function body or return type
- [ ] Trait bounds are minimal (only what's actually needed)
- [ ] Lifetimes don't exceed 2 parameters without documented reason
- [ ] Trait objects used only when dynamic dispatch is needed
- [ ] API is callable without turbofish notation (for most cases)
- [ ] Composition structure is justified by number of implementors
- [ ] No unnecessary traits wrapping single concrete types
- [ ] Lifetime annotations in structs are needed (not just precaution)

## Common Mistakes to Catch

1. **Generic parameters that shadow each other:**
   ```rust
   // ❌ Confusing - what's the difference?
   fn process<T, U>(a: T, b: U) where T: Display, U: Display { }

   // ✅ Use meaningful names or single generic
   fn process<T: Display>(items: [T; 2]) { }
   ```

2. **Trait bounds that belong in impl block:**
   ```rust
   // ❌ Bound every method
   impl<T: Clone> MyType<T> {
       fn duplicate(&self, item: T) -> T { item.clone() }
   }

   // ✅ Put bound only on methods that need it
   impl<T> MyType<T> {
       fn duplicate(&self, item: T) -> T where T: Clone { item.clone() }
   }
   ```

3. **Over-specifying trait bounds:**
   ```rust
   // ❌ Also Debug when not used
   fn serialize<T: Serialize + Debug>(item: T) { }

   // ✅ Just what's needed
   fn serialize<T: Serialize>(item: T) { }
   ```

## Discussion Format

When reviewing type system design:

1. **Identify the pattern:** "This function has 4 generic parameters with complex bounds"
2. **Question the necessity:** "Are all of these actually used in the function body?"
3. **Propose alternatives:** "This could be simplified with a trait object for the handler"
4. **Explain the trade-off:** "You'd get clearer intent but trade some compile-time dispatch guarantees"
5. **Suggest the change:** "Consider moving to static dispatch here unless you need runtime flexibility"

## Questions to Ask During Review

- Does every bound get used?
- Could this be two simpler functions instead of one complex generic?
- Is there only one implementor of this trait?
- Could owned data replace these lifetime parameters?
- Would a caller need to write turbofish `::<>` to call this?
- Is the trait interface stable or does it need frequent changes?
- What happens if this type parameter is removed?

## Resources for Deeper Learning

- Generic bounds: What do they cost vs. what value do they provide?
- Trait objects: When to use, common pitfalls
- Lifetime elision: When can Rust infer, when must you specify?
- API design: Ergonomics of generic signatures
