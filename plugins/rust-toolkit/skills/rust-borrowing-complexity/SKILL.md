---
name: rust-borrowing-complexity
description: Review Rust code for borrow checker complexity, lifetime issues, and ownership patterns - identifies unnecessary lifetimes, over-complex references, self-referential patterns, and opportunities to simplify with owned data
---

# Rust Borrowing Complexity Review

## Overview

Review Rust code for unnecessary borrow complexity. Complex lifetime parameters, confusing reference patterns, and convoluted ownership usually signal a deeper design issue.

**Core principle:** If borrows are complex, the design might be wrong. Simple borrows = clear designs.

**Use when:** Reviewing lifetime parameters, reference patterns, complex borrowing relationships, or ownership decisions.

**Do NOT use this skill for:**
- Type system design in isolation (use `rust-type-system`)
- Async borrow safety (use `rust-async-design`)
- Memory safety (use `rust-systems-review`)

## The Borrowing Complexity Analysis

### Principle 1: Lifetimes are a Code Smell

More than 2 lifetime parameters is often a sign that:
- Design is over-complicated
- Owned data would simplify things
- Abstraction boundaries are unclear
- References are tying unrelated things together

**Pattern: Too many lifetimes**
```rust
// ❌ Three unrelated lifetimes
fn process<'a, 'b, 'c>(x: &'a str, y: &'b str) -> &'c str {
    // How does 'c relate to 'a and 'b?
    // What does caller expect?
    // This signature is confusing
    x
}

// ❌ Lifetime in struct that complicates usage
struct Document<'a> {
    content: &'a str,  // Why not own it?
    metadata: &'a str,  // Multiple borrows make this rigid
}

// Caller must keep source alive for entire struct lifetime
let text = String::from("hello");
let doc = Document {
    content: &text,
    metadata: &text,
};
// text must outlive doc
```

**Questions to ask:**
- How many lifetime parameters does this have?
- Do all lifetimes appear in the public API?
- Could owned data replace any of them?
- Is the relationship between lifetimes clear?
- Do callers struggle with lifetime constraints?

**Red flags:**
- 3+ lifetime parameters without clear relationship
- Lifetime parameter appears only in one field
- Multiple unrelated things borrowing different lifetimes
- Callers need to reason about multiple lifetimes
- Comments trying to explain what lifetimes mean

### Principle 2: Owned Data Often Simpler Than Borrowed

Borrowing is powerful, but owned data is simpler for callers and implementers.

**Pattern: Overly borrowed design**
```rust
// ❌ Borrowed approach - complex for callers
struct Config<'a> {
    host: &'a str,
    port: &'a str,
    username: &'a str,
}

impl<'a> Config<'a> {
    fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// Usage is awkward - caller must keep strings alive
let host = "localhost".to_string();
let port = "8080".to_string();
let username = "admin".to_string();
let config = Config {
    host: &host,
    port: &port,
    username: &username,
};

// ✅ Owned approach - simpler for callers
struct Config {
    host: String,
    port: String,
    username: String,
}

impl Config {
    fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// Usage is straightforward
let config = Config {
    host: "localhost".to_string(),
    port: "8080".to_string(),
    username: "admin".to_string(),
};
```

**Questions to ask:**
- Does the struct own this data or just reference it?
- Would the design be simpler with owned data?
- How much flexibility is gained by borrowing?
- Are there real use cases requiring zero-copy?
- Could `Cow<T>` bridge both worlds?

**Red flags:**
- Struct fields are all borrowed references
- Callers must manage lifetimes to use the struct
- Struct is used in only one context (not really generic)
- Only one implementor actually uses this struct

**When to suggest owned data:**
```
Review observation: "Field is &'a str but could be String"
Suggestion: "Consider owning the String. Callers can pass &owned_string
if borrowing is needed, but the struct becomes simpler and more flexible."
```

### Principle 3: Lifetime Elision Simplifies APIs

