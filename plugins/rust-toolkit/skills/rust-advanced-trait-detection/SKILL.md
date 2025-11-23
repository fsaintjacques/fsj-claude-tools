---
name: rust-advanced-trait-detection
description: Use when reviewing Rust code with custom types, iterators, or collections - provides systematic detection patterns for advanced traits like IntoIterator, Deref, Index, Operator overloads, and Iterator through code smell identification and requirement checklists
---

# Rust Advanced Trait Detection

## Overview

Advanced traits enable powerful idioms: auto-dereferencing, iteration, indexing, and operator overloading.

**Core principle:** If code implements manual methods that mirror trait signatures, the trait should be implemented. If a type supports a particular operation pattern, the corresponding trait enables that generically.

This reference catalogs advanced standard library traits with systematic detection patterns for reviewers and implementers.

## Quick Reference

| Trait | Category | Detection Trigger | When Needed |
|-------|----------|-------------------|------------|
| `IntoIterator` | Iteration | `into_iter()` method exists | Types that can be consumed into items |
| `Iterator` | Iteration | `next()` method or produces items | Custom iteration logic |
| `FromIterator` | Collection | `push()` or methods accept items | Collections built from iterators |
| `Extend<T>` | Collection | Multiple items added incrementally | Collections that grow |
| `Deref` / `DerefMut` | Pointer | `deref()` / `deref_mut()` methods exist | Wrapper types with auto-deref semantics |
| `Index<T>` / `IndexMut<T>` | Access | Subscript access needed | Indexable types like sequences |
| `PartialEq` / `Eq` | Comparison | Equality checks make sense | Comparison in collections, tests |
| `Hash` | Collection Key | Type used as HashMap/HashSet key | When T implements Hash |
| `PartialOrd` / `Ord` | Comparison | Ordering makes semantic sense | Sorted collections, comparisons |
| `Drop` | Cleanup | Custom resource cleanup needed | File handles, locks, RAII patterns |
| `Add`, `Sub`, `Mul`, etc. | Operators | Type supports mathematical operations | Numbers, vectors, time durations |
| `FromStr` | Parsing | `from_string()` or parsing occurs | String conversions, configuration |
| `Error` | Errors | Type represents an error | Custom error types |

---

## Iteration Traits: IntoIterator

### When to implement

Your type has an `into_iter()` method, or users should be able to iterate over it in a `for` loop consuming values.

### Detection Checklist

- [ ] Method `fn into_iter(self) -> impl Iterator<Item = T>` exists (or similar)
- [ ] Type can be **consumed** into a sequence of items
- [ ] `for item in my_type` should work (requires IntoIterator)
- [ ] Usually paired with `iter()` and `iter_mut()` implementations

### Code Smell Patterns

**Pattern: Manual into_iter() method**
```rust
// ❌ Code smell: into_iter() exists but trait not implemented
pub struct SimpleVec<T> {
    items: Vec<T>,
}

impl<T> SimpleVec<T> {
    pub fn into_iter(self) -> impl Iterator<Item = T> {
        self.items.into_iter()
    }
}

// Can't use in for loop without explicit method call
let vec = SimpleVec::new();
for item in vec.into_iter() { }  // Must call method explicitly

// ✅ Better: Implement IntoIterator
impl<T> IntoIterator for SimpleVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

// Now works naturally:
for item in vec { }  // No method call needed!
```

**Pattern: Missing iterator variants**
```rust
// ❌ Code smell: has into_iter() but no iter()/iter_mut()
impl<T> SimpleVec<T> {
    pub fn into_iter(self) -> impl Iterator { /* ... */ }
    // Missing impl IntoIterator for &Self and &mut Self
}

// ✅ Better: All three variants
impl<T> IntoIterator for SimpleVec<T> { /* ... */ }
impl<'a, T> IntoIterator for &'a SimpleVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}
impl<'a, T> IntoIterator for &'a mut SimpleVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter_mut() }
}
```

### API Usage Indicator

