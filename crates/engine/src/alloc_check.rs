//! Allocation detector for the audio thread.
//!
//! `CheckingAllocator` wraps the system allocator. While a `NoAllocGuard` is
//! alive on the current thread, any allocation, reallocation or free counts
//! as a violation; with `ABORT_ON_VIOLATION` (the default in debug builds) the
//! process aborts with a message so the offending stack is caught in a
//! debugger. Install it in the binary:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOC: engine::alloc_check::CheckingAllocator = engine::alloc_check::CheckingAllocator;
//! ```
//!
//! The thread-local flag uses a `const` initialiser with no destructor, so
//! reading it never allocates.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

thread_local! {
    static FORBID: Cell<bool> = const { Cell::new(false) };
}

/// Violations seen so far (all threads).
pub static VIOLATIONS: AtomicU64 = AtomicU64::new(0);
/// Abort on the first violation (set to false to only count).
pub static ABORT_ON_VIOLATION: AtomicBool = AtomicBool::new(cfg!(debug_assertions));

pub struct CheckingAllocator;

#[inline]
fn check(what: &'static str) {
    let forbidden = FORBID.try_with(|f| f.get()).unwrap_or(false);
    if forbidden {
        // Allow the reporting path itself to allocate.
        let _ = FORBID.try_with(|f| f.set(false));
        VIOLATIONS.fetch_add(1, Ordering::Relaxed);
        if ABORT_ON_VIOLATION.load(Ordering::Relaxed) {
            eprintln!("engine: {what} on the audio thread (forbidden); aborting");
            std::process::abort();
        }
        let _ = FORBID.try_with(|f| f.set(true));
    }
}

unsafe impl GlobalAlloc for CheckingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        check("allocation");
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        check("allocation");
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        check("deallocation");
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        check("reallocation");
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Forbids heap use on this thread while alive (only effective when
/// `CheckingAllocator` is the global allocator).
pub struct NoAllocGuard {
    prev: bool,
}

impl NoAllocGuard {
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let prev = FORBID.try_with(|f| f.replace(true)).unwrap_or(false);
        Self { prev }
    }
}

impl Drop for NoAllocGuard {
    #[inline]
    fn drop(&mut self) {
        let _ = FORBID.try_with(|f| f.set(self.prev));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard really catches allocations (the test binary installs
    /// `CheckingAllocator`; see lib.rs).
    #[test]
    fn guard_detects_allocation() {
        ABORT_ON_VIOLATION.store(false, Ordering::SeqCst);
        let before = VIOLATIONS.load(Ordering::SeqCst);
        {
            let _g = NoAllocGuard::new();
            let v = std::hint::black_box(vec![1u8; 16]);
            drop(v);
        }
        let after = VIOLATIONS.load(Ordering::SeqCst);
        ABORT_ON_VIOLATION.store(true, Ordering::SeqCst);
        assert_eq!(after - before, 2, "one allocation and one free");
        // Outside the guard: fine.
        let _v = std::hint::black_box(Box::new([0u8; 16]));
        assert_eq!(VIOLATIONS.load(Ordering::SeqCst), after);
    }
}
