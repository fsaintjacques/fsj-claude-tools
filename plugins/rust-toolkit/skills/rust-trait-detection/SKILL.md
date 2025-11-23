---
name: rust-trait-detection
description: Use when reviewing Rust code and wondering which standard traits a struct should implement - provides systematic detection patterns for conversion traits (Borrow, Into, From, AsRef, AsMut) and utility traits (Clone, Default, Debug, Display) through code smell identification and requirement checklists
---

# Rust Standard Trait Detection

## Overview

Detecting missed trait implementations requires recognizing code patterns that *scream* for a trait.

**Core principle:** If a method does what a trait does, the trait should exist. If a struct has properties that enable a trait, implement it.

This reference catalogs standard library traits with systematic detection patterns, so reviewers can recognize opportunities during code review.

## Quick Reference

| Trait | Category | Detection Trigger | Effort |
|-------|----------|-------------------|--------|
| `AsRef<T>` | Conversion | `as_*(&self) -> &T` method | Derive or 1-2 lines |
| `AsMut<T>` | Conversion | `as_*_mut(&mut self) -> &mut T` method | Derive or 1-2 lines |
| `Borrow<T>` | Conversion | Struct used as HashMap key, slice lookups needed | Redirect to slice field |
| `From<T>` | Conversion | `from_*(&T or T)` constructor method | Delegate to field |
| `Into<T>` | Conversion | `to_*(&self) -> T` or `into_*(self) -> T` | Impl From + auto Into |
| `Clone` | Utility | Value cloning occurs, no Copy-incompatible fields | Derive (most common) |
| `Copy` | Utility | Only small POD data, no heap allocations | Derive if possible |
| `Default` | Utility | `new()` creates empty/sensible default | Derive or delegate |
| `Debug` | Utility | Public struct, needs debugging support | Derive or custom |
| `Display` | Utility | String representation needed for users | Custom impl |

## Conversion Traits: AsRef<T> and AsMut<T>

### When to implement

Your struct has methods like `as_bytes()`, `as_str()`, `as_inner()` that return references to internal data.

### Detection Checklist

- [ ] Struct has method `as_*(&self) -> &T` (or `as_*(self) -> &T`)
- [ ] The `T` matches a field type or slice of a field (e.g., `&self.inner[..]`)
- [ ] The method is **zero-cost** - just returns a reference with no computation, I/O, or side effects
- [ ] No validation, lazy initialization, or state checking happens in the method
- [ ] Callers might benefit from generic code accepting `impl AsRef<T>`

- [ ] (AsMut only) Struct has method `as_*_mut(&mut self) -> &mut T`
- [ ] Safe to give mutable access to internal data without breaking invariants
- [ ] The method is **zero-cost** - just returns `&mut` reference, no computation

### Code Smell Patterns

**Pattern: Redundant accessor methods**
```rust
// ❌ Code smell: as_bytes() doesn't use the trait
impl PathBuffer {
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

// ✅ Better: Implement the trait
impl AsRef<[u8]> for PathBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}
```

**Pattern: Manual conversion in caller code**
```rust
// ❌ Caller must know about as_bytes()
fn process_path(p: PathBuffer) {
    let bytes = p.as_bytes();
    // ...
}

// ✅ Generic over AsRef, works with any reference type
fn process_path<T: AsRef<[u8]>>(p: T) {
    let bytes = p.as_ref();
    // ...
}
```

### API Usage Indicator

- Callers passing to functions expecting `impl AsRef<T>`
- Multiple types needing to implement the same "get reference" behavior
- Generic abstractions that would benefit from reference polymorphism

### Example

```rust
// Before: Custom method, no trait
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }
}

fn process<P: PathBuffer>(p: P) -> usize {
    p.as_bytes().len()  // Can't be generic
}

// After: Trait implementation
impl AsRef<[u8]> for PathBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

fn process<P: AsRef<[u8]>>(p: P) -> usize {
    p.as_ref().len()  // Works with PathBuffer, &[u8], Vec<u8>, etc.
}
```

---

## Conversion Traits: From<T> and Into<T>

### When to implement

Your struct has methods named `from_*` or `to_*` / `into_*` that construct/deconstruct the struct.

### Detection Checklist

**For From<T>:**
- [ ] Method `fn from_*(value: T) -> Self` exists
- [ ] Method is **simple field wrapping** (no computation, scaling, conversion, or validation)
- [ ] The conversion is **lossless** - NO data is lost, truncated, rounded, or approximated
- [ ] Conversion **always succeeds** (never fails, panics, or validates)
- [ ] `T` is a concrete type (not a trait object)
- [ ] If conversion involves computation, scaling, or can fail → use named constructor or `TryFrom` instead

