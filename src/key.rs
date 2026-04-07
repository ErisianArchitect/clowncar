use core::hash::Hash;
use core::any::TypeId;
use std::borrow::Borrow;
use std::hash::{DefaultHasher, Hasher};

pub trait KeyType: Eq + Hash + Send + Sync + 'static {}
impl<K: Eq + Hash + Send + Sync + 'static> KeyType for K {}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct AssetKey<K> {
    pub(crate) hash: u64,
    pub(crate) type_id: TypeId,
    pub(crate) key: K,
}

impl<K: KeyType> AssetKey<K> {
    #[must_use]
    #[inline]
    pub fn hash<Q>(type_id: &TypeId, key: &Q) -> u64
    where
        K: Borrow<Q>,
        Q: Hash
    {
        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[must_use]
    #[inline]
    pub fn eq<Q>(&self, type_id: &TypeId, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        &self.type_id == type_id && self.key.borrow() == key
    }

    #[must_use]
    #[inline]
    pub fn new(type_id: TypeId, key: K) -> Self {
        Self {
            hash: Self::hash(&type_id, &key),
            type_id,
            key,
        }
    }
}