Rust elides (infers) lifetimes in common patterns. Explicit lifetimes should have a reason.

**Pattern: Unnecessary explicit lifetimes**
```rust
// ❌ Explicit when elision would work
fn process<'a>(input: &'a str) -> &'a str {
    input
}

// ✅ Same thing, simpler
fn process(input: &str) -> &str {
    input
}

// ❌ Too many explicit lifetimes
fn extract<'a, 'b>(items: &'a [&'b str], index: usize) -> &'b str {
    items[index]
}

// ✅ Elision handles this
fn extract(items: &[&str], index: usize) -> &str {
    items[index]
}
```

**Questions to ask:**
- Does this lifetime need to be explicit?
- Would elision work if the function was simpler?
- Is the explicit lifetime helping or hurting clarity?
- Does removing the explicit lifetime break anything?

**Red flags:**
- Single lifetime parameter that could be elided
- Lifetime appears in only one place
- Comments explaining what the lifetime means (it should be obvious)
- Elision rules apply but explicit is used anyway

**When to suggest elision:**
```
Review observation: "Function has explicit 'a but it's the only reference"
Suggestion: "This lifetime can be elided. Remove it and let Rust infer it."
```

### Principle 4: Borrowed Generics vs Owned Trade-offs

**When borrowing makes sense:**
- Function takes references to data caller owns
- Function doesn't store the reference beyond the call
- Zero-copy is important (large data, performance critical)
- Multiple types implement the trait (AsRef pattern)

**When owned makes sense:**
- Struct stores the data for later use
- Data lifetime doesn't align with caller's
- API is complex to understand due to lifetimes
- Callers would have to keep data alive anyway

**Pattern: Unnecessary generic borrowing**
```rust
// ❌ Taking borrowed generic when owned would be clearer
fn load_user<'a>(db: &Database, name: &'a str) -> User {
    // Function doesn't store the reference
    // Callers must keep name alive for no reason
    db.find(name)
}

// ✅ Take owned when simpler
fn load_user(db: &Database, name: String) -> User {
    db.find(&name)
}

// ✅ Or accept AsRef for flexibility without complexity
fn load_user<S: AsRef<str>>(db: &Database, name: S) -> User {
    db.find(name.as_ref())
}
```

### Principle 5: Self-Referential Patterns and When They're Wrong

Self-referential structs (struct with reference to its own field) are impossible without special patterns.

**Pattern: Attempted self-reference**
```rust
// ❌ This compiles but is wrong (uses 'static)
struct Node {
    value: i32,
    next: Option<&'static Node>,  // Can only point to static
}

// ❌ This doesn't compile - can't reference own field
struct LinkedList {
    value: i32,
    next: Option<&Node>,  // Borrow checker rejects this
}
```

**The pattern signals:**
- Design doesn't fit Rust's ownership model
- Should use `Rc<RefCell<T>>` or `Rc<T>` for shared ownership
- Or redesign to use indices instead of pointers
- Or use `Pin` + unsafe for careful self-reference

**Questions to ask:**
- Is this truly self-referential or just sharing?
- Would `Rc<T>` or `Rc<RefCell<T>>` work?
- Could this use an arena allocator with indices?
- Is the complexity worth it?

**Red flags:**
- struct with reference to same type (Node -> Node)
- Trying to make a linked list with references
- 'static lifetime used to work around borrow issues
- Comments saying "I know this is weird but..."

**When to suggest alternatives:**
```
Review observation: "Struct tries to reference its own type with 'static"
Suggestion: "Self-reference requires Rc<RefCell<T>> or Pin + unsafe.
Consider if the design could use indices instead (arena-style)."
```

### Principle 6: Multiple Mutable References are Usually Wrong

The borrow checker prevents multiple mutable references. If you find workarounds, reconsider the design.

