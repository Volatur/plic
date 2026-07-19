use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::ptr::NonNull;

pub struct Boxed<T>(Pin<Box<T>>);
pub struct Array<T>(Pin<Box<[T]>>);

impl<T> Boxed<T> {
    pub fn new(value: T) -> Boxed<T> {
        Boxed(Box::pin(value))
    }

    pub fn as_raw(&self) -> *mut T {
        (&*self.0) as *const _ as *mut _
    }
}

impl<T> Array<T> {
    pub fn new<I: Into<Self>>(value: I) -> Self {
        value.into()
    }

    pub fn empty() -> Self {
        Self(Box::pin([]))
    }

    pub fn as_raw(&self) -> *mut [T] {
        (&*self.0) as *const _ as *mut _
    }
}

impl<T> Deref for Boxed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_raw() }
    }
}

impl<T> DerefMut for Boxed<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.as_raw() }
    }
}

impl<T> Into<NonNull<T>> for &Boxed<T> {
    fn into(self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.as_raw()) }
    }
}

impl<T> Default for Array<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Clone> From<&[T]> for Array<T> {
    fn from(value: &[T]) -> Self {
        Self(Pin::from(value.to_vec().into_boxed_slice()))
    }
}

impl<T> From<Box<[T]>> for Array<T> {
    fn from(value: Box<[T]>) -> Self {
        Self(Pin::from(value))
    }
}

impl<T: Clone> From<&Vec<T>> for Array<T> {
    fn from(value: &Vec<T>) -> Self {
        Self(Pin::from(value.clone().into_boxed_slice()))
    }
}


impl<T> From<Vec<T>> for Array<T> {
    fn from(value: Vec<T>) -> Self {
        Self(Pin::from(value.into_boxed_slice()))
    }
}

impl<T> FromIterator<T> for Array<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl<T> Deref for Array<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.as_raw() }
    }
}

impl<T> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.as_raw() }
    }
}

impl<T> Into<NonNull<[T]>> for &Array<T> {
    fn into(self) -> NonNull<[T]> {
        unsafe { NonNull::new_unchecked(self.as_raw()) }
    }
}