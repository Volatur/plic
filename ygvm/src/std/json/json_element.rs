use crate::napi::control::{exit_err, exit_ok};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::core::exception::alloc_exception;
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use serde_json::Value;

pub fn alloc_json_element(mut thread: VMThreadRef, element_class: String, element_value: Value) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/json/JsonElement")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    let mut value = serde_json::Map::new();
    value.insert("class".to_owned(), Value::String(element_class));
    value.insert("value".to_owned(), element_value);
    let value = Value::Object(value);
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Value;
        std::ptr::write(ptr, value);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub fn alloc_json_element_raw(mut thread: VMThreadRef, value: Value) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/json/JsonElement")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Value;
        std::ptr::write(ptr, value);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub fn json_element_to_native(mut thread: VMThreadRef, value: ObjectSmartRef) -> Result<Value, VMError> {
    if let Some(object) = value.try_deref() {
        if object.class.owner.path == "std/json" && object.class.name == "JsonElement" {
            // SAFETY: Гарантия стандарта.
            unsafe {
                let ptr = object.as_raw().0.as_ptr().offset(1);
                let ptr = ptr as *mut Value;
                let ptr = &*ptr;
                Ok(ptr.to_owned())
            }
        } else {
            let value = thread.call_obj(&object, "__to_json__", &[])?;
            json_element_to_native(thread, value)
        }
    } else {
        let mut value = serde_json::Map::new();
        value.insert("class".to_owned(), Value::String("std/core/Object".to_owned()));
        value.insert("value".to_owned(), Value::Array(vec![Value::String("std/core/Object".to_owned()), Value::Null]));
        Ok(Value::Object(value))
    }
}

pub fn json_element_to_object(mut thread: VMThreadRef, value: Value) -> Result<ObjectSmartRef, VMError> {
    let value =
        match value {
            Value::Object(value) => value,
            _ => {
                let exception = alloc_exception(thread, "Json parsing error".to_owned())?;
                return Err(VMError::__Throwing__(exception))
            }
        };
    let class =
        match value.get("class") {
            Some(Value::String(value)) => value,
            _ => {
                let exception = alloc_exception(thread, "Json parsing error".to_owned())?;
                return Err(VMError::__Throwing__(exception))
            }
        };
    let value =
        match value.get("value") {
            Some(value) => value,
            _ => {
                let exception = alloc_exception(thread, "Json parsing error".to_owned())?;
                return Err(VMError::__Throwing__(exception))
            }
        };
    let value = alloc_json_element_raw(thread, value.clone())?;
    let value = value.into();
    let result = thread.call_class(class, "__from_json__", &[value])?;
    Ok(result)
}

pub unsafe extern "C" fn _json_element_init(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = thread.call_class("std/core/Object", "__init__", &[this]);
    let this = napi_try_or_exit!(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    this.flags.mark_uninit();
    let this = Into::<ObjectSmartRef>::into(this);
    exit_ok(frame, &this)
}

pub unsafe extern "C" fn _json_element_uninit(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.0.as_ptr().offset(1);
        let ptr = ptr as *mut Value;
        std::ptr::drop_in_place(ptr);
    }
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _json_element_to_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let value =
        if let Some(value) = this.try_deref() {
            ObjectSmartRef::new(value.into())
        } else {
            let value = alloc_json_element(thread, "std/core/Object".to_string(), Value::Null);
            let value = napi_try_or_exit!(value);
            let value = value.into();
            value
        };
    exit_ok(frame, &value)
}