use std::{alloc::{self, Layout}, any::TypeId, collections::HashMap, mem::transmute, ptr::NonNull};

use hashbrown::HashTable;
use crate::{AssetType, key::KeyType, AssetSlotInner, AssetSlot, Asset};


#[repr(C)]
pub(crate) struct AssetsInner<K: KeyType> {
    pub(crate) dealloc: fn(NonNull<()>),
    pub(crate) key_type_handle: usize,
    pub(crate) table: HashTable<AssetSlot<K, ()>>,
}

#[repr(C)]
#[derive(Debug)]
pub struct Assets<K: KeyType> {
    ptr: NonNull<AssetsInner<K>>,
}

impl<K: KeyType> Assets<K> {
    const ALLOC_LAYOUT: Layout = Layout::new::<AssetsInner<K>>();
    const KEY_TYPE: &'static TypeId = &TypeId::of::<K>();
    const KEY_TYPE_HANDLE: usize = unsafe { core::mem::transmute(Self::KEY_TYPE) };

    pub(crate) fn dealloc(ptr: NonNull<()>) {
        let ptr = ptr.cast::<AssetsInner<K>>();
        unsafe {
            ptr.drop_in_place();
            std::alloc::dealloc(
                ptr.as_ptr().cast(),
                Self::ALLOC_LAYOUT,
            );
        }
    }

    pub(crate) fn alloc_with_capacity(capacity: usize) -> Self {
        let ptr = unsafe { alloc::alloc(Self::ALLOC_LAYOUT) };
        let Some(ptr) = NonNull::new(ptr.cast::<AssetsInner<K>>()) else {
            std::alloc::handle_alloc_error(Self::ALLOC_LAYOUT);
        };
        unsafe {
            ptr.write(AssetsInner {
                dealloc: Self::dealloc,
                key_type_handle: Self::KEY_TYPE_HANDLE,
                table: HashTable::with_capacity(capacity),
            });
        }
        Self { ptr }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self::alloc_with_capacity(capacity)
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self::alloc_with_capacity(4)
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn as_inner_ref(&self) -> &AssetsInner<K> {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn as_inner_mut(&mut self) -> &mut AssetsInner<K> {
        unsafe { self.ptr.as_mut() }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn erased(self) -> Assets<()> {
        Assets {
            ptr: core::mem::ManuallyDrop::new(self).ptr.cast(),
        }
    }
}

impl Assets<()> {
    #[must_use]
    #[inline(always)]
    pub(crate) unsafe fn cast_unchecked<K: KeyType>(&self) -> &Assets<K> {
        debug_assert!(Assets::<K>::KEY_TYPE_HANDLE == self.as_inner_ref().key_type_handle);
        unsafe { transmute(self) }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) unsafe fn cast_mut_unchecked<K: KeyType>(&mut self) -> &mut Assets<K> {
        debug_assert!(Assets::<K>::KEY_TYPE_HANDLE == self.as_inner_ref().key_type_handle);
        unsafe { transmute(self) }
    }

    pub(crate) fn cast<K: KeyType>(&self) -> Option<&Assets<K>> {
        if Assets::<K>::KEY_TYPE_HANDLE == self.as_inner_ref().key_type_handle {
            return Some(unsafe { self.cast_unchecked() })
        }
        None
    }

    pub(crate) fn cast_mut<K: KeyType>(&mut self) -> Option<&mut Assets<K>> {
        if Assets::<K>::KEY_TYPE_HANDLE == self.as_inner_ref().key_type_handle {
            return Some(unsafe { self.cast_mut_unchecked() });
        }
        None
    }
}

impl<K: KeyType> Drop for Assets<K> {
    fn drop(&mut self) {
        let dealloc = self.as_inner_ref().dealloc;
        dealloc(self.ptr.cast());
    }
}

struct AssetStores {
    table: HashMap<TypeId, Assets<()>>,
}