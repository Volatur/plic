use crate::napi::alloc::alloc_string;
use crate::napi::control::{exit_err, exit_ok};
use crate::napi::convert::string_to_native;
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;

pub fn alloc_exception(mut thread: VMThreadRef, message: String) -> Result<ObjectSmartRef, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/core/Exception")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let mut object = object.deref()?;
    let message = alloc_string(thread, message)?;
    let message = Into::<ObjectSmartRef>::into(message);
    object.fields.insert("message".to_string(), message.as_raw());
    Ok(object.into())
}

pub unsafe extern "C" fn _exception_to_string(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let message = thread.obj_get(&this, "message");
    let message = napi_try_or_exit!(message);
    let message = string_to_native(thread, message);
    let message = napi_try_or_exit!(message);
    let mut text = message;
    text.insert_str(0, "Exception: ");
    let text = alloc_string(thread, text);
    let text = napi_try_or_exit!(text);
    let text = Into::<ObjectSmartRef>::into(text);
    exit_ok(frame, &text)
}