**Pattern: Fighting the borrow checker with unsafe**
```rust
// ❌ Multiple mutable borrows via unsafe
struct Container {
    items: Vec<Item>,
}

impl Container {
    fn get_mut(&mut self, i: usize) -> &mut Item {
        &mut self.items[i]
    }

    fn modify_two(&mut self, i: usize, j: usize) {
        // Can't do: let a = self.get_mut(i); let b = self.get_mut(j);
        // Borrow checker rejects it

        // ❌ Using unsafe to work around the borrow checker
        unsafe {
            let a = &mut self.items[i] as *mut Item;
            let b = &mut self.items[j] as *mut Item;
            (*a).modify();
            (*b).modify();
        }
    }
}

// ✅ Better: use split_at_mut or index directly
impl Container {
    fn modify_two(&mut self, i: usize, j: usize) {
        if i == j { return; }  // Can't modify same item twice
        if i > j {
            let (left, right) = self.items.split_at_mut(i);
            left[j].modify();
            right[0].modify();
        } else {
            let (left, right) = self.items.split_at_mut(j);
            left[i].modify();
            right[0].modify();
        }
    }
}
```

**Questions to ask:**
- Why need two mutable borrows?
- Could the design avoid this?
- Is there a safe method (split_at_mut, etc)?
- Should this be two separate operations?

**Red flags:**
- unsafe blocks to handle multiple mutable references
- Comments about "working around borrow checker"
- Interior mutability (RefCell) just to avoid single mutable borrow
- Conversion to raw pointers for no safety reason

### Principle 7: Lifetime Constraints in Trait Bounds

Trait bounds can have lifetime constraints. Unnecessary constraints make the API harder to use.

**Pattern: Overly constrained lifetime**
```rust
// ❌ Trait requires 'static, limiting usage
trait CachedValue: 'static {
    fn get(&self) -> &str;
}

fn use_cache<T: CachedValue>(value: T) {
    // T must be 'static, so you can't pass borrowed data
}

// ✅ Only constrain if really needed
trait CachedValue {
    fn get(&self) -> String;  // Owned, no 'static needed
}

fn use_cache<T: CachedValue>(value: T) {
    // T can be anything now
}
```

**Questions to ask:**
- Why does this trait need 'static?
- Could the method return owned data instead?
- Are all implementors 'static?
- Are callers frustrated by this constraint?

**Red flags:**
- `'static` in trait bound without clear reason
- Comments saying "we need this to be 'static for thread safety"
- Trait implementors having to use owned data just for 'static
- Callers unable to pass certain types because of 'static

## The Borrowing Complexity Checklist

When reviewing borrowing patterns:

### Lifetime Complexity
- [ ] Number of lifetime parameters is minimal (≤ 2 usually)
- [ ] Each lifetime appears in multiple places (not isolated)
- [ ] Lifetimes have clear relationships
- [ ] Lifetime elision is used where possible
- [ ] Explicit lifetimes have a documented reason

### Ownership Decisions
- [ ] Struct owns data that needs to stay with it
- [ ] Borrowed references are only when necessary
- [ ] Callers don't struggle with lifetime constraints
- [ ] No unnecessary lifetime parameters propagating to callers
- [ ] Trade-off between borrowing and owning is justified

### Reference Patterns
- [ ] No fighting the borrow checker with unsafe
- [ ] Multiple mutable borrows avoided or handled safely
- [ ] Self-referential patterns use appropriate tools (Rc, Pin, indices)
- [ ] Interior mutability used only when appropriate
- [ ] No unnecessary raw pointers

### API Clarity
- [ ] Function signatures are simple to read
- [ ] Callers don't need to reason about multiple lifetimes
- [ ] Lifetime constraints in trait bounds are justified
- [ ] Documentation explains any unusual borrowing patterns
- [ ] No lifetime in name suggests simpler is better

### Trait Bounds
- [ ] 'static lifetime only when truly needed
- [ ] Lifetime constraints match actual usage
- [ ] Bounds enable intended flexibility without over-constraining
- [ ] No constraints that limit practical usage

## Red Flags Requiring Immediate Attention

