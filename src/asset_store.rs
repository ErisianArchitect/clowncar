use std::{
    alloc::{self, Layout}, borrow::Borrow, hash::{DefaultHasher, Hash, Hasher}, mem::{ManuallyDrop, transmute}, ptr::NonNull
};

use hashbrown::{
    hash_table::{
        HashTable,
        Entry,
        OccupiedEntry,
    },
};
use parking_lot::Mutex;
use crate::{
    Asset, AssetSlot, AssetSlotInner, AssetType, key::{AssetKey, KeyType}, util::TypeId
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct KeyTypeId(usize);

#[repr(C)]
pub(crate) struct AssetsInner {
    pub(crate) table: HashTable<AssetSlot<()>>,
}

#[repr(C)]
#[derive(Debug)]
pub struct AssetTable {
    pub(crate) ptr: NonNull<AssetsInner>,
    pub(crate) dealloc: fn(NonNull<()>),
}

impl AssetTable {
    const ALLOC_LAYOUT: Layout = Layout::new::<AssetsInner>();

    pub(crate) fn dealloc(ptr: NonNull<()>) {
        let ptr = ptr.cast::<AssetsInner>();
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
        let Some(ptr) = NonNull::new(ptr.cast::<AssetsInner>()) else {
            std::alloc::handle_alloc_error(Self::ALLOC_LAYOUT);
        };
        unsafe {
            ptr.write(AssetsInner {
                table: HashTable::with_capacity(capacity),
            });
        }
        Self {
            ptr,
            dealloc: Self::dealloc,
        }
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
    pub(crate) fn as_inner_ref(&self) -> &AssetsInner {
        unsafe { self.ptr.as_ref() }
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn as_inner_mut(&mut self) -> &mut AssetsInner {
        unsafe { self.ptr.as_mut() }
    }

    pub(crate) fn asset_key_hash(type_id: TypeId, key: &str) -> u64
    {
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn unload<T: AssetType>(&mut self, asset: &Asset<T>) {
        let entry = self.as_inner_mut().table
            .entry(
                asset.as_inner_ref().key.hash,
                |slot| slot.ptr.cast::<()>() == asset.ptr.cast::<()>(),
                |slot| slot.key.hash,
            );
        match entry {
            Entry::Occupied(entry) => _=entry.remove(),
            _=> (),
        }
    }

    pub fn get_or_init<'a, T: AssetType, F: FnOnce() -> T>(&mut self, key: &str, init: F) -> Asset<'a, T>
    where
        F: FnOnce() -> T,
    {
        let type_id = TypeId::of::<T>();
        let key_ref = key;
        let key_hash = Self::asset_key_hash(TypeId::of::<T>(), key_ref);
        let mut table = self.as_inner_ref().table.lock();
        let entry = table
            .entry(
                key_hash,
                move |slot| {
                    type_id.inner() == slot.key.type_id.inner() && slot.key.key.as_ref() == key_ref
                },
                |slot| slot.key.hash,
            );
        match entry {
            Entry::Occupied(entry) => Asset::new(entry.get().ptr.cast()),
            Entry::Vacant(slot) => {
                let owned_key: Box<str> = key.into();
                let asset_slot = AssetSlot::new(owned_key, init());
                let asset = Asset::new(asset_slot.ptr);
                slot.insert(asset_slot.erased());
                asset
            }
        }
    }
}

impl Drop for AssetTable {
    fn drop(&mut self) {
        let dealloc = self.dealloc;
        dealloc(self.ptr.cast());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn asset_store_test() {
        let store = AssetTable::new();
        let asset1 = store.get_or_init("test", || "hello, world 1");
        let asset2 = store.get_or_init("test", || "hello, world 2");
        println!("{}", asset1.as_ref());
        println!("{}", asset2.as_ref());
    }
}