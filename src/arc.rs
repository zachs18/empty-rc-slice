use parking_lot::RwLock;

use alloc::{sync::Arc, vec, vec::Vec};
use core::sync::atomic::{AtomicPtr, Ordering};

/// Safety: `ptr` must only ever be initialized with an `Arc::into_raw`'d
/// pointer to a `[U; 0]` with the same alignment as `T`.
unsafe fn get_or_make_arc<T>(ptr: &AtomicPtr<()>) -> Arc<[T; 0]> {
    let p = ptr.load(Ordering::Acquire) as *const [T; 0];
    if !p.is_null() {
        unsafe {
            Arc::increment_strong_count(p);
            return Arc::from_raw(p);
        }
    }
    let arc: Arc<[T; 0]> = Arc::new([]);
    let raw = Arc::into_raw(arc.clone());
    match ptr.compare_exchange(
        core::ptr::null_mut(),
        raw.cast_mut().cast(),
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_null) => arc,
        Err(p) => {
            unsafe {
                drop(Arc::from_raw(raw));
            }
            unsafe {
                let p = p as *const [T; 0];
                // debug_assert!(p.is_aligned());
                Arc::increment_strong_count(p);
                Arc::from_raw(p)
            }
        }
    }
}

// Each element is either null or an `into_raw`'d `Arc<[U; 0]>` where U's
// alignment is 2^index. Users must Arc::increment_strong_count the
// pointer they get *before* dropping their RwLock guard.
static RAWS: RwLock<Vec<AtomicPtr<()>>> = RwLock::new(vec![]);

/// Returns an [`Arc`] which points to an empty array of `T`. This `Arc` may or
/// may not share an allocation with other `Arc`s returned from this library,
/// including those pointing to other zero-sized types.
pub fn empty_arc_array<T>() -> Arc<[T; 0]> {
    let guard = RAWS.read();
    let idx: usize = core::mem::align_of::<T>()
        .ilog2()
        .try_into()
        .expect("alignment power should fit in usize");
    match guard.get(idx) {
        Some(ptr) => unsafe { get_or_make_arc::<T>(ptr) },
        None => {
            drop(guard);
            let mut guard = RAWS.write();
            if guard.len() <= idx {
                guard.resize_with(idx + 1, || {
                    AtomicPtr::new(core::ptr::null_mut())
                });
            }
            let ptr = &guard[idx];
            unsafe { get_or_make_arc::<T>(ptr) }
        }
    }
}

/// Returns an [`Arc`] which points to an empty slice of `T`. This `Arc` may or
/// may not share an allocation with other `Arc`s returned from this library,
/// including those pointing to other zero-sized types.
#[inline]
pub fn empty_arc_slice<T>() -> Arc<[T]> {
    empty_arc_array()
}

/// Returns an [`Arc`] which points to an empty string slice. This `Arc` may or
/// may not share an allocation with other `Arc`s returned from this library,
/// including those pointing to other zero-sized types.
pub fn empty_arc_str() -> Arc<str> {
    let arc: Arc<[u8]> = empty_arc_slice();
    debug_assert!(core::str::from_utf8(&arc).is_ok());
    unsafe { Arc::from_raw(Arc::into_raw(arc) as *const str) }
}

#[test]
fn works() {
    extern crate std;
    let _: Arc<[u16]> = empty_arc_slice();
    let _: Arc<[u16]> = empty_arc_slice();
    let _: Arc<[u16]> = empty_arc_slice();
    let _: Arc<[u16]> = empty_arc_slice();
    let u8: Arc<[u8]> = empty_arc_slice();
    let empty_str = std::thread::spawn(|| {
        let a: Arc<[u64]> = empty_arc_slice();
        let b: Arc<[u64; 0]> = empty_arc_array();
        assert!(std::ptr::eq(&a[..], &b[..]));
        let s: Arc<str> = empty_arc_str();
        s
    });
    let _: Arc<[u32]> = empty_arc_slice();
    let empty_str = empty_str.join().unwrap();
    assert!(std::ptr::eq(&u8[..], empty_str.as_bytes()));
}
