use core::{
    any::{Any, TypeId as CoreTypeId},
    marker::PhantomData,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(usize);

struct TypeIdMaker<T>(PhantomData<*const T>);

impl<T: Any> TypeIdMaker<T> {
    const TYPE_ID_VALUE: CoreTypeId = CoreTypeId::of::<T>();
    const TYPE_ID_REF: &'static CoreTypeId = &Self::TYPE_ID_VALUE;
}

impl TypeId {
    #[must_use]
    #[inline(always)]
    pub fn of<T: Any>() -> TypeId {
        TypeId(unsafe { core::mem::transmute(TypeIdMaker::<T>::TYPE_ID_REF) })
    }

    #[must_use]
    #[inline(always)]
    pub const fn inner(self) -> usize {
        self.0
    }
}