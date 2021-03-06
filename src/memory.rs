use std::mem;
use std::slice;

// this frees memory we released earlier
#[no_mangle]
pub extern "C" fn free_rust(buf: Buffer) {
    unsafe {
        let _ = buf.consume();
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Buffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

impl Buffer {
    // read provides a reference to the included data to be parsed or copied elsewhere
    // data is only guaranteed to live as long as the Buffer
    // (or the scope of the extern "C" call it came from)
    pub fn read(&self) -> Option<&[u8]> {
        if self.len == 0 {
            None
        } else {
            unsafe { Some(slice::from_raw_parts(self.ptr, self.len)) }
        }
    }

    /// consume must only be used on memory previously released by from_vec
    /// when the Vec is out of scope, it will deallocate the memory previously referenced by Buffer
    ///
    /// # Safety
    ///
    /// if not empty, `ptr` must be a valid memory reference, which was previously
    /// created by `from_vec`. You may not consume a slice twice.
    /// Otherwise you risk double free panics
    pub unsafe fn consume(self) -> Vec<u8> {
        Vec::from_raw_parts(self.ptr, self.len, self.cap)
    }

    /// Creates a new zero length Buffer with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Buffer::from_vec(Vec::<u8>::with_capacity(capacity))
    }

    // this releases our memory to the caller
    pub fn from_vec(v: Vec<u8>) -> Self {
        let mut v = mem::ManuallyDrop::new(v);
        Buffer {
            ptr: v.as_mut_ptr(),
            len: v.len(),
            cap: v.capacity(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_works() {
        let buffer1 = Buffer::from_vec(vec![0xAA]);
        assert_eq!(buffer1.read(), Some(&[0xAAu8] as &[u8]));

        let buffer2 = Buffer::from_vec(vec![0xAA, 0xBB, 0xCC]);
        assert_eq!(buffer2.read(), Some(&[0xAAu8, 0xBBu8, 0xCCu8] as &[u8]));

        let buffer3 = Buffer::from_vec(Vec::new());
        assert_eq!(buffer3.read(), None);

        let buffer4 = Buffer::with_capacity(7);
        assert_eq!(buffer4.read(), None);

        // Cleanup
        unsafe { buffer1.consume() };
        unsafe { buffer2.consume() };
        unsafe { buffer3.consume() };
        unsafe { buffer4.consume() };
    }

    #[test]
    fn with_capacity_works() {
        let buffer = Buffer::with_capacity(7);
        assert_eq!(buffer.ptr.is_null(), false);
        assert_eq!(buffer.len, 0);
        assert_eq!(buffer.cap, 7);

        // Cleanup
        unsafe { buffer.consume() };
    }

    #[test]
    fn from_vec_and_consume_work() {
        let mut original: Vec<u8> = vec![0x00, 0xaa, 0x76];
        original.reserve_exact(2);
        let original_ptr = original.as_ptr();

        let buffer = Buffer::from_vec(original);
        assert_eq!(buffer.ptr.is_null(), false);
        assert_eq!(buffer.len, 3);
        assert_eq!(buffer.cap, 5);

        let restored = unsafe { buffer.consume() };
        assert_eq!(restored.as_ptr(), original_ptr);
        assert_eq!(restored.len(), 3);
        assert_eq!(restored.capacity(), 5);
        assert_eq!(&restored, &[0x00, 0xaa, 0x76]);
    }

    #[test]
    fn from_vec_and_consume_work_for_zero_len() {
        let mut original: Vec<u8> = vec![];
        original.reserve_exact(2);
        let original_ptr = original.as_ptr();

        let buffer = Buffer::from_vec(original);
        assert_eq!(buffer.ptr.is_null(), false);
        assert_eq!(buffer.len, 0);
        assert_eq!(buffer.cap, 2);

        let restored = unsafe { buffer.consume() };
        assert_eq!(restored.as_ptr(), original_ptr);
        assert_eq!(restored.len(), 0);
        assert_eq!(restored.capacity(), 2);
    }

    #[test]
    fn from_vec_and_consume_work_for_zero_capacity() {
        let original: Vec<u8> = vec![];
        let original_ptr = original.as_ptr();

        let buffer = Buffer::from_vec(original);
        // Skip ptr test here. Since Vec does not allocate memory when capacity is 0, this could be anything
        assert_eq!(buffer.len, 0);
        assert_eq!(buffer.cap, 0);

        let restored = unsafe { buffer.consume() };
        assert_eq!(restored.as_ptr(), original_ptr);
        assert_eq!(restored.len(), 0);
        assert_eq!(restored.capacity(), 0);
    }
}
