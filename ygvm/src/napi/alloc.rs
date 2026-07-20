use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi::thread::{NativeFunction, NativeUserdata};
use crate::syntax::parser::Block;
use crate::utils::alloc::Array;
use crate::vm::VMError;
use std::alloc::Layout;
use std::ffi::c_void;
use crate::vm::thread::VMThreadRef;

pub fn alloc_object(thread: VMThreadRef) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::object::alloc_object(thread)
}

pub fn alloc_bool(thread: VMThreadRef, value: bool) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::bool::alloc_bool(thread, value)
}

pub fn alloc_i64(thread: VMThreadRef, value: i64) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::i64::alloc_i64(thread, value)
}

pub fn alloc_f64(thread: VMThreadRef, value: f64) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::f64::alloc_f64(thread, value)
}

pub fn alloc_string(thread: VMThreadRef, value: String) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::string::alloc_string(thread, value)
}

pub fn alloc_array(thread: VMThreadRef, value: Vec<ObjectSmartRef>) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::array::alloc_array(thread, value)
}

pub fn alloc_map(thread: VMThreadRef, value: Vec<(ObjectSmartRef, ObjectSmartRef)>) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::map::alloc_map(thread, value)
}

pub fn alloc_func_ast(thread: VMThreadRef, params: Array<String>, captures: Array<(String, ObjectSmartRef)>, ast: Block) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::function::alloc_func_ast(thread, params, captures, ast)
}

pub fn alloc_func_native(thread: VMThreadRef, params: Array<String>, captures: Array<(String, ObjectSmartRef)>, function: NativeFunction, userdata: NativeUserdata) -> Result<ObjectSmartRefNN, VMError> {
    crate::std::core::function::alloc_func_native(thread, params, captures, function, userdata)
}

pub unsafe fn alloc_native_function_result(value: Result<(), VMError>) -> *mut Result<(), VMError> {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe {
        let result = std::alloc::alloc(Layout::new::<Result<(), VMError>>()) as *mut Result<(), VMError>;
        if result.is_null() { panic!("Out of memory"); }
        std::ptr::write(result, value);
        result
    }
}

pub unsafe fn dealloc_native_function_result(value: *mut Result<(), VMError>) {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe {
        std::ptr::drop_in_place(value);
        std::alloc::dealloc(value as *mut u8, Layout::new::<Result<(), VMError>>());
    }
}

pub unsafe fn alloc_native_function_userdata<T>(value: T) -> *mut c_void {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe {
        let ptr = libc::malloc(size_of::<T>());
        if ptr.is_null() { panic!("Out of memory"); }
        (ptr as *mut T).write(value);
        ptr
    }
}

pub unsafe fn dealloc_native_function_userdata(userdata: *mut c_void) {
    // SAFETY: Гарантия вызывающей стороны.
    unsafe {
        libc::free(userdata);
    }
}