- `for item in collection` syntax expected to work
- Type consumed in `for` loops
- Functions taking `impl IntoIterator<Item = T>`

### Example

```rust
// Before: Manual method only
pub struct SimpleVec<T> {
    items: Vec<T>,
}

impl<T> SimpleVec<T> {
    pub fn into_iter(self) -> impl Iterator<Item = T> {
        self.items.into_iter()
    }
}

let vec = SimpleVec::new();
for item in vec.into_iter() { }  // Ugly

// After: IntoIterator trait
impl<T> IntoIterator for SimpleVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

for item in vec { }  // Natural, idiomatic
```

---

## Pointer Traits: Deref and DerefMut

### When to implement

Your type wraps another type and users should be able to access methods on the inner type through auto-deref coercion.

### Detection Checklist

- [ ] Method `fn deref(&self) -> &T` exists (exact trait signature)
- [ ] Method `fn deref_mut(&mut self) -> &mut T` exists (for mutable variant)
- [ ] Type is a **wrapper** that should transparently provide access to inner type
- [ ] You want `.` operator auto-deref coercion to work
- [ ] NOT just for exposing a reference (use AsRef for that)

### Code Smell Patterns

**Pattern: Method named "deref" exists**
```rust
// ❌ Code smell: deref() method with trait signature
pub struct Wrapper<T> {
    inner: Box<T>,
}

impl<T> Wrapper<T> {
    pub fn deref(&self) -> &T {
        &*self.inner
    }
}

// Doesn't provide auto-deref behavior
let w = Wrapper::new(5);
(*w).to_string();  // Must manually dereference

// ✅ Better: Implement Deref
impl<T> Deref for Wrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.inner
    }
}

// Now auto-deref works:
w.to_string();  // Automatic deref coercion
```

**Anti-Pattern: Deref for non-wrapper types**
```rust
// ❌ WRONG: Implementing Deref on a non-wrapper
pub struct CustomString {
    data: Vec<u8>,
}

// Questionable - does this "is a" str or just "has" a slice?
impl Deref for CustomString {
    type Target = str;
    fn deref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.data) }
    }
}

// ✅ BETTER: Use AsRef instead
impl AsRef<str> for CustomString {
    fn as_ref(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.data) }
    }
}
```

**Anti-Pattern: Deref vs AsRef confusion**
```rust
// ❌ WRONG: Using Deref when AsRef is more appropriate
impl Deref for PathWrapper {
    type Target = Path;
    fn deref(&self) -> &Path { &self.path }
}

// ✅ RIGHT: AsRef for reference conversions
impl AsRef<Path> for PathWrapper {
    fn as_ref(&self) -> &Path { &self.path }
}

// The difference:
// Deref = "I am a pointer-like thing, dereference me for auto-coercion"
// AsRef = "I can provide you a reference, ask explicitly"
```

### API Usage Indicator

- Type wraps another and should be transparent (Box<T>, Rc<T>, etc.)
- Method calls on wrapper should reach inner type via `.`
- Auto-deref coercion desired

### Example

```rust
// Before: Manual deref method
pub struct Wrapper<T> {
    inner: T,
}

impl<T> Wrapper<T> {
    pub fn deref(&self) -> &T {
        &self.inner
    }
}

let w = Wrapper::new(vec![1, 2, 3]);
// Can't call Vec methods directly
(*w.deref()).len();  // Ugly

// After: Deref trait
impl<T> Deref for Wrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

// Vec methods available through auto-deref:
w.len();  // Just works!
w.push(4);  // Error: need DerefMut too

impl<T> DerefMut for Wrapper<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

w.push(4);  // Now works
```

---

## Indexing Traits: Index and IndexMut

### When to implement

Your type supports subscript access like `my_collection[index]`.

### Detection Checklist

- [ ] Type contains indexed/sequential data (Vec, array, slice)
- [ ] Users would naturally expect `collection[index]` syntax
- [ ] Has methods like `get(index)` or similar access pattern
- [ ] `Deref<Target = [T]>` is implemented (alternative to direct Index)

