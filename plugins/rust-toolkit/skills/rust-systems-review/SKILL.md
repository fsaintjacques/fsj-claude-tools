---
name: rust-systems-review
description: Review Rust code for memory safety, unsafe correctness, FFI safety, and performance - identifies undefined behavior, use-after-free, buffer overflows, alignment violations, and missing safety invariants
---

# Rust Systems and Memory Safety Review

## Overview

Review Rust code for memory safety, unsafe correctness, and performance-critical operations. This skill focuses on systems-level concerns: raw pointers, FFI, unsafe blocks, and performance.

**Core principle:** Memory safety violations are silent. Unsafe code must be treated with extreme rigor.

**Use when:** Reviewing code with `unsafe`, raw pointers, FFI, performance-critical sections, or low-level operations.

**Do NOT use this skill for:**
- Type system issues (use `rust-type-system`)
- Async correctness (use `rust-async-design`)
- Error handling strategy (use `rust-error-handling`)

## The Unsafe Code Review Process

### Golden Rule

```
Every unsafe block is a contract with the compiler.
The contract must be:
1. Explicit - What preconditions must callers satisfy?
2. Documented - Why is this unsafe?
3. Minimal - Use the smallest unsafe scope
4. Validated - Prove preconditions are met
```

### Phase 1: Understand Unsafe Preconditions

Before reviewing any unsafe code, extract the safety invariants.

**Questions to ask:**
1. What must be true for this code to be safe?
2. Who is responsible for ensuring it? (Caller or function?)
3. Is this documented?
4. Can callers accidentally violate it?

**Red flag: No documentation**
```rust
// ❌ Unsafe with no explanation
unsafe fn process(ptr: *const u8) -> u8 {
    *ptr
}
```

**Better: Clear preconditions**
```rust
/// # Safety
///
/// Caller must ensure:
/// - `ptr` points to a valid, initialized u8
/// - `ptr` is not modified concurrently
unsafe fn process(ptr: *const u8) -> u8 {
    *ptr
}
```

### Phase 2: Validate Pointer Preconditions

Raw pointers have specific requirements. Every unsafe deref must satisfy them.

**Valid pointer preconditions:**
1. **Non-null** - Does the pointer come from somewhere valid?
2. **Aligned** - Is it aligned for the type?
3. **Initialized** - Does it point to valid data?
4. **Dereferenceable** - Is the memory still allocated?
5. **Unique if mutable** - For `*mut`, is there exclusive access?

**Pattern: Unvalidated pointer deref**
```rust
// ❌ No validation
unsafe fn dereference(ptr: *const u8) -> u8 {
    *ptr  // What if ptr is null? Misaligned? Already freed?
}

// Questions:
// - Could ptr be null?
// - Could ptr be dangling?
// - Could ptr be misaligned?
```

**How to fix: Validate preconditions**
```rust
// ✅ Validated before deref
unsafe fn dereference(ptr: *const u8) -> Option<u8> {
    if ptr.is_null() {
        return None;  // Null check
    }

    if ptr.align_offset(std::mem::align_of::<u8>()) != 0 {
        return None;  // Alignment check
    }

    // Now safe to deref
    Some(*ptr)
}

// ✅ Or document preconditions
/// # Safety
///
/// Caller must ensure ptr:
/// - is non-null
/// - points to initialized u8
/// - is not accessed concurrently
unsafe fn dereference_unchecked(ptr: *const u8) -> u8 {
    *ptr
}
```

### Phase 3: Check for Use-After-Free

The most critical memory safety bug: accessing freed memory.

**Pattern: Dangling pointer**
```rust
// ❌ Use-after-free
unsafe fn dangle() {
    let data = vec![1, 2, 3];
    let ptr = data.as_ptr();
    drop(data);
    let value = *ptr;  // ❌ data is freed, ptr is dangling
}

// ❌ Subtle version
let ptr = unsafe {
    let temp = Box::new(42);
    let p = &*temp as *const i32;
    p  // temp dropped here, p is dangling
};
```

**Questions to ask:**
- How long can this pointer be held?
- What owns the memory it points to?
- Could the owner be dropped while pointer exists?
- Is the lifetime tied to borrowed data?

**Red flags:**
- Pointer returned from function with local scope
- Pointer stored in struct without lifetime
- No lifetime parameters tying pointer to data
- Memory freed before pointer use

