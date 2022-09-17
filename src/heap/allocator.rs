use super::pool::{Fits, Pool};
use core::{
    alloc::{AllocError, Layout},
    ptr,
    ptr::NonNull,
    slice::SliceIndex,
};

/// Allocator for a generic memory pools layout.
///
/// The trait is supposed to be implemented for an array of pools.
/// [`heap`](crate::heap) macro should be used to generate the concrete type and
/// the implementation.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub trait Allocator: Sized {
    /// The total number of memory pools.
    const POOL_COUNT: usize;

    /// Stream for heap tracing. Heap tracing is disabled if `None`.
    const TRACE_STREAM: Option<u8>;

    /// Returns a reference to a pool or subslice, without doing bounds
    /// checking.
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is Undefined Behavior.
    unsafe fn get_pool_unchecked<I>(&self, index: I) -> &I::Output
    where
        I: SliceIndex<[Pool]>;
}

/// Does a binary search for the pool with the smallest block size to fit
/// `value`.
pub fn binary_search<A: Allocator, T: Fits>(heap: &A, value: T) -> usize {
    let (mut left, mut right) = (0, A::POOL_COUNT);
    while right > left {
        let middle = left + ((right - left) >> 1);
        let pool = unsafe { heap.get_pool_unchecked(middle) };
        if value.fits(pool) {
            right = middle;
        } else {
            left = middle + 1;
        }
    }
    left
}

#[doc(hidden)]
pub fn allocate<A: Allocator>(heap: &A, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
    if let Some(trace_stream) = A::TRACE_STREAM {
        trace::allocate(trace_stream, layout);
    }
    if layout.size() == 0 {
        return Ok(NonNull::slice_from_raw_parts(layout.dangling(), 0));
    }
    for pool_idx in binary_search(heap, &layout)..A::POOL_COUNT {
        let pool = unsafe { heap.get_pool_unchecked(pool_idx) };
        if let Some(ptr) = pool.allocate() {
            return Ok(NonNull::slice_from_raw_parts(ptr, pool.size()));
        }
    }
    Err(AllocError)
}

#[doc(hidden)]
pub fn allocate_zeroed<A: Allocator>(
    heap: &A,
    layout: Layout,
) -> Result<NonNull<[u8]>, AllocError> {
    let ptr = allocate(heap, layout)?;
    unsafe { ptr.as_non_null_ptr().as_ptr().write_bytes(0, ptr.len()) }
    Ok(ptr)
}

#[doc(hidden)]
pub unsafe fn deallocate<A: Allocator>(heap: &A, ptr: NonNull<u8>, layout: Layout) {
    if let Some(trace_stream) = A::TRACE_STREAM {
        trace::deallocate(trace_stream, layout);
    }
    if layout.size() == 0 {
        return;
    }
    unsafe {
        let pool = heap.get_pool_unchecked(binary_search(heap, ptr));
        pool.deallocate(ptr);
    }
}

#[doc(hidden)]
pub unsafe fn grow<A: Allocator>(
    heap: &A,
    ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError> {
    if let Some(trace_stream) = A::TRACE_STREAM {
        trace::grow(trace_stream, old_layout, new_layout);
    }
    unsafe {
        let new_ptr = allocate(heap, new_layout)?;
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), old_layout.size());
        deallocate(heap, ptr, old_layout);
        Ok(new_ptr)
    }
}

#[doc(hidden)]
pub unsafe fn grow_zeroed<A: Allocator>(
    heap: &A,
    ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError> {
    if let Some(trace_stream) = A::TRACE_STREAM {
        trace::grow(trace_stream, old_layout, new_layout);
    }
    unsafe {
        let new_ptr = allocate_zeroed(heap, new_layout)?;
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), old_layout.size());
        deallocate(heap, ptr, old_layout);
        Ok(new_ptr)
    }
}

