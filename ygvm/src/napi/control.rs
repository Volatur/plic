use crate::napi;
use crate::napi::ptr::ObjectSmartRef;
use crate::vm::thread::VMStackFrameRef;
use crate::vm::VMError;

pub fn exit_ok(mut frame: VMStackFrameRef, result: &ObjectSmartRef) -> *mut Result<(), VMError> {
    frame.locals.set_global("__return__", result.as_raw());
    // SAFETY: Гарантия вызывающей стороны.
    unsafe { napi::alloc::alloc_native_function_result(Ok(())) }
}

pub fn exit_err(error: VMError) -> *mut Result<(), VMError> {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe { napi::alloc::alloc_native_function_result(Err(error)) }
}

pub fn exit_throw(exception: ObjectSmartRef) -> *mut Result<(), VMError> {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe { napi::alloc::alloc_native_function_result(Err(VMError::__Throwing__(exception))) }
}

#[macro_export]
macro_rules! napi_try_or_exit {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => return exit_err(err),
        }
    };
}