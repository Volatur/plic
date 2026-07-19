use crate::napi::control::{exit_err, exit_ok, exit_throw};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::core::exception::alloc_exception;
use crate::std::json::json_element::{alloc_json_element, json_element_to_native};
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use serde_json::Value;
use crate::napi::alloc::alloc_string;

pub fn alloc_bool(thread: VMThreadRef, value: bool) -> Result<ObjectSmartRefNN, VMError> {
    let object = if value { "std/core/True" } else { "std/core/False" };
    let object = VMModuleManager::find_object(thread.vm, object)?;
    Ok(object)
}

pub fn bool_to_native(mut thread: VMThreadRef, value: ObjectSmartRef) -> Result<bool, VMError> {
    if let Some(object) = value.try_deref() {
        if object.class.owner.path == "std/core" {
            match object.class.name.as_str() {
                "True" => return Ok(true),
                "False" => return Ok(false),
                _ => { }
            };
        }
        let value = thread.call_obj(&object, "__to_bool__", &[])?;
        bool_to_native(thread, value)
    } else {
        Ok(false)
    }
}

pub unsafe extern "C" fn _bool_eq(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let other = frame.locals.get_global("other");
    let value =
        if this.0 == other.0 {
            true
        } else if this.0.is_null() {
            other.0.is_null()
        } else if other.0.is_null() {
            this.0.is_null()
        } else {
            let this = ObjectSmartRef::new(this);
            let this = bool_to_native(thread, this);
            let this = napi_try_or_exit!(this);
            let other = ObjectSmartRef::new(other);
            let other = bool_to_native(thread, other);
            let other = napi_try_or_exit!(other);
            this == other
        };
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}


pub unsafe extern "C" fn _bool_and(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = bool_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = bool_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this && other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _bool_or(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = bool_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = bool_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this || other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _bool_not(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value = bool_to_native(thread, this);
    let value = napi_try_or_exit!(value);
    let value = !value;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _bool_to_bool(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value =
        if let Some(this) = this.try_deref() {
            this
        } else {
            let text = alloc_bool(thread, false);
            let text = napi_try_or_exit!(text);
            text
        };
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _bool_to_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = bool_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let text = if this { "true" } else { "false" };
    let text = text.to_owned();
    let text = alloc_string(thread, text);
    let text = napi_try_or_exit!(text);
    let text = Into::<ObjectSmartRef>::into(text);
    exit_ok(frame, &text)
}

pub unsafe extern "C" fn _bool_to_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value = bool_to_native(thread, this);
    let value = napi_try_or_exit!(value);
    let value = Value::Bool(value);
    let value = alloc_json_element(thread, "std/core/Bool".to_owned(), value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _bool_from_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = json_element_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value =
        match value {
            Value::Bool(value) => value,
            _ => {
                let exception = alloc_exception(thread, "Bool from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception)
            }
        };
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}