**How to fix: Tie lifetime to borrowed data**
```rust
// ✅ Lifetime ensures pointer validity
unsafe fn process<'a>(ptr: *const u8, _borrowed: &'a [u8]) -> u8 {
    // ptr's validity is tied to borrowed lifetime
    *ptr
}

// ✅ Or wrap in owned type
struct OwnedPtr {
    data: Box<u8>,
    ptr: *const u8,
}

impl OwnedPtr {
    fn new(value: u8) -> Self {
        let data = Box::new(value);
        let ptr = &*data as *const u8;
        OwnedPtr { data, ptr }
    }

    unsafe fn deref(&self) -> u8 {
        *self.ptr
    }
}
```

### Phase 4: Check for Buffer Overflows

Writing beyond allocated memory is catastrophic.

**Pattern: Unbounded write**
```rust
// ❌ No size check
unsafe fn copy_string(src: &str, dst: *mut u8, dst_capacity: usize) {
    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    // What if src.len() > dst_capacity? Buffer overflow!
}

// ❌ Integer overflow in size calculation
fn allocate_buffer(width: u32, height: u32) -> Vec<u8> {
    let size = (width * height) as usize;  // Overflow not detected
    vec![0; size]
}
```

**Questions to ask:**
- Is there a bounds check before writing?
- Could the size calculation overflow?
- Is the destination sized correctly?
- Are overlapping regions checked?

**Red flags:**
- `.copy_nonoverlapping()` without size validation
- Cast from larger to smaller type without checking
- Multiplication without overflow check
- Pointer arithmetic without bounds

**How to fix: Validate bounds**
```rust
// ✅ Bounds checked
unsafe fn copy_string(
    src: &str,
    dst: *mut u8,
    dst_capacity: usize,
) -> Result<(), &'static str> {
    if src.len() > dst_capacity {
        return Err("Buffer too small");
    }

    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    Ok(())
}

// ✅ Overflow checked
fn allocate_buffer(width: u32, height: u32) -> Option<Vec<u8>> {
    let size = (width as u64)
        .checked_mul(height as u64)?
        .try_into()
        .ok()?;
    Some(vec![0; size])
}
```

### Phase 5: Check for Alignment Violations

Misaligned memory access is undefined behavior.

**Pattern: Assuming alignment**
```rust
// ❌ No alignment check
unsafe fn read_u32(bytes: &[u8]) -> u32 {
    let ptr = bytes.as_ptr() as *const u32;
    *ptr  // What if bytes[0] is misaligned?
}

// ❌ Alignment from cast without checking
unsafe fn cast_to_struct(data: &[u8]) -> &MyStruct {
    &*(data.as_ptr() as *const MyStruct)
}
```

**Questions to ask:**
- Is the pointer aligned for the type?
- Could the alignment requirement be violated?
- Is `#[repr(C)]` needed for FFI types?
- Are padding bytes accounted for?

**Red flags:**
- Cast to larger type without alignment check
- Raw pointer cast without `align_offset` check
- Assuming alignment without validating
- `#[repr(Rust)]` with FFI operations

**How to fix: Validate alignment**
```rust
// ✅ Alignment validated
unsafe fn read_u32_safe(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 {
        return None;
    }

    let ptr = bytes.as_ptr();
    if ptr.align_offset(std::mem::align_of::<u32>()) != 0 {
        return None;  // Not aligned
    }

    Some(*(ptr as *const u32))
}

// ✅ Use helpers
fn read_u32_from_slice(bytes: &[u8]) -> Option<u32> {
    bytes.get(0..4).map(|chunk| {
        let arr: [u8; 4] = chunk.try_into().unwrap();
        u32::from_le_bytes(arr)
    })
}
```

### Phase 6: Check FFI Safety

FFI violations often go undetected until production.

**Pattern: Unvalidated FFI**
```rust
// ❌ Borrowed data passed to C function
extern "C" {
    fn c_process(data: *const u8, len: usize);
}

unsafe fn process_with_c(data: &[u8]) {
    c_process(data.as_ptr(), data.len());
    // What if c_process stores the pointer?
    // The data may be dropped while C code holds the pointer
}

// ❌ Return value not validated
extern "C" {
    fn c_allocate() -> *mut u8;
}

unsafe fn get_c_data() -> Vec<u8> {
    let ptr = c_allocate();  // Could be null, invalid, etc
    Vec::from_raw_parts(ptr, 100, 100)  // Assumes size, ownership
}
```

**Questions to ask:**
- Does the C function store pointers? For how long?
- Who owns the returned memory?
- What's the expected size and alignment?
- Could the C function fail silently?
- Are types correctly represented?

**Red flags:**
- Borrowed data passed to C function
- No validation of returned pointers
- Assuming memory ownership
- Type mismatches (C int vs u32)
- No error handling for C failures

