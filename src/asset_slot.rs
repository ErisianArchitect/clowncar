use std::{alloc::Layout, any::TypeId, hash::Hash, ptr::NonNull};

use lolevel::counting::RefCounter;

use crate::{AssetKey, KeyType};

pub trait AssetType: Sized + Send + Sync + 'static {}
impl<T> AssetType for T
where T: Sized + Send + Sync + 'static {}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct AssetSlotInner<K, T: AssetType> {
    ref_count: RefCounter,
    dealloc: fn(NonNull<()>),
    key: AssetKey<K>,
    asset: T,
}

impl<K: KeyType, T: AssetType> AssetSlotInner<K, T> {
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn dealloc(ptr: NonNull<()>) {
        unsafe {
            std::alloc::dealloc(
                ptr.as_ptr().cast(),
                Self::LAYOUT,
            );
        }
    }
    
    unsafe fn alloc(key: AssetKey<K>, asset: T) -> NonNull<Self> {
        let ptr = unsafe { std::alloc::alloc(Self::LAYOUT).cast::<Self>() };
        let Some(ptr) = NonNull::new(ptr) else {
            std::alloc::handle_alloc_error(Self::LAYOUT);
        };
        let dealloc: fn(NonNull<()>) = |ptr: NonNull<()>| {
            unsafe {
                ptr.cast::<AssetSlotInner<K, T>>().drop_in_place();
                Self::dealloc(ptr);
            }
        };
        unsafe {
            ptr.write(Self {
                ref_count: RefCounter::new(0),
                key,
                dealloc,
                asset,
            });
        }
        ptr
    }
}

#[repr(transparent)]
pub(crate) struct AssetSlot<K: KeyType, T: AssetType = ()> {
    ptr: NonNull<AssetSlotInner<K, T>>,
}
const _: () = lolevel::checks::assert_pointer_niche::<AssetSlot<Box<str>, ()>>();
const _: () = lolevel::checks::assert_pointer_niche::<AssetSlot<Box<str>, Box<str>>>(); // Doesn't make a difference, but whatever.

unsafe impl<K, T> Send for AssetSlot<K, T>
where K: KeyType, T: AssetType {}
unsafe impl<K, T> Sync for AssetSlot<K, T>
where K: KeyType, T: AssetType {}

impl<K: KeyType, T: AssetType> AssetSlot<K, T> {
    const TYPE_ID: TypeId = TypeId::of::<T>();

    pub fn new(key: K, asset: T) -> AssetSlot<K, T> {
        Self {
            ptr: unsafe { AssetSlotInner::<K, T>::alloc(
                AssetKey::new(Self::TYPE_ID, key),
                asset
            ) },
        }
    }

    #[must_use]
    #[inline(always)]
    pub fn as_ref(&self) -> &AssetSlotInner<K, T> {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub fn erased(self) -> AssetSlot<K, ()> {
        AssetSlot {
            ptr: std::mem::ManuallyDrop::new(self).ptr.cast(),
        }
    }
}

impl<K: KeyType, T: AssetType> std::ops::Deref for AssetSlot<K, T> {
    type Target = AssetSlotInner<K, T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<K: KeyType, T: AssetType> Drop for AssetSlot<K, T> {
    fn drop(&mut self) {
        let dealloc = self.dealloc;
        (dealloc)(self.ptr.cast());
    }
}

pub struct Asset<K: KeyType, T: AssetType> {
    ptr: NonNull<AssetSlotInner<K, T>>,
}

unsafe impl<K, T> Send for Asset<K, T>
where K: KeyType, T: AssetType {}
unsafe impl<K, T> Sync for Asset<K, T>
where K: KeyType, T: AssetType {}

impl<K: KeyType, T: AssetType> Asset<K, T> {
    #[must_use]
    #[inline(always)]
    pub(crate) fn new(ptr: NonNull<AssetSlotInner<K, T>>) -> Self {
        let ptr_ref = unsafe { ptr.as_ref() };
        ptr_ref.ref_count.increment();
        Self { ptr }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn as_inner_ref(&self) -> &AssetSlotInner<K, T> {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub fn as_ref(&self) -> &T {
        &self.as_inner_ref().asset
    }
}

impl<K: KeyType, T: AssetType> std::ops::Deref for Asset<K, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<K: KeyType, T: AssetType> Drop for Asset<K, T> {
    fn drop(&mut self) {
        if matches!(self.as_inner_ref().ref_count.decrement(), Ok(true)) {
            // TODO: Unload asset.
        }
    }
}