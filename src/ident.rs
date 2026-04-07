use core::{
    ptr::NonNull,
};
use std::alloc::Layout;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct IdentHeap {
    len: u16,
    flags: u16,
    extra: u32,
    ptr: NonNull<u8>,
}

#[derive(Debug, Clone, Copy)]
struct IdentStack {
    len: u16,
    bytes: [u8; 14],
}

union IdentUnion {
    len: u16,
    heap: IdentHeap,
    stack: IdentStack,
}

#[repr(C)]
pub struct Ident {
    raw: IdentUnion,
}

impl Ident {
    pub fn new(value: &str) -> Self {
        assert!(value.len() <= u16::MAX as usize, "Length cannot be greater than u16::MAX.");
        if value.len() <= 14 {
            let mut bytes = [0u8; 14];
            bytes[0..value.len()].copy_from_slice(value.as_bytes());
            Self {
                raw: IdentUnion { stack: IdentStack { len: value.len() as u16, bytes } }
            }
        } else {
            let layout = Layout::array::<u8>(value.len()).unwrap();
            let heap_mem = unsafe { std::alloc::alloc(layout) };
            let Some(heap_mem) = NonNull::new(heap_mem) else {
                std::alloc::handle_alloc_error(layout);
            };
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), heap_mem.as_ptr(), value.len());
            }
            Self {
                raw: IdentUnion {
                    heap: IdentHeap {
                        len: value.len() as u16,
                        padding: [0u8; 2],
                        extra: 0,
                        ptr: heap_mem,
                    }
                }
            }
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            if self.raw.len <= 14 {
                core::mem::transmute(
                    core::slice::from_raw_parts(self.raw.stack.bytes.as_ptr(), self.raw.len as usize)
                )
            } else {
                core::mem::transmute(
                    core::slice::from_raw_parts(self.raw.heap.ptr.as_ptr(), self.raw.len as usize)
                )
            }
        }
    }

    #[must_use]
    #[inline(always)]
    pub fn is_stack(&self) -> bool {
        unsafe {
            self.raw.len as usize <= self.raw.stack.bytes.len()
        }
    }
}

impl Drop for Ident {
    fn drop(&mut self) {
        unsafe {
            if self.raw.stack.len as usize > self.raw.stack.bytes.len() {
                let ptr = self.raw.heap.ptr.as_ptr();
                let len = self.raw.heap.len as usize;
                let layout = Layout::array::<u8>(len).unwrap();
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn ident_test() {
        let hello = Ident::new("hello");
        println!("{}", hello.as_str());
        assert!(hello.is_stack());
        let bigger = Ident::new("This is a much longer string, and will end up on the heap.");

    }
}