**For Into<T>:**
- [ ] Method `fn to_*(self) -> T` or `fn into_*(self) -> T` exists
- [ ] Conversion is always valid, never fails
- [ ] Converts **into** an external type (String, Vec, etc.)
- [ ] (Note: Implement `From<Self> for T` instead; `Into` is automatic)

### Code Smell Patterns

**Pattern: Constructor mirrors external type**
```rust
// ❌ Code smell: from_vec() exists but no From trait
impl PathBuffer {
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self { inner: vec }
    }
}

// ✅ Better: Use the trait
impl From<Vec<u8>> for PathBuffer {
    fn from(vec: Vec<u8>) -> Self {
        Self { inner: vec }
    }
}
```

**Pattern: "from" methods are numerous**
```rust
// ❌ Code smell: Multiple from_* methods suggest From trait
impl PathBuffer {
    pub fn from_vec(v: Vec<u8>) -> Self { /* ... */ }
    pub fn from_slice(s: &[u8]) -> Self { /* ... */ }
    pub fn from_string(s: String) -> Self { /* ... */ }
    pub fn from_str(s: &str) -> Self { /* ... */ }
}

// ✅ Better: Each becomes a From impl
impl From<Vec<u8>> for PathBuffer { /* ... */ }
impl From<&[u8]> for PathBuffer { /* ... */ }
impl From<String> for PathBuffer { /* ... */ }
impl From<&str> for PathBuffer { /* ... */ }
```

**Pattern: Consuming conversion called "to_"**
```rust
// ❌ Code smell: to_* methods that consume self usually should be From inverse
impl PathBuffer {
    pub fn to_vec(self) -> Vec<u8> {
        self.inner
    }
}

// ✅ Better: Implement From<PathBuffer> for Vec<u8>
impl From<PathBuffer> for Vec<u8> {
    fn from(pb: PathBuffer) -> Self {
        pb.inner
    }
}
```

**Anti-Pattern: Transformation or loss of data**
```rust
// ❌ WRONG: Using From for lossy conversion (truncates string)
pub struct ShortString([u8; 32]);

impl From<String> for ShortString {
    fn from(s: String) -> Self {
        let mut arr = [0u8; 32];
        let bytes = s.as_bytes();
        let len = bytes.len().min(32);  // ← LOSES data if > 32 bytes
        arr[..len].copy_from_slice(&bytes[..len]);
        Self(arr)
    }
}

// ❌ WRONG: Using From for computed conversion
pub struct Milliseconds(i64);

impl From<i64> for Milliseconds {
    fn from(seconds: i64) -> Self {
        Self(seconds * 1000)  // ← Computation, not wrapping
    }
}

// ✅ RIGHT: Use TryFrom for fallible conversions
impl TryFrom<String> for ShortString {
    type Error = &'static str;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() > 32 { return Err("string too long"); }
        // ... safe to construct
    }
}

// ✅ RIGHT: Use named constructors for transformations
impl Milliseconds {
    pub fn from_seconds(seconds: i64) -> Self {
        Self(seconds * 1000)  // Clear intent
    }
}
```

### API Usage Indicator

- `PathBuffer.into()` call site with explicit type (caller expects Into)
- Generic bounds like `fn foo<T: Into<PathBuffer>>`
- Ergonomic conversions in function arguments

### Example

```rust
// Before: Custom methods
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn from_vec(vec: Vec<u8>) -> Self {
        Self { inner: vec }
    }

    pub fn from_slice(s: &[u8]) -> Self {
        Self { inner: s.to_vec() }
    }

    pub fn to_vec(self) -> Vec<u8> {
        self.inner
    }
}

// Caller must know the specific method
let pb1 = PathBuffer::from_vec(vec);
let pb2 = PathBuffer::from_slice(&bytes);
let v = pb1.to_vec();

// After: Trait implementations
impl From<Vec<u8>> for PathBuffer {
    fn from(vec: Vec<u8>) -> Self {
        Self { inner: vec }
    }
}

impl From<&[u8]> for PathBuffer {
    fn from(s: &[u8]) -> Self {
        Self { inner: s.to_vec() }
    }
}

impl From<PathBuffer> for Vec<u8> {
    fn from(pb: PathBuffer) -> Self {
        pb.inner
    }
}

// Caller can use .into() and generics
let pb1: PathBuffer = vec.into();
let pb2: PathBuffer = bytes.into();
let v: Vec<u8> = pb1.into();

// Generic over From
fn accept<T: Into<PathBuffer>>(t: T) -> PathBuffer {
    t.into()
}
```