#[doc(hidden)]
pub unsafe fn shrink<A: Allocator>(
    heap: &A,
    ptr: NonNull<u8>,
    old_layout: Layout,
    new_layout: Layout,
) -> Result<NonNull<[u8]>, AllocError> {
    if let Some(trace_stream) = A::TRACE_STREAM {
        trace::shrink(trace_stream, old_layout, new_layout);
    }
    unsafe {
        let new_ptr = allocate(heap, new_layout)?;
        ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut_ptr(), new_layout.size());
        deallocate(heap, ptr, old_layout);
        Ok(new_ptr)
    }
}

mod trace {
    use crate::stream::Stream;
    use core::{alloc::Layout, mem};

    #[inline(always)]
    pub(super) fn allocate(trace_stream: u8, layout: Layout) {
        #[inline(never)]
        fn trace(trace_stream: u8, layout: Layout) {
            let buffer: [usize; 2] = [0_usize.to_be(), layout.size()];
            let buffer: [u8; mem::size_of::<[usize; 2]>()] = unsafe { mem::transmute(buffer) };
            Stream::new(trace_stream).write_transaction(&buffer[3..]);
        }
        if Stream::new(trace_stream).is_enabled() {
            trace(trace_stream, layout);
        }
    }

    #[inline(always)]
    pub(super) fn deallocate(trace_stream: u8, layout: Layout) {
        #[inline(never)]
        fn trace(trace_stream: u8, layout: Layout) {
            let buffer: [usize; 2] = [1_usize.to_be(), layout.size()];
            let buffer: [u8; mem::size_of::<[usize; 2]>()] = unsafe { mem::transmute(buffer) };
            Stream::new(trace_stream).write_transaction(&buffer[3..]);
        }
        if Stream::new(trace_stream).is_enabled() {
            trace(trace_stream, layout);
        }
    }

    #[inline(always)]
    pub(super) fn grow(trace_stream: u8, old_layout: Layout, new_layout: Layout) {
        #[inline(never)]
        fn trace(trace_stream: u8, old_layout: Layout, new_layout: Layout) {
            let buffer: [usize; 3] = [2_usize.to_be(), old_layout.size(), new_layout.size()];
            let buffer: [u8; mem::size_of::<[usize; 3]>()] = unsafe { mem::transmute(buffer) };
            Stream::new(trace_stream).write_transaction(&buffer[3..]);
        }
        if Stream::new(trace_stream).is_enabled() {
            trace(trace_stream, old_layout, new_layout);
        }
    }

