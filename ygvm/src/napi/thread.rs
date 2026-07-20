use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use std::ffi::c_void;

pub type NativeFunction = unsafe extern "C" fn (VMThreadRef, VMStackFrameRef) -> *mut Result<(), VMError>;
pub type NativeUserdata = *mut c_void;