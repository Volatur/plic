use crate::napi::control::{exit_err, exit_ok, exit_throw};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::core::{call_eq_or_eq, call_hash_or_nil};
use crate::std::json::json_element::{alloc_json_element, json_element_to_native, json_element_to_object};
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use serde_json::Value;
use std::hash::{DefaultHasher, Hash, Hasher};
use crate::napi::alloc::{alloc_bool, alloc_i64, alloc_string};
use crate::napi::convert::bool_to_native;
use crate::std::core::exception::alloc_exception;

pub fn alloc_object(mut thread: VMThreadRef) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/core/Object")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub unsafe extern "C" fn _object_init(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    exit_ok(frame, &this)
}

pub unsafe extern "C" fn _object_hash(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let value =
        if let Some(this) = this.try_deref() {
            let hasher = &mut DefaultHasher::new();
            this.class.owner.path.hash(hasher);
            for (_, field) in this.fields.iter() {
                let field = ObjectSmartRef::new(field.clone());
                let value = call_hash_or_nil(thread, field);
                let value = napi_try_or_exit!(value);
                hasher.write_i64(value);
            }
            let value = hasher.finish();
            value
        } else {
            0
        };
    let value = alloc_i64(thread, value as i64);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _object_eq(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let other = frame.locals.get_global("other");
    let value =
        if this.0 == other.0 {
            true
        } else if let Some(this) = this.try_deref() {
            if let Some(other) = other.try_deref()
                && this.class == other.class
                && this.fields.len() == other.fields.len() {
                let mut result = true;
                for (name, field) in this.fields.iter() {
                    if let Some(other) = other.fields.get(name) {
                        let field = ObjectSmartRef::new(field.clone());
                        let other = ObjectSmartRef::new(other.clone());
                        match call_eq_or_eq(thread, field, other) {
                            Ok(value) => if !value {
                                result = false;
                                break;
                            }
                            Err(err) => return exit_err(err)
                        }
                    } else {
                        result = false;
                        break;
                    }
                }
                result
            } else {
                false
            }
        } else {
            other.0.is_null()
        };
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _object_neq(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRefNN::deref(this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let value = thread.call_obj(&this, "__eq__", &[other]);
    let value = napi_try_or_exit!(value);
    let value = bool_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value = !value;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _object_to_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let text =
        if let Some(this) = this.try_deref() {
            "[".to_owned() + this.class.name.as_str() + "] " + (this.0.as_ptr() as usize).to_string().as_str()
        } else {
            "null".to_string()
        };
    let text = alloc_string(thread, text);
    let text = napi_try_or_exit!(text);
    let text = Into::<ObjectSmartRef>::into(text);
    exit_ok(frame, &text)
}

pub unsafe extern "C" fn _object_to_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let mut map = Vec::new();
    for (key, value) in this.fields.iter() {
        let key = Value::String(key.to_owned());
        let value = ObjectSmartRef::new(value.clone());
        let value = json_element_to_native(thread, value);
        let value = napi_try_or_exit!(value);
        map.push(Value::Array(vec![key, value]));
    }
    let value = Value::Array(map);
    let class = this.class.owner.path.to_owned() + "/" + this.class.name.as_str();
    let value = Value::Array(vec![Value::String(class.to_owned()), value]);
    let value = alloc_json_element(thread, class, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _object_from_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value.clone());
    let value = json_element_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let (class, fields) =
        if let Value::Array(value) = &value && value.len() == 2 {
            // SAFETY: Проверка len == 2 выше.
            unsafe { (value.get_unchecked(0), value.get_unchecked(1)) }
        } else {
            let exception = alloc_exception(thread, "Object from json parsing error".to_owned());
            let exception = napi_try_or_exit!(exception);
            return exit_throw(exception)
        };
    if *fields == Value::Null { return exit_ok(frame, &ObjectSmartRef::null()) }
    let class =
        match class {
            Value::String(class) => class,
            _ => {
                let exception = alloc_exception(thread, "Object from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception);
            }
        };
    let class = VMModuleManager::find_class(thread.vm, class);
    let class = napi_try_or_exit!(class);
    let object = VMHeap::alloc(thread.vm, class);
    let mut object = napi_try_or_exit!(object);
    if let Value::Array(fields) = fields {
        for field in fields {
            if let Value::Array(field) = field && field.len() == 2 {
                // SAFETY: Проверка len == 2 выше.
                let key = unsafe { field.get_unchecked(0) };
                let key =
                    match key {
                        Value::String(key) => key.to_owned(),
                        _ => {
                            let exception = alloc_exception(thread, "Object from json parsing error".to_owned());
                            let exception = napi_try_or_exit!(exception);
                            return exit_throw(exception);
                        }
                    };
                // SAFETY: Проверка len == 2 выше.
                let value = unsafe { field.get_unchecked(1) };
                let value = json_element_to_object(thread, value.clone());
                let value = napi_try_or_exit!(value);
                object.fields.insert(key, value.as_raw());
            } else {
                let exception = alloc_exception(thread, "Object from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception);
            }
        }
        let object = object.into();
        exit_ok(frame, &object)
    } else {
        let exception = alloc_exception(thread, "Object from json parsing error".to_owned());
        let exception = napi_try_or_exit!(exception);
        exit_throw(exception)
    }
}