- [ ] 3+ lifetime parameters without clear hierarchy
- [ ] Lifetime in trait bound with no justification
- [ ] Self-referential struct without Rc/Pin/indices
- [ ] unsafe blocks to work around borrow checker
- [ ] Multiple mutable borrows via raw pointers
- [ ] 'static constraint that limits usage
- [ ] Lifetime parameter appears in only one place
- [ ] Comments explaining "this is how Rust works" (design issue)
- [ ] Callers report lifetime issues frequently
- [ ] Struct fields are all borrowed, none owned

## Common Borrowing Patterns

### Pattern 1: Simple Borrowed Reference
```rust
// ✅ Clean and simple
fn process(data: &[u8]) -> usize {
    data.len()
}
```

### Pattern 2: Owned Data with Borrowed Accessors
```rust
// ✅ Struct owns, methods return borrowed slices
struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    fn as_slice(&self) -> &[u8] {
        &self.data
    }
}
```

### Pattern 3: Shared Ownership When Needed
```rust
// ✅ Using Rc for shared ownership
use std::rc::Rc;

struct Node {
    value: i32,
    children: Vec<Rc<Node>>,
}
```

### Pattern 4: Generic Over Reference Type
```rust
// ✅ Accepting any reference type
fn process<T: AsRef<[u8]>>(data: T) {
    let slice = data.as_ref();
}
```

### Pattern 5: Cow for Flexibility
```rust
// ✅ Copy-on-write for owned or borrowed
use std::borrow::Cow;

fn process(data: Cow<str>) -> String {
    format!("Processed: {}", data)
}
```

## Decision Tree for Reviewing

```
Does the struct have lifetime parameters?

  Yes?
  ├─ How many?
  │  ├─ 3+: Likely over-complex, challenge the design
  │  └─ 1-2: Is it justified?
  └─ Could any field be owned instead?
     ├─ Yes: Suggest owned, explain benefits
     └─ No: Lifetime is necessary

Does the function have lifetime parameters?

  Yes?
  ├─ Would elision work?
  │  ├─ Yes: Remove explicit lifetime
  │  └─ No: Is there a reason it can't be elided?
  └─ Do callers struggle with this?
     ├─ Yes: Consider simplifying
     └─ No: It's probably fine

Is there attempted self-reference?

  Yes?
  ├─ Using Rc/Pin/indices? → Probably correct
  └─ Using 'static or other? → Suggest proper pattern

Multiple mutable borrows?

  Yes?
  ├─ Using split_at_mut or similar? → Good
  └─ Using unsafe? → Challenge, suggest safe alternative
```

## When to Suggest Simplification

**Signals to suggest owned data:**
- Struct stores borrowed data but doesn't share it
- Only one place actually constructs this struct
- Callers report lifetime issues
- Lifetimes are the main complexity

**Signals to suggest removing lifetime:**
- Lifetime appears in only one field
- Lifetime is elided in most uses
- Explicit lifetime adds no value to signature

**Signals to suggest better pattern:**
- Self-referential attempt visible
- Fighting borrow checker with unsafe
- Multiple mutable borrow workarounds
- Comments about Rust's ownership limitations

## Example: Good Borrowing Design

```rust
// ✅ Clear ownership model
struct Config {
    name: String,      // Owned
    settings: HashMap<String, String>,  // Owned
}

impl Config {
    // Borrowed access to internal data
    fn get(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }

    // Owned for modification
    fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
}

// No lifetime parameters, clear ownership semantics
// Simple for callers to understand and use
```

## Example: Over-Complex Borrowing Design

```rust
// ❌ Unnecessarily complex
struct Config<'a> {
    name: &'a str,
    settings: &'a HashMap<String, String>,
}

impl<'a> Config<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.settings.get(key).map(|s| s.as_str())
    }
}

// Callers must keep both name and settings alive
// Lifetime parameter doesn't add value
// Owned version is simpler
```