### Code Smell Patterns

**Pattern: Manual indexing methods instead of trait**
```rust
// ❌ Code smell: Custom indexing methods
pub struct SimpleVec<T> {
    items: Vec<T>,
}

impl<T> SimpleVec<T> {
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
}

// Must use .get() everywhere
let val = sv.get(0).unwrap();

// ✅ Better: Implement Index
impl<T> Index<usize> for SimpleVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.items[index]
    }
}

// Natural subscript syntax:
let val = &sv[0];
```

### API Usage Indicator

- Collection type (Vec-like, array-like)
- Indexed access is fundamental operation
- Slice indexing needed (Range, RangeFrom, etc.)

### Example

```rust
// Before: Manual indexing
impl<T> SimpleVec<T> {
    pub fn get(&self, i: usize) -> Option<&T> { /* ... */ }
}

let val = sv.get(0);

// After: Index trait
impl<T> Index<usize> for SimpleVec<T> {
    type Output = T;
    fn index(&self, index: usize) -> &T {
        &self.items[index]
    }
}

impl<T> Index<Range<usize>> for SimpleVec<T> {
    type Output = [T];
    fn index(&self, range: Range<usize>) -> &[T] {
        &self.items[range]
    }
}

let val = &sv[0];
let slice = &sv[0..5];
```

---

## Collection Traits: FromIterator and Extend

### When to implement

Your type is a collection that can be built from items or extended with items.

### Detection Checklist

**For FromIterator:**
- [ ] Has `push()` or similar item-adding method
- [ ] Makes sense to construct from an iterator
- [ ] Users would expect `.collect::<MyType>()`
- [ ] Similar types (Vec, HashSet, etc.) implement this

**For Extend:**
- [ ] Has methods that add multiple items
- [ ] Can be extended in-place efficiently
- [ ] Pairs with `push()` or append functionality

### Code Smell Patterns

**Pattern: Manual from_iter constructor**
```rust
// ❌ Code smell: Custom collector method
pub struct SimpleVec<T> {
    items: Vec<T>,
}

impl<T> SimpleVec<T> {
    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self { items: Vec::from_iter(iter) }
    }
}

// Must use custom method
let v: SimpleVec<i32> = SimpleVec::from_iter(0..10);

// ✅ Better: Implement FromIterator
impl<T> FromIterator<T> for SimpleVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self { items: Vec::from_iter(iter) }
    }
}

// Works with standard collect:
let v: SimpleVec<i32> = (0..10).collect();
```

**Pattern: Manual extend method**
```rust
// ❌ Code smell: Custom extend-like method
impl<T> SimpleVec<T> {
    pub fn add_all<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.items.push(item);
        }
    }
}

sv.add_all(0..10);

// ✅ Better: Implement Extend
impl<T> Extend<T> for SimpleVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.items.extend(iter);
    }
}

sv.extend(0..10);
```

### API Usage Indicator

- `.collect::<MyType>()` should work
- Construction from iterators is common
- Efficient bulk addition of items

### Example

```rust
// Before: Manual collectors
impl<T> SimpleVec<T> {
    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self { /* ... */ }
    pub fn add_all<I: IntoIterator<Item = T>>(&mut self, iter: I) { /* ... */ }
}

let v = SimpleVec::from_iter(0..10);
sv.add_all(vec![1, 2, 3]);

// After: Trait implementations
impl<T> FromIterator<T> for SimpleVec<T> { /* ... */ }
impl<T> Extend<T> for SimpleVec<T> { /* ... */ }

let v: SimpleVec<i32> = (0..10).collect();
sv.extend(vec![1, 2, 3]);
```

---

## Comparison Traits: PartialEq, Eq, Hash, Ord

### When to implement

Your type supports comparison or can be used in collections.

### Detection Checklist

**PartialEq / Eq:**
- [ ] Type has fields that support comparison
- [ ] Equality semantics make sense
- [ ] Used in tests, assertions, or collections
- [ ] All fields implement PartialEq (for derivation)

