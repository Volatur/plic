use crate::napi::alloc::alloc_bool;
use crate::napi::control::{exit_err, exit_ok};
use crate::napi::ptr::ObjectSmartRef;
use crate::napi_try_or_exit;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;

pub extern "C" fn _callable_eq(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let value = this.0.as_ptr() == other.0;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub extern "C" fn _callable_call(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    exit_ok(frame, &ObjectSmartRef::null())
}
