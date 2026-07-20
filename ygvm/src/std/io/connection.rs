use crate::napi::alloc::alloc_string;
use crate::napi::control::exit_err;
use crate::napi::control::exit_ok;
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::io::map_std_io_err_to_vm_throw;
use crate::std::json::{deserialize_from_json, serialize_to_json};
use crate::utils::socket::server::Connection;
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;

pub fn alloc_connection(mut thread: VMThreadRef, connection: Connection, addr: String) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/io/Connection")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Connection;
        std::ptr::write(ptr, connection);
        let ptr = ptr.offset(1);
        let ptr = ptr as *mut u8;
        let ptr = ptr.offset(4);
        let ptr = ptr as *mut String;
        std::ptr::write(ptr, addr);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub unsafe extern "C" fn _connection_init(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = thread.call_class("std/core/Object", "__init__", &[this]);
    let this = napi_try_or_exit!(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    this.flags.mark_uninit();
    let this = this.into();
    exit_ok(frame, &this)
}

pub unsafe extern "C" fn _connection_uninit(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.0.as_ptr().offset(1);
        let ptr = ptr as *mut Connection;
        std::ptr::drop_in_place(ptr);
        let ptr = ptr.offset(1);
        let ptr = ptr as *mut u8;
        let ptr = ptr.offset(4);
        let ptr = ptr as *mut String;
        std::ptr::drop_in_place(ptr);
    }
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _connection_addr(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let (_, addr) = connection_native_data(&this);
    let value = addr.to_owned();
    let value = alloc_string(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _connection_send(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = serialize_to_json(thread, value);
    let value = napi_try_or_exit!(value);
    let (connection, _) = connection_native_data(&this);
    napi_try_or_exit!(map_std_io_err_to_vm_throw(thread, connection.send(value)));
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _connection_recv(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let (connection, _) = connection_native_data(&this);
    let value = connection.recv();
    let value = map_std_io_err_to_vm_throw(thread, value);
    let value = napi_try_or_exit!(value);
    let value =
        if let Some(value) = value {
            let value = deserialize_from_json(thread, value);
            let value = napi_try_or_exit!(value);
            value
        } else {
            ObjectSmartRef::null()
        };
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _connection_close(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let (connection, _) = connection_native_data(&this);
    napi_try_or_exit!(map_std_io_err_to_vm_throw(thread, connection.close()));
    exit_ok(frame, &ObjectSmartRef::null())
}

fn connection_native_data(this: &ObjectSmartRefNN) -> (&'static mut Connection, &'static mut String) {
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Connection;
        let connection = &mut *ptr;
        let ptr = ptr.offset(1);
        let ptr = ptr as *mut u8;
        let ptr = ptr.offset(4);
        let ptr = ptr as *mut String;
        let addr = &mut *ptr;
        (connection, addr)
    }
}