**Hash:**
- [ ] Type will be used as HashMap/HashSet key
- [ ] Implement only if PartialEq is also implemented
- [ ] Hash and Eq must be consistent (if a == b, hash(a) == hash(b))

**PartialOrd / Ord:**
- [ ] Type has meaningful ordering
- [ ] Used in sorted structures or comparisons
- [ ] Ordering should be consistent with equality

### Code Smell Patterns

**Pattern: Missing comparisons on comparable types**
```rust
// ❌ Code smell: Wrapper with comparable fields but no PartialEq
pub struct Timestamp(i64);

impl Timestamp {
    pub fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

// Must use custom method
if timestamp.eq(&other) { }

// ✅ Better: Implement PartialEq/Eq
#[derive(PartialEq, Eq)]
pub struct Timestamp(i64);

// Standard comparison:
if timestamp == other { }
```

**Anti-Pattern: Missing Hash when using as key**
```rust
// ❌ WRONG: Used in HashMap but Hash not implemented
#[derive(PartialEq, Eq)]
pub struct UserId(u32);

let mut map = HashMap::new();
map.insert(UserId(1), "user");  // Compile error!

// ✅ CORRECT: Implement Hash
#[derive(PartialEq, Eq, Hash)]
pub struct UserId(u32);

let mut map = HashMap::new();
map.insert(UserId(1), "user");  // Works
```

**Anti-Pattern: Inconsistent Hash and Eq**
```rust
// ❌ WRONG: Hash and Eq don't match
#[derive(Hash)]
pub struct Key {
    id: u32,
    name: String,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id  // Only compares id, ignores name
    }
}

// This violates Hash contract: if a == b, hash(a) must equal hash(b)
// But hash includes both fields!
```

### API Usage Indicator

- Type appears in tests with equality checks
- HashMap/HashSet key type
- Used in sorted containers (BTreeSet, BTreeMap)

### Example

```rust
// Before: Manual comparisons
pub struct UserId(u32);

impl UserId {
    pub fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

// After: Derive traits
#[derive(PartialEq, Eq, Hash)]
pub struct UserId(u32);

// Now works in standard ways:
if user_id == other_id { }  // PartialEq
let mut set = HashSet::new();
set.insert(user_id);  // Hash + Eq
```

---

## Resource Trait: Drop

### When to implement

Your type holds resources that need cleanup (files, locks, network connections).

### Detection Checklist

- [ ] Type holds resource handles (file, socket, lock, database connection)
- [ ] Cleanup is needed when value is dropped
- [ ] Default drop (just dropping fields) isn't sufficient
- [ ] RAII pattern: acquisition and cleanup must be paired
- [ ] **Critical:** Cleanup will be infallible and fast (no I/O, no blocking, no exceptions)

### Code Smell Patterns

**Pattern: Open without guaranteed close**
```rust
// ❌ Code smell: File handle with no cleanup guarantee
pub struct FileWriter {
    file: std::fs::File,
}

impl FileWriter {
    pub fn new(path: &str) -> Self {
        Self { file: std::fs::File::create(path).unwrap() }
    }

    pub fn close(&mut self) {
        drop(std::mem::take(&mut self.file));  // Manual close needed
    }
}

// Forgetting close() means file not flushed/closed
let mut fw = FileWriter::new("file.txt");
fw.write_all(b"data").unwrap();
// Oops, forgot close() - file not properly closed

// ✅ Better: Implement Drop
impl Drop for FileWriter {
    fn drop(&mut self) {
        // Cleanup happens automatically
        // File is flushed and closed
    }
}

let mut fw = FileWriter::new("file.txt");
fw.write_all(b"data").unwrap();
// Automatically closed when fw is dropped
```