    #[inline(always)]
    pub(super) fn shrink(trace_stream: u8, old_layout: Layout, new_layout: Layout) {
        #[inline(never)]
        fn trace(trace_stream: u8, old_layout: Layout, new_layout: Layout) {
            let buffer: [usize; 3] = [3_usize.to_be(), old_layout.size(), new_layout.size()];
            let buffer: [u8; mem::size_of::<[usize; 3]>()] = unsafe { mem::transmute(buffer) };
            Stream::new(trace_stream).write_transaction(&buffer[3..]);
        }
        if Stream::new(trace_stream).is_enabled() {
            trace(trace_stream, old_layout, new_layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHeap {
        pools: [Pool; 10],
    }

    impl Allocator for TestHeap {
        const POOL_COUNT: usize = 10;
        const TRACE_STREAM: Option<u8> = None;

        unsafe fn get_pool_unchecked<I>(&self, index: I) -> &I::Output
        where
            I: SliceIndex<[Pool]>,
        {
            unsafe { self.pools.get_unchecked(index) }
        }
    }

    #[test]
    fn test_binary_search() {
        fn search_layout(heap: &TestHeap, size: usize) -> Option<usize> {
            let pool_idx = binary_search(heap, &Layout::from_size_align(size, 4).unwrap());
            if pool_idx < TestHeap::POOL_COUNT {
                unsafe { Some(heap.get_pool_unchecked(pool_idx).size()) }
            } else {
                None
            }
        }
        fn search_ptr(heap: &TestHeap, ptr: usize) -> Option<usize> {
            let pool_idx = binary_search(heap, unsafe { NonNull::new_unchecked(ptr as *mut u8) });
            if pool_idx < TestHeap::POOL_COUNT {
                unsafe { Some(heap.get_pool_unchecked(pool_idx).size()) }
            } else {
                None
            }
        }
        let heap = TestHeap {
            pools: [
                Pool::new(20, 2, 100),
                Pool::new(220, 5, 100),
                Pool::new(720, 8, 100),
                Pool::new(1520, 12, 100),
                Pool::new(2720, 16, 100),
                Pool::new(4320, 23, 100),
                Pool::new(6620, 38, 100),
                Pool::new(10420, 56, 100),
                Pool::new(16020, 72, 100),
                Pool::new(23220, 91, 100),
            ],
        };
        assert_eq!(search_layout(&heap, 1), Some(2));
        assert_eq!(search_layout(&heap, 2), Some(2));
        assert_eq!(search_layout(&heap, 15), Some(16));
        assert_eq!(search_layout(&heap, 16), Some(16));
        assert_eq!(search_layout(&heap, 17), Some(23));
        assert_eq!(search_layout(&heap, 91), Some(91));
        assert_eq!(search_layout(&heap, 92), None);
        assert_eq!(search_ptr(&heap, 0), Some(2));
        assert_eq!(search_ptr(&heap, 20), Some(2));
        assert_eq!(search_ptr(&heap, 219), Some(2));
        assert_eq!(search_ptr(&heap, 220), Some(5));
        assert_eq!(search_ptr(&heap, 719), Some(5));
        assert_eq!(search_ptr(&heap, 720), Some(8));
        assert_eq!(search_ptr(&heap, 721), Some(8));
        assert_eq!(search_ptr(&heap, 5000), Some(23));
        assert_eq!(search_ptr(&heap, 23220), Some(91));
        assert_eq!(search_ptr(&heap, 32319), Some(91));
        assert_eq!(search_ptr(&heap, 32320), None);
        assert_eq!(search_ptr(&heap, 50000), None);
    }

    #[test]
    fn allocations() {
        unsafe fn allocate_and_set(heap: &TestHeap, layout: Layout, value: u8) {
            unsafe {
                *allocate(heap, layout).unwrap().as_mut_ptr() = value;
            }
        }
        let mut m = [0u8; 3230];
        let o = &mut m as *mut _ as usize;
        let heap = TestHeap {
            pools: [
                Pool::new(o + 0, 2, 10),
                Pool::new(o + 20, 5, 10),
                Pool::new(o + 70, 8, 10),
                Pool::new(o + 150, 12, 10),
                Pool::new(o + 270, 16, 10),
                Pool::new(o + 430, 23, 10),
                Pool::new(o + 660, 38, 10),
                Pool::new(o + 1040, 56, 10),
                Pool::new(o + 1600, 72, 10),
                Pool::new(o + 2320, 91, 10),
            ],
        };
        let layout = Layout::from_size_align(32, 1).unwrap();
        unsafe {
            allocate_and_set(&heap, layout, 111);
            assert_eq!(m[660], 111);
            allocate_and_set(&heap, layout, 222);
            assert_eq!(m[698], 222);
            allocate_and_set(&heap, layout, 123);
            assert_eq!(m[736], 123);
            deallocate(&heap, NonNull::new_unchecked((o + 660) as *mut u8), layout);
            assert_eq!(m[660], 0);
            deallocate(&heap, NonNull::new_unchecked((o + 736) as *mut u8), layout);
            assert_eq!(*(&m[736] as *const _ as *const usize), o + 660);
            allocate_and_set(&heap, layout, 202);
            assert_eq!(m[736], 202);
            deallocate(&heap, NonNull::new_unchecked((o + 698) as *mut u8), layout);
            assert_eq!(*(&m[698] as *const _ as *const usize), o + 660);
            deallocate(&heap, NonNull::new_unchecked((o + 736) as *mut u8), layout);
            assert_eq!(*(&m[736] as *const _ as *const usize), o + 698);
        }
    }
}
