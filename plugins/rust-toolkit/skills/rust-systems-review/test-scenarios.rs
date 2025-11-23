// Test scenarios for rust-systems-review skill
// Memory safety, FFI, unsafe code, and performance-critical sections

// SCENARIO 1: Unsafe block with insufficient documentation
unsafe fn process_pointer(ptr: *const u8) -> u8 {
    *ptr  // ❌ No validation that ptr is valid, aligned, dereferenceable
}

// SCENARIO 2: Unsafe transmute misuse
fn reinterpret_as_string(bytes: Vec<u8>) -> String {
    unsafe {
        String::from_utf8_unchecked(bytes)  // ❌ No validation that bytes are valid UTF-8
    }
}

// SCENARIO 3: Buffer overflow potential
unsafe fn copy_to_buffer(src: &str, dst: *mut u8, dst_size: usize) {
    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    // ❌ No check that src.len() <= dst_size
}

// SCENARIO 4: Use-after-free pattern
unsafe fn use_after_free() {
    let ptr = Box::into_raw(Box::new(42));
    drop(Box::from_raw(ptr));
    let value = *ptr;  // ❌ Use-after-free
}

// SCENARIO 5: FFI without proper type safety
#[repr(C)]
struct CStruct {
    data: *const u8,
    len: usize,
}

extern "C" {
    fn c_function(s: CStruct) -> i32;
}

unsafe fn call_c_function(data: &[u8]) -> i32 {
    c_function(CStruct {
        data: data.as_ptr(),
        len: data.len(),
    })
    // ❌ What if c_function stores the pointer? It outlives data
}

// SCENARIO 6: Undefined behavior from invalid pointer
unsafe fn invalid_pointer() {
    let ptr = std::mem::zeroed::<*const u8>();  // Null pointer
    *ptr  // ❌ UB - dereferencing null
}

// SCENARIO 7: Alignment violations
unsafe fn misaligned_read() {
    let bytes = [1u8, 2, 3, 4, 5];
    let ptr = &bytes[1] as *const u8;
    let _value = *(ptr as *const u32);  // ❌ Misaligned read
}

// SCENARIO 8: Memory layout assumptions
#[repr(C)]
struct WithPadding {
    a: u8,
    b: u32,
}

unsafe fn assume_layout() {
    let s = WithPadding { a: 1, b: 2 };
    let bytes = std::slice::from_raw_parts(
        &s as *const _ as *const u8,
        std::mem::size_of::<WithPadding>(),
    );
    // ❌ Assumes repr(C) layout, but padding may not be zero-initialized
}

// SCENARIO 9: Lifetime violation with raw pointers
unsafe fn lifetime_violation() {
    let data = vec![1, 2, 3];
    let ptr = data.as_ptr();
    drop(data);
    let _value = *ptr;  // ❌ Pointer to freed memory
}

// SCENARIO 10: Integer overflow in offset calculation
unsafe fn offset_overflow(ptr: *const u8, offset: usize) -> *const u8 {
    ptr.offset(offset as isize)  // ❌ No check for overflow/wrapping
}

// SCENARIO 11: Good unsafe: well-documented, constrained
/// # Safety
///
/// Caller must ensure:
/// - `ptr` points to a valid, initialized u32
/// - `ptr` is properly aligned for u32 (4-byte boundary)
/// - `ptr` is not accessed concurrently
unsafe fn read_u32(ptr: *const u32) -> u32 {
    *ptr  // ✅ Preconditions documented
}

// SCENARIO 12: Good FFI: type-safe wrapper
#[repr(C)]
struct SliceRef {
    data: *const u8,
    len: usize,
}

extern "C" {
    fn c_process(slice: SliceRef) -> i32;
}

fn safe_process(data: &[u8]) -> i32 {
    // ✅ Wrapped in safe interface, lifetime tied to borrowed data
    unsafe {
        c_process(SliceRef {
            data: data.as_ptr(),
            len: data.len(),
        })
    }
}

// SCENARIO 13: Integer overflow not handled
fn calculate_buffer_size(width: u32, height: u32, channels: u32) -> usize {
    (width * height * channels) as usize  // ❌ Overflow not detected
}

// SCENARIO 14: SAFETY comment without real safety
unsafe fn misleading_safety() {
    let ptr = std::mem::zeroed::<*const u8>();
    *ptr  // "SAFETY: Pointer is valid" (it's not!)
}

// SCENARIO 15: Good performance-critical code: minimal unsafe
fn process_bytes(data: &[u8]) -> u64 {
    // ✅ Unsafe only where necessary, bounds checked
    let mut hash: u64 = 0;
    for chunk in data.chunks_exact(8) {
        let bytes: [u8; 8] = chunk.try_into().unwrap();
        hash = hash.wrapping_add(u64::from_le_bytes(bytes));
    }
    hash
}

// SCENARIO 16: Unsafe with proper validation
unsafe fn validated_pointer_deref(ptr: *const u8, alignment: usize) -> Option<u8> {
    // ✅ Validates preconditions before unsafe operation
    if ptr.is_null() {
        return None;
    }

    if ptr.align_offset(alignment) != 0 {
        return None;
    }

    Some(*ptr)
}

// SCENARIO 17: Drop implementation with unsafe
struct RawBuffer {
    ptr: *mut u8,
    len: usize,
}

impl Drop for RawBuffer {
    fn drop(&mut self) {
        unsafe {
            // ✅ Well-documented that this owns the allocation
            if !self.ptr.is_null() {
                let _ = Vec::from_raw_parts(self.ptr, self.len, self.len);
            }
        }
    }
}

// SCENARIO 18: Performance regression from safety
fn slow_iteration(data: &[u8]) -> u64 {
    // ❌ Bounds check in hot loop
    let mut sum = 0u64;
    for i in 0..data.len() {
        sum = sum.wrapping_add(data[i] as u64);
    }
    sum
}

fn fast_iteration(data: &[u8]) -> u64 {
    // ✅ Single bounds check
    let mut sum = 0u64;
    for &byte in data {
        sum = sum.wrapping_add(byte as u64);
    }
    sum
}