**Pattern: Mutex guard without automatic unlock**
```rust
// ❌ Code smell: Manual lock/unlock needed
pub struct GuardedCounter {
    mutex: Arc<Mutex<i32>>,
}

impl GuardedCounter {
    pub fn increment(&self) -> i32 {
        let mut guard = self.mutex.lock().unwrap();
        *guard += 1;
        drop(guard);  // Manual unlock (Mutex drop impl calls this)
        *guard  // ERROR: use-after-free!
    }
}

// ✅ CORRECT: Mutex already implements Drop
impl GuardedCounter {
    pub fn increment(&self) -> i32 {
        let mut guard = self.mutex.lock().unwrap();
        *guard += 1;
        // Guard is automatically dropped here, unlocking mutex
        // (Though can't use guard after this point)
    }
}
```

### API Usage Indicator

- Struct wraps resource handles
- Must guarantee cleanup on exit
- Error if cleanup is forgotten

### Example

```rust
// Before: Manual cleanup
pub struct Connection {
    socket: TcpStream,
}

impl Connection {
    pub fn close(&mut self) {
        drop(std::mem::take(&mut self.socket));
    }
}

let mut conn = Connection::new("127.0.0.1:8080")?;
conn.send(data)?;
conn.close();  // Must remember!

// After: Implement Drop
impl Drop for Connection {
    fn drop(&mut self) {
        // Automatic cleanup
    }
}

let mut conn = Connection::new("127.0.0.1:8080")?;
conn.send(data)?;
// Automatic cleanup when conn is dropped
```

---

## Iterator Trait

### When to implement

Your type produces a sequence of items through iteration.

### Detection Checklist

- [ ] Custom iteration logic needed (not just delegating to inner iterator)
- [ ] Has state that changes across iteration steps
- [ ] Implements or wraps an `Iterator`
- [ ] Significant enough to warrant custom Iterator implementation

### Code Smell Patterns

**Pattern: Struct that produces items**
```rust
// ❌ Code smell: Custom next-like logic in struct
pub struct RangeDoubler {
    current: i32,
    end: i32,
}

impl RangeDoubler {
    pub fn next(&mut self) -> Option<i32> {
        if self.current <= self.end {
            let val = self.current * 2;
            self.current += 1;
            Some(val)
        } else {
            None
        }
    }
}

// Must call .next() manually
let mut range = RangeDoubler::new(1, 5);
while let Some(val) = range.next() {
    println!("{}", val);
}

// ✅ Better: Implement Iterator
impl Iterator for RangeDoubler {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        // Same implementation
    }
}

// Works with for loop and iterator adapters:
for val in RangeDoubler::new(1, 5) {
    println!("{}", val);
}
```

### API Usage Indicator

- Type with custom iteration state
- Should work in `for` loops with custom logic
- Iterator combinators (.map, .filter, etc.) should work

### Example

```rust
// Before: Manual iteration
impl RangeDoubler {
    pub fn next(&mut self) -> Option<i32> { /* ... */ }
}

// After: Iterator trait
impl Iterator for RangeDoubler {
    type Item = i32;
    fn next(&mut self) -> Option<i32> { /* ... */ }
}

// Both enable iteration, but Iterator is idiomatic
```

---

## Operator Overloading Traits

### When to implement

Your type supports mathematical, logical, or other operations.

### Detection Checklist

- [ ] Type has clear semantics for operation (Add, Sub, Mul, etc.)
- [ ] Operation produces meaningful result
- [ ] Existing `add()`, `subtract()`, or similar methods exist
- [ ] Natural for users to use operator syntax

### Code Smell Patterns

**Pattern: Arithmetic methods instead of operators**
```rust
// ❌ Code smell: add() method instead of + operator
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn add(&self, other: &Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

let p = p1.add(&p2);

// ✅ Better: Implement Add
impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

let p = p1 + p2;
```

**Anti-Pattern: Operators where semantics unclear**
```rust
// ❌ WRONG: Overloading + for something not additive
impl Add for UserId {  // What does UserId + UserId mean?
    type Output = UserId;
    fn add(self, other: UserId) -> UserId {
        UserId(self.0 + other.0)
    }
}

// Confusing - users expect + to be meaningful
let id = user_id1 + user_id2;  // What does this mean?
```

### API Usage Indicator