---

## Conversion Trait: Borrow<T>

### When to implement

Your struct is used as a HashMap/BTreeMap key, or you want to enable generic code to accept either owned or borrowed forms.

### Detection Checklist

- [ ] Struct implements `Eq` and `Hash` (used as HashMap key)
- [ ] Internal data can be viewed as a different type (e.g., `Vec<u8>` → `[u8]`)
- [ ] Callers need to look up using the slice/borrowed form
- [ ] HashMap contains `PathBuffer`, but lookups use `&[u8]`

### Code Smell Patterns

**Pattern: HashMap key lookups fail without special handling**
```rust
// ❌ Code smell: Can't look up PathBuffer with &[u8]
let mut map = HashMap::new();
map.insert(PathBuffer::from(&[1, 2, 3]), "value");

// This fails: requires &PathBuffer
// let val = map.get(&[1, 2, 3]);

// ✅ Better: Implement Borrow<[u8]>
impl Borrow<[u8]> for PathBuffer {
    fn borrow(&self) -> &[u8] {
        &self.inner
    }
}

// Now this works:
let val = map.get(&[1, 2, 3][..]);
```

**Pattern: Manual lookup conversions**
```rust
// ❌ Code smell: Caller must convert to look up
let val = map.iter()
    .find(|(k, _)| k.as_ref() == search_bytes)
    .map(|(_, v)| v);

// ✅ Better: Borrow enables direct lookup
let val = map.get(search_bytes);
```

### API Usage Indicator

- `HashMap<PathBuffer, V>` but lookups needed with `&[u8]`
- `BTreeMap<PathBuffer, V>` with slice-based queries
- Generic bounds like `fn lookup<T: Borrow<[u8]>>(key: &T)`

### Example

```rust
// Before: Can't lookup HashMap with borrowed form
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl Hash for PathBuffer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl Eq for PathBuffer {}
impl PartialEq for PathBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

let mut map: HashMap<PathBuffer, &str> = HashMap::new();
map.insert(PathBuffer::from(&[1, 2, 3]), "value");

// This fails: need PathBuffer, not &[u8]
// let val = map.get(&[1, 2, 3][..]);

// After: Implement Borrow<[u8]>
impl Borrow<[u8]> for PathBuffer {
    fn borrow(&self) -> &[u8] {
        &self.inner
    }
}

// Now slice-based lookups work:
let val = map.get(&[1, 2, 3][..]);  // Returns Some("value")
```

---

## Utility Trait: Clone

### When to implement

Your struct's fields all implement Clone and you want value semantics.

### Detection Checklist

- [ ] All fields implement Clone (safe to duplicate)
- [ ] No resource concerns (file handles, network connections, locks, database connections)
- [ ] Cloning creates an independent copy, not a shared reference
- [ ] Value semantics make sense - type name doesn't imply uniqueness (not "Connection", "Handle", "Guard")
- [ ] Callers expect to be able to duplicate the value

### Code Smell Patterns

**Pattern: Derive is available but missing**
```rust
// ❌ Code smell: All fields Clone-able, but trait not derived
pub struct PathBuffer {
    inner: Vec<u8>,  // Vec<u8> is Clone
}

// ✅ Better: Derive Clone
#[derive(Clone)]
pub struct PathBuffer {
    inner: Vec<u8>,
}
```

**Pattern: Manual clone implementations**
```rust
// ❌ Code smell: Reimplementing Clone manually
impl PathBuffer {
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// ✅ Better: Use the trait
#[derive(Clone)]
pub struct PathBuffer {
    inner: Vec<u8>,
}
```

### API Usage Indicator

- Callers calling `.clone()` on the struct
- Generic bounds like `fn foo<T: Clone>`
- Copy-like semantics expected but Copy not possible (due to Vec)

### Example

```rust
// Before: Clone method exists but no trait
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// Callers call the method directly
let pb1 = pb.clone();

// After: Derive Clone trait
#[derive(Clone)]
pub struct PathBuffer {
    inner: Vec<u8>,
}

// Works with generic code and trait bounds
let pb1 = pb.clone();
fn copy_it<T: Clone>(t: &T) -> T { t.clone() }
```

---

## Utility Trait: Default

### When to implement

Your struct has a `new()` constructor that creates an empty/sensible default state.

### Detection Checklist

- [ ] `pub fn new() -> Self` exists
- [ ] Creates empty or zero-initialized state
- [ ] No parameters required
- [ ] State is a reasonable "default" for the type

