use crate::napi::alloc::alloc_native_function_userdata;
use crate::napi::control::{exit_err, exit_ok};
use crate::napi::ptr::ObjectSmartRef;
use crate::utils::alloc::Array;
use crate::vm::module::Function;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::{VMError, VMRef};

pub fn execute(mut vm: VMRef, func: fn (VMThreadRef, VMStackFrameRef) -> Result<ObjectSmartRef, VMError>) -> Result<ObjectSmartRef, VMError> {
    let func = Function::Native {
        name: "<execute>".to_owned(),
        params: Array::empty(),
        function: execute_wrapper,
        // SAFETY: Гарантия стандарта.
        userdata: unsafe { alloc_native_function_userdata(func) }
    };
    vm.call_func(&ObjectSmartRef::null(), &func, &[])
}

unsafe extern "C" fn execute_wrapper(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    // SAFETY: Гарантия стандарта.
    if let Function::Native { userdata, .. } = unsafe { &*frame.function } {
        let func = *userdata as *mut fn(VMThreadRef, VMStackFrameRef) -> Result<ObjectSmartRef, VMError>;
        // SAFETY: Гарантия стандарта.
        let result =
            match (unsafe { *func })(thread, frame) {
                Ok(value) => exit_ok(frame, &value),
                Err(err) => exit_err(err)
            };
        // SAFETY: Гарантия стандарта.
        unsafe { std::ptr::drop_in_place(func) };
        result
    } else {
        unreachable!()
    }
}