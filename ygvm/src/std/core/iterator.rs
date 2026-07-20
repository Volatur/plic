use crate::napi::alloc::alloc_bool;
use crate::napi::control::{exit_err, exit_ok};
use crate::napi::ptr::ObjectSmartRef;
use crate::napi_try_or_exit;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;

pub unsafe extern "C" fn _iterator_has_next(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = alloc_bool(thread, false);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _iterator_next(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    exit_ok(frame, &ObjectSmartRef::null())
}