extern crate std;

use alloc::{rc::Rc, vec, vec::Vec};
use core::cell::RefCell;
use std::thread_local;

/// Safety: `ptr` must only ever be initialized with an `Rc::into_raw`'d
/// pointer to a `[U; 0]` with the same alignment as `T`, allocated on the
/// current thread.
unsafe fn get_or_make_rc<T>(ptr: &mut *const ()) -> Rc<[T; 0]> {
    let p = *ptr as *const [T; 0];
    if !p.is_null() {
        unsafe {
            Rc::increment_strong_count(p);
            return Rc::from_raw(p);
        }
    }
    let rc: Rc<[T; 0]> = Rc::new([]);
    let raw = Rc::into_raw(rc.clone());
    *ptr = raw.cast();
    rc
}
// Each element is either null or an `into_raw`'d `Rc<[U; 0]>` where U's
// alignment is 2^index. Users must Rc::increment_strong_count the
// pointer they get *before* dropping their RefCell guard.
thread_local! {
    static RAWS: RefCell<Vec<*const ()>> = const { RefCell::new(vec![]) };
}

/// Returns an [`Rc`] which points to an empty array of `T`. This `Rc` may or
/// may not share an allocation with other `Rc`s returned from this library on
/// the same thread, including those pointing to other zero-sized types.
pub fn empty_rc_array<T>() -> Rc<[T; 0]> {
    let idx: usize = core::mem::align_of::<T>()
        .ilog2()
        .try_into()
        .expect("alignment power should fit in usize");
    RAWS.with_borrow_mut(|raws| {
        if raws.len() <= idx {
            raws.resize(idx + 1, std::ptr::null());
        }
        let ptr = &mut raws[idx];
        unsafe { get_or_make_rc::<T>(ptr) }
    })
}

/// Returns an [`Rc`] which points to an empty slice of `T`. This `Rc` may or
/// may not share an allocation with other `Rc`s returned from this library on
/// the same thread, including those pointing to other zero-sized types.
#[inline]
pub fn empty_rc_slice<T>() -> Rc<[T]> {
    empty_rc_array()
}

/// Returns an [`Rc`] which points to an empty string slice. This `Rc` may or
/// may not share an allocation with other `Rc`s returned from this library on
/// the same thread, including those pointing to other zero-sized types.
pub fn empty_rc_str() -> Rc<str> {
    let rc: Rc<[u8]> = empty_rc_slice();
    debug_assert!(core::str::from_utf8(&rc).is_ok());
    unsafe { Rc::from_raw(Rc::into_raw(rc) as *const str) }
}

#[test]
fn works() {
    extern crate std;
    let _: Rc<[u16]> = empty_rc_slice();
    let _: Rc<[u16]> = empty_rc_slice();
    let _: Rc<[u16]> = empty_rc_slice();
    let _: Rc<[u16]> = empty_rc_slice();
    let u8: Rc<[u8]> = empty_rc_slice();
    let empty_str = std::thread::spawn(|| {
        let a: Rc<[u64]> = empty_rc_slice();
        let b: Rc<[u64; 0]> = empty_rc_array();
        assert!(std::ptr::eq(&a[..], &b[..]));
        let s: Rc<str> = empty_rc_str();
        Rc::into_raw(s) as *const () as usize
    });
    let _: Rc<[u32]> = empty_rc_slice();
    let empty_str = empty_str.join().unwrap();
    assert_ne!(u8.as_ptr() as usize, empty_str);
}