### Code Smell Patterns

**Pattern: Parameterless new() is Default**
```rust
// ❌ Code smell: new() without args is really Default
impl PathBuffer {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

// ✅ Better: Implement Default
impl Default for PathBuffer {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}
// Can also derive if fields implement Default:
#[derive(Default)]
pub struct PathBuffer {
    inner: Vec<u8>,
}
```

**Pattern: Generic code expecting Default**
```rust
// ❌ Caller must know about new()
fn make_path() -> PathBuffer {
    PathBuffer::new()
}

// ✅ Works with Default and generic code
fn make<T: Default>() -> T {
    T::default()
}
```

### API Usage Indicator

- Generic bounds like `fn foo<T: Default>`
- Struct initialization patterns expecting `Default::default()`
- Factory patterns that benefit from trait-based construction

### Example

```rust
// Before: Custom new() method
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
}

let pb = PathBuffer::new();

// After: Implement Default
impl Default for PathBuffer {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

let pb = PathBuffer::default();
// Also works in generic contexts:
let pb = <PathBuffer>::default();
```

---

## Utility Trait: Debug

### When to implement

Your struct is public and users will need to print it for debugging.

### Detection Checklist

- [ ] Struct is public (part of API)
- [ ] All fields implement Debug (or you provide custom formatting)
- [ ] Useful for logging, error messages, test output
- [ ] No sensitive data that shouldn't be printed (if so, custom impl)

### Code Smell Patterns

**Pattern: Public struct without Debug**
```rust
// ❌ Code smell: Public struct, no debug output support
pub struct PathBuffer {
    inner: Vec<u8>,  // Vec<u8> is Debug
}

// ✅ Better: Derive Debug
#[derive(Debug)]
pub struct PathBuffer {
    inner: Vec<u8>,
}
```

**Pattern: Used in assertions/tests without Debug**
```rust
// ❌ Can't print struct in test failure message
assert_eq!(result, expected);  // Unhelpful if types not Debug

// ✅ Better: Implement Debug for better error messages
#[derive(Debug)]
pub struct PathBuffer {
    inner: Vec<u8>,
}
assert_eq!(result, expected);  // Shows values on failure
```

### API Usage Indicator

- Struct appears in error/logging context
- Test code with assertions on the type
- Public API where users need inspection capability

### Example

```rust
// Before: No Debug support
pub struct PathBuffer {
    inner: Vec<u8>,
}

fn test_parsing() {
    let result = parse_path(&[1, 2, 3]);
    // If assertion fails, can't see what result was
    assert_eq!(result, expected);
}

// After: Derive Debug
#[derive(Debug)]
pub struct PathBuffer {
    inner: Vec<u8>,
}

fn test_parsing() {
    let result = parse_path(&[1, 2, 3]);
    // Assertion failure shows: `left: PathBuffer { inner: [1, 2, 3] }`
    assert_eq!(result, expected);
}
```

---

## Utility Trait: Display

### When to implement

Your struct represents something that should have a user-friendly string representation.

### Detection Checklist

- [ ] Struct represents displayable domain concept (path, name, address, etc.)
- [ ] There's a clear, meaningful string format
- [ ] Distinct from Debug (Debug = internal state, Display = user representation)
- [ ] You have a `to_string()` or similar method suggesting Display

### Code Smell Patterns

**Pattern: String conversion method suggests Display**
```rust
// ❌ Code smell: Custom string method could be Display
impl PathBuffer {
    pub fn to_display_string(&self) -> String {
        String::from_utf8_lossy(&self.inner).to_string()
    }
}

// ✅ Better: Implement Display
impl Display for PathBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.inner))
    }
}
```

**Pattern: Generic code expecting Display**
```rust
// ❌ Caller must know type-specific method
println!("{}", pb.to_display_string());

// ✅ Works with Display generic code
fn print_item<T: Display>(item: T) {
    println!("{}", item);
}
print_item(pb);
```

### API Usage Indicator

- Users will print the struct for logging/output
- Web responses, CLI output, user-facing text
- Error messages or status displays

### Example

```rust
// Before: Custom string method
pub struct PathBuffer {
    inner: Vec<u8>,
}

impl PathBuffer {
    pub fn to_display_string(&self) -> String {
        String::from_utf8_lossy(&self.inner).into_owned()
    }
}

println!("{}", pb.to_display_string());

// After: Implement Display
impl Display for PathBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.inner))
    }
}

println!("{}", pb);  // Cleaner, works in generic code
```

---

## Common Mistakes

