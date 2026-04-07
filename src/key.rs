use core::hash::Hash;
use std::borrow::Borrow;
use std::hash::{DefaultHasher, Hasher};

use crate::util::TypeId;

pub trait KeyType: Eq + Hash + Send + Sync + 'static {}
impl<K: Eq + Hash + Send + Sync + 'static> KeyType for K {}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AssetKey {
    pub(crate) hash: u64,
    pub(crate) type_id: TypeId,
    pub(crate) key: Box<str>,
}

impl AssetKey {
    #[must_use]
    #[inline]
    pub fn hash<Q>(type_id: TypeId, key: &Q) -> u64
    where
        Box<str>: Borrow<Q>,
        Q: Hash
    {
        let mut hasher = DefaultHasher::new();
        type_id.inner().hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[must_use]
    #[inline]
    pub fn eq<Q>(&self, type_id: TypeId, key: &Q) -> bool
    where
        Box<str>: Borrow<Q>,
        Q: Eq,
    {
        self.type_id == type_id && self.key.borrow() == key
    }

    #[must_use]
    #[inline]
    pub fn new(type_id: TypeId, key: Box<str>) -> Self {
        Self {
            hash: Self::hash(type_id, &key),
            type_id,
            key,
        }
    }
}