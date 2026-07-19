use crate::napi::control::exit_err;
use crate::napi::control::exit_ok;
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::io::connection::alloc_connection;
use crate::std::io::map_std_io_err_to_vm_throw;
use crate::utils::socket::server::Server;
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;

pub fn alloc_server(mut thread: VMThreadRef, addr: String) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/io/ServerSocket")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    let server = Server::new(addr);
    let server = map_std_io_err_to_vm_throw(thread, server)?;
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Server;
        std::ptr::write(ptr, server);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub unsafe extern "C" fn _server_init(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
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

pub unsafe extern "C" fn _server_uninit(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.0.as_ptr().offset(1);
        let ptr = ptr as *mut Server;
        std::ptr::drop_in_place(ptr);
    }
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _server_accept(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let server = server_native_data(&this);
    let connection = server.accept();
    let connection = map_std_io_err_to_vm_throw(thread, connection);
    let connection = napi_try_or_exit!(connection);
    let connection =
        if let Some((connection, addr)) = connection {
            let connection = alloc_connection(thread, connection, addr.to_string());
            let connection = napi_try_or_exit!(connection);
            connection.into()
        } else {
            ObjectSmartRef::null()
        };
    exit_ok(frame, &connection)
}

fn server_native_data(this: &ObjectSmartRefNN) -> &'static mut Server {
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Server;
        let ptr = &mut *ptr;
        ptr
    }
}