### Mistake 1: Confusing Borrow with AsRef

| Trait | Use When | Difference |
|-------|----------|-----------|
| **AsRef** | Type can view internal data as another type | Any type, for reference polymorphism |
| **Borrow** | Type is a HashMap key needing slice lookups | Requires Eq + Hash consistency |

- **AsRef** is for "give me a reference as if I were this type"
- **Borrow** is for "I'm a key, you can look me up with my borrowed form"

### Mistake 2: Implementing From when Into should exist

- Always implement `From<T>` for your type
- `Into` is automatic via `From`
- Implement `From<YourType> for T` for the inverse (consuming conversion)
- Never manually implement `Into` - let it derive from `From`

### Mistake 3: Forgetting Borrow consistency with Hash/Eq

If you implement `Borrow<T>`, the borrowed form must hash/compare identically:

```rust
// ❌ Wrong: Hash/equality don't match
impl Borrow<[u8]> for PathBuffer {
    fn borrow(&self) -> &[u8] {
        // Returns different bytes than what was stored
        &self.inner[1..]  // Skips first byte!
    }
}

// ✅ Correct: Borrow must be exact
impl Borrow<[u8]> for PathBuffer {
    fn borrow(&self) -> &[u8] {
        &self.inner[..]  // Full content
    }
}
```

### Mistake 4: Copy without ensuring all fields Copy

- `Copy` requires ALL fields to implement `Copy`
- No exceptions - if you have a `Vec`, `String`, or reference, you can't `Copy`
- `Copy` implies implicit duplication - use when truly needed

### Mistake 5: Clone on types that shouldn't clone

- Cloning a guard type (mutex, file) usually wrong
- Cloning breaks unique ownership semantics
- If Clone exists but callers rarely use it, check if it should exist
- Be careful with Arc-wrapped types: technically cloneable but semantically confusing if the type name implies uniqueness

### Mistake 6: Using From for lossy or computed conversions

```rust
// ❌ WRONG: From for lossy conversion
impl From<String> for ShortString {  // Truncates strings > 32 bytes!
    fn from(s: String) -> Self { /* ... */ }
}

// ❌ WRONG: From for computed conversion
impl From<i64> for Milliseconds {  // Multiplies by 1000 (not wrapping)
    fn from(seconds: i64) -> Self { Self(seconds * 1000) }
}
```

- `From` must be **lossless** - no truncation, rounding, approximation, or loss of data
- `From` should be **simple field assignment** - no computation, scaling, or transformation
- Use `TryFrom` for conversions that can fail (validation, parsing, range checks)
- Use named constructors for transformations (from_seconds(), from_celsius(), etc.)

### Mistake 7: AsRef with computation or side effects

```rust
// ❌ WRONG: AsRef that does I/O or computation
impl AsRef<Path> for CachedPath {
    fn as_ref(&self) -> &Path {
        self.verify_exists()?;  // ← I/O, not zero-cost!
        &self.path
    }
}

// ❌ WRONG: AsRef with lazy initialization
impl AsRef<[u8]> for LazyBuffer {
    fn as_ref(&self) -> &[u8] {
        self.initialize_if_needed();  // ← Side effect!
        &self.inner
    }
}
```

- `AsRef` must be **zero-cost** - just returns a reference, no computation
- Callers expect `as_ref()` to be O(1) and side-effect free
- Never do validation, I/O, lazy initialization, or state checking in AsRef
- If you need to do computation, use a regular method (as_verified_path(), as_path_checked(), etc.)

### Mistake 8: Confusing AsRef with Deref

- **AsRef** is for explicit conversion in generic code: `fn process<T: AsRef<[u8]>>(t: T)`
- **Deref** is for smart pointer auto-dereferencing: `let val = *box_value;`
- Use `AsRef` for most types; `Deref` is rarely needed for custom types
- If you have `.` auto-dereferencing expectations, you probably want Deref, but reconsider - AsRef is usually better

---

## Integration Workflow

**When reviewing a Rust struct:**

1. **Scan methods** - look for `as_*`, `from_*`, `to_*`, `into_*` patterns
2. **Check fields** - what types they are, what they can do
3. **Cross-reference this guide** - find the matching trait sections
4. **Validate checklist** - does the struct meet the requirements?
5. **Suggest trait** - provide example from this guide showing before/after

**Example code review comment:**
```
I notice this struct has an `as_bytes()` method. Consider implementing
`AsRef<[u8]>` instead - this allows callers to write generic code that
works with any type that can provide a byte reference. See the AsRef
section for pattern and example.
```