**How to fix: Safe FFI wrappers**
```rust
// ✅ Type-safe wrapper
#[repr(C)]
pub struct CSlice {
    data: *const u8,
    len: usize,
}

extern "C" {
    fn c_process(slice: CSlice) -> i32;  // Non-owning reference
}

pub fn safe_process(data: &[u8]) -> Result<i32, i32> {
    let result = unsafe {
        c_process(CSlice {
            data: data.as_ptr(),
            len: data.len(),
        })
    };
    if result < 0 { Err(result) } else { Ok(result) }
}

// ✅ Owned data from C
extern "C" {
    fn c_allocate(size: usize) -> *mut u8;
    fn c_deallocate(ptr: *mut u8);
}

struct CBuffer {
    ptr: *mut u8,
    len: usize,
}

impl CBuffer {
    fn allocate(size: usize) -> Option<Self> {
        let ptr = unsafe { c_allocate(size) };
        if ptr.is_null() {
            return None;
        }
        Some(CBuffer { ptr, len: size })
    }

    fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for CBuffer {
    fn drop(&mut self) {
        unsafe { c_deallocate(self.ptr) }
    }
}
```

### Phase 7: Check Unsafe Scope Minimization

Unsafe blocks should be as small as possible.

**Pattern: Large unsafe scope**
```rust
// ❌ Too much unsafe
unsafe fn process_large(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for i in 0..data.len() {
        sum += data[i] as u32;  // This could be safe!
    }
    *std::mem::zeroed::<*const u32>()  // This is the unsafe part
}

// ❌ Unsafe for everything
unsafe fn barely_unsafe(x: i32) -> i32 {
    let y = x + 1;
    let z = y * 2;
    let ptr = std::mem::zeroed::<*const i32>();
    *ptr  // Only this line is truly unsafe
}
```

**Questions to ask:**
- What lines in this unsafe block actually need to be unsafe?
- Could any of this be moved outside unsafe?
- Is there a safe wrapper?
- Can the unsafe code be extracted to a minimal function?

**Red flags:**
- Unsafe block larger than 10 lines
- Multiple unrelated unsafe operations in one block
- Safe code mixed with unsafe without clear reason
- No comment explaining why scope is large

**How to fix: Minimize unsafe scope**
```rust
// ✅ Small, focused unsafe
fn process_safe(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for &byte in data {
        sum = sum.wrapping_add(byte as u32);
    }
    sum
}

// ✅ When unsafe is needed, isolate it
unsafe fn read_pointer(ptr: *const u32) -> u32 {
    *ptr  // Only the dereference is unsafe
}

fn safe_read(ptr: *const u32) -> Option<u32> {
    if ptr.is_null() { return None; }
    Some(unsafe { *ptr })  // Minimal unsafe scope
}
```

### Phase 8: Check Drop Implementation Safety

Drop implementations are called automatically and must be safe.

**Pattern: Unsafe Drop**
```rust
// ❌ Unsafe Drop without proper documentation
struct RawBuffer {
    ptr: *mut u8,
}

impl Drop for RawBuffer {
    fn drop(&mut self) {
        unsafe {
            Vec::from_raw_parts(self.ptr, 0, 0);  // Assumes ptr was allocated by Vec
        }
    }
}

// ❌ Drop could panic
impl Drop for Resource {
    fn drop(&mut self) {
        // If this panics, Rust aborts
        unsafe { dangerous_cleanup() }
    }
}
```

**Questions to ask:**
- Does Drop use unsafe? Why?
- What are the preconditions?
- Could Drop panic?
- Is the cleanup guaranteed to work?

**Red flags:**
- Drop implementation is long
- Unsafe without clear purpose
- No documentation of assumptions
- Potential for panics

**How to fix: Safe and clear Drop**
```rust
// ✅ Clear, documented Drop
struct RawBuffer {
    ptr: *mut u8,
    len: usize,
    capacity: usize,
}

impl Drop for RawBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                // SAFETY: ptr/len/capacity were set by Vec during creation
                let _ = Vec::from_raw_parts(self.ptr, self.len, self.capacity);
            }
        }
    }
}
```

## The Memory Safety Checklist

Before approving unsafe code:

### Unsafe Preconditions
- [ ] All preconditions documented in safety comment
- [ ] Preconditions are explicit and checkable
- [ ] Caller or function responsibility is clear
- [ ] Violations are detectable or prevented

### Pointer Validity
- [ ] Non-null check if needed
- [ ] Alignment validated if needed
- [ ] Pointer doesn't outlive its data
- [ ] Lifetime properly constrained
- [ ] No use-after-free possible

### Memory Access
- [ ] Bounds checked before writing
- [ ] Buffer sizes validated
- [ ] Integer overflow in sizes prevented
- [ ] No overlapping writes without validation
- [ ] Reads access initialized memory

