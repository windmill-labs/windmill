use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::{
    alloc::{self, Layout},
    ffi::{c_char, c_int, c_void},
    mem::align_of,
    ptr,
};
use wasm_bindgen::prelude::*;

/* -------------------------------- stdlib.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn abort() {
    panic!("Aborted from C");
}

macro_rules! console_log {
    ($($t:tt)*) => (unsafe { log(&format_args!($($t)*).to_string()) })
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(a: &str);
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }

    let (layout, offset_to_data) = layout_for_size_prepended(size);
    let buf = alloc::alloc(layout);
    store_layout(buf, layout, offset_to_data)
}

#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    if count == 0 || size == 0 {
        return ptr::null_mut();
    }

    let (layout, offset_to_data) = layout_for_size_prepended(size * count);
    let buf = alloc::alloc_zeroed(layout);
    store_layout(buf, layout, offset_to_data)
}

#[no_mangle]
pub unsafe extern "C" fn realloc(buf: *mut c_void, new_size: usize) -> *mut c_void {
    if buf.is_null() {
        malloc(new_size)
    } else if new_size == 0 {
        free(buf);
        ptr::null_mut()
    } else {
        let (old_buf, old_layout) = retrieve_layout(buf);
        let (new_layout, offset_to_data) = layout_for_size_prepended(new_size);
        let new_buf = alloc::realloc(old_buf, old_layout, new_layout.size());
        store_layout(new_buf, new_layout, offset_to_data)
    }
}

#[no_mangle]
pub unsafe extern "C" fn free(buf: *mut c_void) {
    if buf.is_null() {
        return;
    }
    let (buf, layout) = retrieve_layout(buf);
    alloc::dealloc(buf, layout);
}

// In all these allocations, we store the layout before the data for later retrieval.
// This is because we need to know the layout when deallocating the memory.
// Here are some helper methods for that:

/// Given a pointer to the data, retrieve the layout and the pointer to the layout.
unsafe fn retrieve_layout(buf: *mut c_void) -> (*mut u8, Layout) {
    let (_, layout_offset) = Layout::new::<Layout>()
        .extend(Layout::from_size_align(0, align_of::<*const u8>() * 2).unwrap())
        .unwrap();

    let buf = (buf as *mut u8).offset(-(layout_offset as isize));
    let layout = *(buf as *mut Layout);

    (buf, layout)
}

/// Calculate a layout for a given size with space for storing a layout at the start.
/// Returns the layout and the offset to the data.
fn layout_for_size_prepended(size: usize) -> (Layout, usize) {
    Layout::new::<Layout>()
        .extend(Layout::from_size_align(size, align_of::<*const u8>() * 2).unwrap())
        .unwrap()
}

/// Store a layout in the pointer, returning a pointer to where the data should be stored.
unsafe fn store_layout(buf: *mut u8, layout: Layout, offset_to_data: usize) -> *mut c_void {
    *(buf as *mut Layout) = layout;
    (buf as *mut u8).offset(offset_to_data as isize) as *mut c_void
}

/* -------------------------------- string.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn strncmp(ptr1: *const c_void, ptr2: *const c_void, n: usize) -> c_int {
    let s1 = std::slice::from_raw_parts(ptr1 as *const u8, n);
    let s2 = std::slice::from_raw_parts(ptr2 as *const u8, n);

    for (a, b) in s1.iter().zip(s2.iter()) {
        if *a != *b || *a == 0 {
            return (*a as i32) - (*b as i32);
        }
    }

    0
}

// Implementation by AI:
pub type size_t = usize;
use std::slice;
#[no_mangle]
pub unsafe extern "C" fn memchr(
    haystack: *const c_void,
    needle: c_int,
    len: usize,
) -> *mut c_void {
    if haystack.is_null() || len == 0 {
        return ptr::null_mut(); // Return null if the input pointer is null or length is zero
    }

    let needle_byte = needle as u8; // Convert needle to a byte

    // Create a pointer to the start of the haystack
    let mut current = haystack as *const u8;

    // Iterate through the memory block
    for _ in 0..len {
        if *current == needle_byte {
            return current as *mut c_void; // Return the pointer to the found byte
        }
        current = current.add(1); // Move to the next byte
    }

    ptr::null_mut() // Return null if the byte was not found
}

#[no_mangle]
pub unsafe extern "C" fn strchr(mut s: *const c_char, c: c_int) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut(); // Return null if the input string is null
    }

    let target = c as u8 as char; // Convert c to a char
    let mut current = s;

    // Iterate through the string until we find the character or reach the end
    while *current != 0 {
        if *current as u8 as char == target {
            return current as *mut c_char; // Return the pointer to the found character
        }
        current = current.add(1); // Move to the next character
    }

    std::ptr::null_mut() // Return null if the character was not found
}
// End of AI implemetation 
/* -------------------------------- wctype.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn iswspace(c: c_int) -> bool {
    char::from_u32(c as u32).map_or(false, |c| c.is_whitespace())
}

#[no_mangle]
pub unsafe extern "C" fn iswalnum(c: c_int) -> bool {
    char::from_u32(c as u32).map_or(false, |c| c.is_alphanumeric())
}

// Implementation by AI:
pub type wint_t = u32;

#[no_mangle]
pub extern "C" fn iswdigit(wc: wint_t) -> c_int {
    // Check if the character is a digit ('0' to '9')
    if wc >= '0' as wint_t && wc <= '9' as wint_t {
        return 1; // Return true (1)
    }
    0 // Return false (0)
}

#[no_mangle]
pub extern "C" fn iswupper(wc: wint_t) -> c_int {
    // Check if the character is an uppercase letter ('A' to 'Z')
    if wc >= 'A' as wint_t && wc <= 'Z' as wint_t {
        return 1; // Return true (1)
    }
    0 // Return false (0)
}

#[no_mangle]
pub extern "C" fn iswalpha(wc: wint_t) -> c_int {
    // Check if the character is an alphabetic character ('A' to 'Z' or 'a' to 'z')
    if (wc >= 'A' as wint_t && wc <= 'Z' as wint_t) || (wc >= 'a' as wint_t && wc <= 'z' as wint_t) {
        return 1; // Return true (1)
    }
    0 // Return false (0)
}

#[no_mangle]
pub extern "C" fn iswlower(wc: wint_t) -> c_int {
    // Check if the character is a lowercase letter ('a' to 'z')
    if wc >= 'a' as wint_t && wc <= 'z' as wint_t {
        return 1; // Return true (1)
    }
    0 // Return false (0)
}
// End of AI implemetation 


/* --------------------------------- time.h --------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn clock() -> u64 {
    panic!("clock is not supported");
}

/* --------------------------------- ctype.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn isprint(c: c_int) -> bool {
    c >= 32 && c <= 126
}

/* --------------------------------- stdio.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn fprintf(_file: *mut c_void, _format: *const c_void, _args: ...) -> c_int {
    panic!("fprintf is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn fputs(_s: *const c_void, _file: *mut c_void) -> c_int {
    panic!("fputs is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn fputc(_c: c_int, _file: *mut c_void) -> c_int {
    panic!("fputc is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn fdopen(_fd: c_int, _mode: *const c_void) -> *mut c_void {
    panic!("fdopen is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn fclose(_file: *mut c_void) -> c_int {
    panic!("fclose is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn fwrite(
    _ptr: *const c_void,
    _size: usize,
    _nmemb: usize,
    _stream: *mut c_void,
) -> usize {
    panic!("fwrite is not supported");
}

#[no_mangle]
pub unsafe extern "C" fn vsnprintf(
    _buf: *mut c_char,
    _size: usize,
    _format: *const c_char,
    _args: ...
) -> c_int {
    panic!("vsnprintf is not supported");
}

#[no_mangle]
pub extern "C" fn clock_gettime(ptr: usize, new_size: usize) {
    panic!("clock_gettime is not supported");
}

// int snprintf( char* restrict buffer, size_t bufsz, const char* restrict format, ... );
#[no_mangle]
pub extern "C" fn snprintf() {
    panic!("snprintf is not supported");
}

#[no_mangle]
pub extern "C" fn __assert_fail(_: *const i32, _: *const i32, _: *const i32, _: *const i32) {
    panic!("oh no");
}