- Type represents mathematical entity (numbers, vectors, durations)
- Operations have obvious semantics
- Cleaner with operator syntax

### Example

```rust
// Before: Method calls
let result = p1.add(&p2);
let difference = p1.subtract(&p2);

// After: Operator traits
impl Add for Point { /* ... */ }
impl Sub for Point { /* ... */ }

let result = p1 + p2;
let difference = p1 - p2;
```

---

## Parsing Trait: FromStr

### When to implement

Your type can be constructed from a string.

### Detection Checklist

- [ ] Has `from_str()` or `parse_string()` method
- [ ] Conversion from String is common operation
- [ ] Users would expect `"value".parse::<MyType>()`
- [ ] May fail (syntax error, out of range) → `Result<Self, Err>`

### Code Smell Patterns

**Pattern: Custom parse method instead of trait**
```rust
// ❌ Code smell: from_str method instead of trait
pub struct UserId(u32);

impl UserId {
    pub fn from_str(s: &str) -> Result<Self, std::num::ParseIntError> {
        Ok(UserId(s.parse()?))
    }
}

let id = UserId::from_str("42")?;

// ✅ Better: Implement FromStr
impl std::str::FromStr for UserId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserId(s.parse()?))
    }
}

let id: UserId = "42".parse()?;
```

### API Usage Indicator

- Parsing from user input, config files
- `String::parse()` should work
- Interactive or configuration-heavy code

### Example

```rust
// Before: Custom method
let id = UserId::from_str("42")?;

// After: FromStr trait
let id: UserId = "42".parse()?;
```

---

## Anti-Patterns in Drop

**Drop must be infallible and fast (no I/O, no blocking, no exceptions):**

```rust
// ❌ WRONG: I/O and network operations in Drop
impl Drop for Logger {
    fn drop(&mut self) {
        println!("Logger dropped");        // I/O can fail (broken pipe, disk full)
        self.file.flush().unwrap();        // Can panic
        self.send_telemetry().unwrap();    // Network I/O, blocking
    }
}

// Why this is dangerous:
// 1. Drop runs during panic unwinding - any failure = double panic = abort
// 2. println! and I/O operations can fail
// 3. Blocking operations in Drop hurt async performance severely
// 4. If wrapped in Arc<Mutex<T>>, Drop might not run if references exist

// ❌ WRONG: Deref for API layering (mimicking inheritance)
pub struct PathBuilder { path: PathBuf }

impl Deref for PathBuilder {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf { &self.path }
}

impl PathBuilder {
    pub fn and_then(self, segment: &str) -> Self { /* ... */ }
}

// Confusing: unclear what type this is
let builder = PathBuilder::new();
builder.push("foo");       // PathBuf method via Deref
builder.and_then("bar");   // Builder method
// Users confused: is this PathBuf or a builder?

// ✅ CORRECT: Explicit cleanup without I/O
impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.file.flush();  // Suppress errors gracefully
    }
}

// ✅ BETTER: Explicit cleanup method + emergency Drop
impl Logger {
    pub fn close(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.send_telemetry()?;
        Ok(())
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.close();  // Best effort, errors ignored
    }
}

// ✅ CORRECT: Don't use Deref for API layering
impl PathBuilder {
    pub fn as_path(&self) -> &Path { &self.path }
    pub fn push(mut self, segment: &str) -> Self {
        self.path.push(segment);
        self
    }
}

// Clear API - all methods on Builder are builder methods
let builder = PathBuilder::new();
let path = builder.push("foo").and_then("bar").as_path();
```

---

## Common Mistakes

### Mistake 1: IntoIterator vs Iterator

- **IntoIterator** = "I can be consumed into an iterator" (Consume self)
- **Iterator** = "I produce items as you call next()"
- Implement both when appropriate: IntoIterator for outer type, Iterator for the iterator it produces

### Mistake 2: Deref vs AsRef