### Concurrency
- [ ] Concurrent access properly synchronized
- [ ] Mutable pointers have exclusive access
- [ ] No data races
- [ ] Atomics used correctly if applicable

### FFI Safety
- [ ] Type sizes match C definitions
- [ ] Alignment matches C requirements
- [ ] Pointer ownership clearly documented
- [ ] Return values validated
- [ ] Error codes checked

### Scope Minimization
- [ ] Unsafe block is as small as possible
- [ ] Safe code extracted outside unsafe
- [ ] Purpose of unsafe is clear
- [ ] Comments explain why unsafe is needed

### Drop Implementation
- [ ] Drop doesn't panic
- [ ] Resources are properly freed
- [ ] Assumptions are documented
- [ ] Order of cleanup is correct

## Common Memory Bugs

| Bug | Pattern | Fix |
|-----|---------|-----|
| Use-after-free | Pointer outlives data | Constrain lifetime or own data |
| Buffer overflow | Write without bounds | Validate size before write |
| Misalignment | Cast without alignment check | Use `align_offset()` |
| Null dereference | Don't check null | Add `if ptr.is_null()` |
| Data race | Unsync access | Use Mutex/RwLock or thread-safe types |
| Double-free | Memory freed twice | Track ownership clearly |
| Type confusion | Reinterpret without validation | Validate encoding/layout |
| Dangling ref from FFI | C function stores pointer | Document lifetime clearly |

## SAFETY Comments That Are Red Flags

**Never accept these:**
- "SAFETY: This should be fine" → Should be proof, not hope
- "SAFETY: We know this is safe" → Prove it
- "SAFETY: Pointer is valid" → Why? What makes it valid?
- "SAFETY: We tested it" → Testing isn't proof
- No SAFETY comment at all → Must have one

**Good SAFETY comments:**
```rust
// SAFETY: ptr is guaranteed non-null and aligned by precondition.
//         Caller is responsible for ensuring ptr is valid.
unsafe { *ptr }

// SAFETY: data is owned by this struct and valid for 'a.
//         Drop implementation ensures data is freed exactly once.
unsafe { std::slice::from_raw_parts(ptr, len) }
```

## Performance Considerations

Not all unsafe code is for performance, but when optimizing:

**Pattern: Premature unsafe optimization**
```rust
// ❌ Unsafe for speed when safe version is fast enough
unsafe fn sum_bytes(data: &[u8]) -> u64 {
    let mut sum = 0u64;
    for i in 0..data.len() {
        sum += *data.as_ptr().add(i) as u64;  // Unnecessary
    }
    sum
}
```

**Better approach:**
1. **Measure first** - Is the safe version actually slow?
2. **Profile** - Where is time spent?
3. **Use safe optimizations** - Iterator fusion, SIMD intrinsics
4. **Only then unsafe** - If safe optimizations insufficient

```rust
// ✅ Safe and fast
fn sum_bytes(data: &[u8]) -> u64 {
    data.iter().map(|&b| b as u64).sum()
}

// ✅ If SIMD needed, use intrinsics not unsafe pointer math
// ✅ If verified unsafe is faster, document measurements
```

## Red Flags Requiring Immediate Review

- [ ] Use-after-free pattern (pointer to local scope)
- [ ] Unvalidated pointer deref
- [ ] Buffer overflow (write without bounds)
- [ ] Alignment violation (cast without checking)
- [ ] Unsafe without SAFETY comment
- [ ] SAFETY comment that doesn't justify (see list above)
- [ ] Data race (concurrent access without sync)
- [ ] FFI without proper validation
- [ ] Drop with unsafe or panic risk

## Example: Well-Reviewed Unsafe Code

```rust
/// Safe wrapper for reading from raw memory.
///
/// # Safety
///
/// Caller must ensure:
/// - `ptr` points to a valid, initialized u32
/// - `ptr` is properly aligned (4-byte boundary)
/// - `ptr` is not accessed concurrently
unsafe fn read_u32(ptr: *const u32) -> u32 {
    debug_assert!(!ptr.is_null(), "ptr is null");
    debug_assert_eq!(
        ptr.align_offset(std::mem::align_of::<u32>()),
        0,
        "ptr is misaligned"
    );
    *ptr
}

/// Public safe interface with validation
pub fn read_u32_safe(ptr: *const u32) -> Option<u32> {
    if ptr.is_null() {
        return None;
    }

    if ptr.align_offset(std::mem::align_of::<u32>()) != 0 {
        return None;
    }

    Some(unsafe { read_u32(ptr) })
}
```
