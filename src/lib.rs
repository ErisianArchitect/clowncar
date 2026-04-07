//! `clowncar` is an asset storage system with an additional `TypeId` key.

mod asset_slot;
mod asset_store;
mod key;

use std::hash::DefaultHasher;
use hashbrown::HashTable;

pub use asset_slot::*;
pub use asset_store::*;
use key::*;