- **Deref** = Smart pointer auto-deref: `Box<T>`, `Rc<T>`, wrapper types
- **AsRef** = Reference conversion: explicit in generics `<T: AsRef<[u8]>>`
- If you need auto-deref (`.` operator), use Deref
- If you just need reference conversion, use AsRef
- Rarely implement both on same type

### Mistake 3: Index vs Deref + Index

```rust
// ❌ Usually don't need both
impl Deref for SimpleVec<T> {
    type Target = [T];  // Deref to slice
    fn deref(&self) -> &[T] { &self.items }
}
impl Index<usize> for SimpleVec<T> { /* ... */ }  // Redundant!

// ✅ Better: Just Deref provides indexing through coercion
// or just Index if you don't want Deref
```

### Mistake 4: FromIterator without Extend

- These pair naturally - FromIterator builds from scratch, Extend adds to existing
- If collection has push(), usually should implement both
- Asymmetry often indicates missing trait

### Mistake 5: Hash without Eq, or with inconsistent Eq

```rust
// ❌ WRONG: Hash without Eq
#[derive(Hash)]
pub struct Key(u32);
// Can't use in HashMap - missing Eq

// ❌ WRONG: Hash and Eq don't align
#[derive(Hash)]
pub struct Key {
    id: u32,
    extra: u32,
}

impl Eq for Key {}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id  // Ignores extra!
    }
}

// ✅ CORRECT: Hash and Eq consistent
#[derive(Hash, Eq, PartialEq)]
pub struct Key {
    id: u32,
}
```

### Mistake 6: Drop with panics

```rust
// ❌ WRONG: Panic in Drop
impl Drop for Handler {
    fn drop(&mut self) {
        self.validate().unwrap();  // Can panic!
    }
}

// ✅ CORRECT: Handle errors gracefully
impl Drop for Handler {
    fn drop(&mut self) {
        let _ = self.validate();  // Ignore errors
    }
}
```

### Mistake 7: Deref for type-safety newtypes defeats encapsulation

**Type-safety newtypes exist to PREVENT mixing of different types:**

```rust
// ❌ WRONG: Newtype with Deref defeats type safety
pub struct UserId(u32);
pub struct OrderId(u32);

impl Deref for UserId {
    type Target = u32;
    fn deref(&self) -> &u32 { &self.0 }
}

// Now you can accidentally mix them:
let user = UserId(42);
let order = OrderId(99);
if *user == *order { }  // Compiles! Type distinction lost.

// ✅ CORRECT: No Deref for newtypes that exist for type distinction
// Deref should only be used for:
// - Smart pointers (Box<T>, Rc<T>, Arc<T>)
// - Wrappers where transparency IS the purpose (not type safety)

// If you need transparent access, use AsRef instead:
impl AsRef<u32> for UserId {
    fn as_ref(&self) -> &u32 { &self.0 }
}

// Users explicitly opt-in:
let val = user.as_ref();  // Clear intent, type safety preserved
```

Rule: **Never implement Deref for newtypes used for type distinction.** Deref defeats the entire purpose of newtypes. Use Deref only for smart pointer-like types.

### Mistake 8: Hash and Eq with different semantic bases

**Hash and Eq must evaluate the SAME criteria:**

```rust
// ❌ WRONG: Hash and Eq use different semantics
#[derive(Hash)]  // Hashes by fd only
pub struct FileHandle { fd: i32 }

impl PartialEq for FileHandle {
    fn eq(&self, other: &Self) -> bool {
        get_inode(self.fd) == get_inode(other.fd)  // Different!
    }
}

// Two handles with different fds but same inode:
// hash(fd=3) != hash(fd=5)              // Different hashes
// FileHandle(3) == FileHandle(5)        // Equal!
// HashMap lookup fails: finds hash-bucket for fd=3, but key compares equal to fd=5
// Data corruption and silent bugs!

// ❌ WRONG: Both manual but with different criteria still fails
impl Hash for FileHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.fd.hash(state);  // Hash by fd
    }
}

impl PartialEq for FileHandle {
    fn eq(&self, other: &Self) -> bool {
        get_inode(self.fd) == get_inode(other.fd)  // Compare by inode
    }
}

// ✅ CORRECT: Hash and Eq use identical criteria
impl Hash for FileHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        get_inode(self.fd).hash(state);  // Hash what you compare!
    }
}

impl PartialEq for FileHandle {
    fn eq(&self, other: &Self) -> bool {
        get_inode(self.fd) == get_inode(other.fd)  // Same basis
    }
}
```

