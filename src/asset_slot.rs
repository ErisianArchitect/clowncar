use std::{alloc::Layout, hash::Hash, marker::PhantomData, ptr::NonNull};

use lolevel::counting::RefCounter;

use crate::{
    AssetKey,
    KeyType,
    util::{
        TypeId,
    }
};

pub trait AssetType: Sized + Send + Sync + 'static {}
impl<T> AssetType for T
where T: Sized + Send + Sync + 'static {}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct AssetSlotInner<T: AssetType> {
    pub(crate) ref_count: RefCounter,
    pub(crate) key: AssetKey,
    pub(crate) asset: T,
}

impl<T: AssetType> AssetSlotInner<T> {
    const LAYOUT: Layout = Layout::new::<Self>();

    unsafe fn dealloc(ptr: NonNull<()>) {
        unsafe {
            std::alloc::dealloc(
                ptr.as_ptr().cast(),
                Self::LAYOUT,
            );
        }
    }
    
    unsafe fn alloc(key: AssetKey, asset: T) -> NonNull<Self> {
        let ptr = unsafe { std::alloc::alloc(Self::LAYOUT).cast::<Self>() };
        let Some(ptr) = NonNull::new(ptr) else {
            std::alloc::handle_alloc_error(Self::LAYOUT);
        };
        unsafe {
            ptr.write(Self {
                ref_count: RefCounter::new(1),
                key,
                asset,
            });
        }
        ptr
    }
}

#[repr(transparent)]
pub(crate) struct AssetSlot<T: AssetType = ()> {
    pub(crate) ptr: NonNull<AssetSlotInner<T>>,
}
const _: () = lolevel::checks::assert_pointer_niche::<AssetSlot<()>>();
const _: () = lolevel::checks::assert_pointer_niche::<AssetSlot<Box<str>>>(); // Doesn't make a difference, but whatever.

unsafe impl<T> Send for AssetSlot<T>
where T: AssetType {}
unsafe impl<T> Sync for AssetSlot<T>
where T: AssetType {}

impl<T: AssetType> AssetSlot<T> {

    pub fn new(key: Box<str>, asset: T) -> AssetSlot<T> {
        Self {
            ptr: unsafe { AssetSlotInner::<T>::alloc(
                AssetKey::new(TypeId::of::<T>(), key),
                asset
            ) },
        }
    }

    #[must_use]
    #[inline(always)]
    pub fn as_ref(&self) -> &AssetSlotInner<T> {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub fn erased(self) -> AssetSlot<()> {
        AssetSlot {
            ptr: std::mem::ManuallyDrop::new(self).ptr.cast(),
        }
    }
}

impl<T: AssetType> std::ops::Deref for AssetSlot<T> {
    type Target = AssetSlotInner<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub struct Asset<'a, T: AssetType> {
    pub(crate) ptr: NonNull<AssetSlotInner<T>>,
    _phantom: PhantomData<*const &'a T>,
}

impl<'a, T: AssetType> Asset<'a, T> {
    #[must_use]
    #[inline(always)]
    pub(crate) fn new(ptr: NonNull<AssetSlotInner<T>>) -> Self {
        let ptr_ref = unsafe { ptr.as_ref() };
        ptr_ref.ref_count.increment();
        Self { ptr, _phantom: PhantomData }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn as_inner_ref(&self) -> &AssetSlotInner<T> {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub fn as_ref(&self) -> &T {
        &self.as_inner_ref().asset
    }
}

impl<'a, T: AssetType> std::ops::Deref for Asset<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, T: AssetType> Drop for Asset<'a, T> {
    fn drop(&mut self) {
        if matches!(self.as_inner_ref().ref_count.decrement(), Ok(0)) {
            // TODO: Asset store unload when that is implemented.
            unsafe { AssetSlotInner::<T>::dealloc(self.ptr.cast()); }
        }
    }
}