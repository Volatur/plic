use crate::vm::heap::{Object, ObjectRef, ObjectRefNN, ObjectWeakRef, VMHeap};
use crate::vm::{VMError, VMRef};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
#[repr(transparent)]
pub struct ObjectSmartRef(pub Option<ObjectRefNN>);
#[repr(C)]
pub struct ObjectSmartRefNN(pub ObjectRefNN);
#[repr(transparent)]
pub struct ObjectSmartWeakRef(pub Option<(VMRef, ObjectWeakRef)>);

impl ObjectSmartRef {
    pub fn new(native: ObjectRef) -> Self {
        if let Some(native) = native.try_deref() {
            // SAFETY: Проверка выше.
            unsafe { Self::new_unchecked(native) }
        } else {
            Self::null()
        }
    }
    
    pub unsafe fn new_unchecked(native: ObjectRefNN) -> Self {
        native.flags.inc_rc();
        Self(Some(native))
    }

    pub fn null() -> Self {
        Self(None)
    }

    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }

    pub fn deref(self) -> Result<ObjectSmartRefNN, VMError> {
        if let Some(object) = self.try_deref() {
            Ok(object)
        } else {
            Err(VMError::NullPointer)
        }
    }

    pub fn try_deref(self) -> Option<ObjectSmartRefNN> {
        if let Some(native) = &self.0 {
            Some(ObjectSmartRefNN::new(native.clone()))
        } else {
            None
        }
    }

    pub fn as_weak(&self, vm: VMRef) -> ObjectSmartWeakRef {
        if let Some(native) = &self.0 {
            ObjectSmartWeakRef::new(vm, native.clone())
        } else {
            ObjectSmartWeakRef::null()
        }
    }

    pub fn as_raw(&self) -> ObjectRef {
        if let Some(native) = &self.0 {
            ObjectRef(native.0.as_ptr())
        } else {
            ObjectRef::null()
        }
    }
}

impl From<ObjectRef> for ObjectSmartRef {
    fn from(native: ObjectRef) -> Self {
        Self::new(native)
    }
}

impl From<&ObjectRef> for ObjectSmartRef {
    fn from(native: &ObjectRef) -> Self {
        Self::new(native.clone())
    }
}

impl Clone for ObjectSmartRef {
    fn clone(&self) -> Self {
        if let Some(native) = &self.0 {
            // SAFETY: Проверка выше
            unsafe { Self::new_unchecked(native.clone()) }
        } else {
            Self::null()
        }
    }
}

impl Drop for ObjectSmartRef {
    fn drop(&mut self) {
        if let Some(native) = &self.0 {
            native.flags.dec_rc();
        }
    }
}

impl ObjectSmartRefNN {
    pub fn new(native: ObjectRefNN) -> Self {
        native.flags.inc_rc();
        Self(native)
    }
    
    pub fn deref(native: ObjectRef) -> Result<Self, VMError> {
        if let Some(value) = Self::try_deref(native) {
            Ok(value)
        } else {
            Err(VMError::NullPointer)
        }
    }
    
    pub fn try_deref(native: ObjectRef) -> Option<Self> {
        if let Some(native) = native.try_deref() {
            Some(Self::new(native))
        } else {
            None
        }
    }

    pub fn to_weak(&self, vm :VMRef) -> ObjectSmartWeakRef {
        ObjectSmartWeakRef::new(vm, self.0.clone())
    }

    pub fn as_raw(&self) -> ObjectRefNN {
        self.0.clone()
    }
}

impl Into<ObjectSmartRef> for ObjectSmartRefNN {
    fn into(self) -> ObjectSmartRef {
        ObjectSmartRef::new(self.0.into())
    }
}

impl Clone for ObjectSmartRefNN {
    fn clone(&self) -> Self {
        Self::new(self.0.clone())
    }
}

impl Drop for ObjectSmartRefNN {
    fn drop(&mut self) {
        self.flags.dec_rc();
    }
}

impl Deref for ObjectSmartRefNN {
    type Target = Object;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Гарантия структуры.
        unsafe { self.as_raw().0.as_ref() }
    }
}

impl DerefMut for ObjectSmartRefNN {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Гарантия структуры.
        unsafe { self.as_raw().0.as_mut() }
    }
}

impl ObjectSmartWeakRef {
    pub fn new(vm: VMRef, native: ObjectRefNN) -> Self {
        Self(Some((vm, VMHeap::new_weak(vm, native))))
    }

    pub fn null() -> Self {
        Self(None)
    }

    pub fn to_smart(&self) -> ObjectSmartRef {
        if let Some((_, native)) = self.0.as_ref() &&
            // SAFETY: Проверка выше.
            let Some(object) = unsafe { native.0.as_ref() }.object.try_deref()
        {
            // SAFETY: Проверка выше.
            unsafe { ObjectSmartRef::new_unchecked(object) }
        } else {
            ObjectSmartRef::null()
        }
    }

    pub fn as_raw(&self) -> ObjectRef {
        if let Some((_, native)) = &self.0 {
            // SAFETY: Проверка на Some.
            unsafe { native.0.as_ref() }.object
        } else {
            ObjectRef::null()
        }
    }
}

impl Clone for ObjectSmartWeakRef {
    fn clone(&self) -> Self {
        if let Some((vm, native)) = self.0.as_ref() &&
            // SAFETY: Проверка на Some.
            let Some(object) = unsafe { native.0.as_ref() }.object.try_deref()
        {
            Self::new(vm.clone(), object)
        } else {
            Self::null()
        }
    }
}

impl Drop for ObjectSmartWeakRef {
    fn drop(&mut self) {
        if let Some((vm, native)) = &self.0 {
            VMHeap::drop_weak(vm.clone(), native)
        }
    }
}