Rule: **Hash and Eq must use identical evaluation criteria.** When in doubt, derive both or manually implement both using exactly the same fields.

### Mistake 9: Operator overloading with ambiguous semantics

**Operations must have ONE unambiguous interpretation in the problem domain:**

```rust
// ❌ UNCLEAR: What does Time + Time mean?
impl Add<Duration> for Time {
    type Output = Time;
    fn add(self, duration: Duration) -> Time { /* ... */ }
}

// Two plausible interpretations:
// - Add durations together (nonsensical for Time)
// - Add duration to timestamp (add, not plus)

// ❌ NONSENSICAL: What would UserId + UserId even mean?
impl Add for UserId {
    type Output = UserId;
    fn add(self, other: UserId) -> UserId {
        UserId(self.0 + other.0)  // Meaningless
    }
}

// ✅ CLEAR: Unambiguous single interpretation
impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, duration: Duration) -> Instant {
        // + a duration to a timestamp: clear, unambiguous
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        // Vector addition: clear, mathematical
    }
}
```

Rule: **Only implement operators if the semantics are mathematically or domain-wise unambiguous.** When in doubt, use named methods instead (`.add_duration()`, `.combine_with()`, etc.).

### Mistake 10: IntoIterator with borrowed references

```rust
// ❌ WRONG: IntoIterator for &MyType doesn't consume
impl IntoIterator for &SimpleVec<T> {
    type Item = T;  // Should be &T!
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.clone().into_iter()  // Clone just to consume?
    }
}

// ✅ CORRECT: Borrowed iterators yield references
impl<'a, T> IntoIterator for &'a SimpleVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}
```

### Mistake 11: Consuming methods returning Iterator that should be IntoIterator

**If a method consumes self and returns Iterator, it IS IntoIterator::into_iter:**

```rust
// ❌ WRONG: Consuming method with custom name
impl<T> MyCollection<T> {
    pub fn iter(self) -> impl Iterator<Item = T> {  // Takes self, consumes it
        CustomIter { items: self.items, index: 0 }
    }
}

// Must call method explicitly - not idiomatic:
for item in collection.iter() { }  // iter() call visible

// ✅ CORRECT: This IS IntoIterator
impl<T> IntoIterator for MyCollection<T> {
    type Item = T;
    type IntoIter = CustomIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        CustomIter { items: self.items, index: 0 }
    }
}

// Natural idiomatic syntax:
for item in collection { }  // No method call needed
```

Rule: **If a method takes `self` (not `&self` or `&mut self`) and returns an Iterator, rename it to `into_iter()` and implement the IntoIterator trait.**

---

## Integration Workflow

**When reviewing code with advanced patterns:**

1. **Identify iteration** - Are there `iter()`, `into_iter()` methods? → IntoIterator
2. **Check for dereferencing** - Are there `deref()` or `deref_mut()` methods? → Deref/DerefMut
3. **Look for indexing** - Can this type be indexed? → Index/IndexMut
4. **Verify comparison** - Used in collections or comparisons? → PartialEq/Eq/Hash/Ord
5. **Resource checks** - Does it hold handles? → Drop
6. **Collection patterns** - Does it collect or extend? → FromIterator/Extend
7. **Operators** - Are there operation methods? → Operator traits
8. **String conversion** - Does it parse strings? → FromStr

**Example code review comment:**
```
This type has an `into_iter()` method and stores a Vec. Consider implementing
IntoIterator for all three variants (&Self, &mut Self, Self) to enable
for-loop iteration. See the rust-advanced-trait-detection skill for patterns.
```
