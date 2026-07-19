use crate::napi::control::{exit_err, exit_ok, exit_throw};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::core::exception::alloc_exception;
use crate::utils::map::Map;
use crate::std::core::{call_eq_or_eq, call_to_string_or_text_null};
use crate::std::json::json_element::{alloc_json_element, json_element_to_native, json_element_to_object};
use crate::vm::heap::{VMHeap, VMHeapGC};
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use serde_json::Value;
pub use std::ops::Deref;
use crate::napi::alloc::{alloc_bool, alloc_string};
use crate::napi::convert::bool_to_native;

pub fn alloc_map(mut thread: VMThreadRef, value: Vec<(ObjectSmartRef, ObjectSmartRef)>) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/core/Map")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    let value = value.iter().map(|(k, v)| (k.as_raw(), v.as_raw())).collect();
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Map;
        let value = Map::from_vec(thread.into(), value)?;
        std::ptr::write(ptr, value);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub fn map_to_native(mut thread: VMThreadRef, value: ObjectSmartRef) -> Result<Vec<(ObjectSmartRef, ObjectSmartRef)>, VMError> {
    let object = value.deref()?;
    if object.class.owner.path == "std/core" && object.class.name == "Map" {
        let value = map_native_data(&object);
        let value = value.iter().map(|(k, v)| (ObjectSmartRef::new(k), ObjectSmartRef::new(v))).collect();
        Ok(value)
    } else {
        let value = thread.call_obj(&object, "__to_map__", &[])?;
        map_to_native(thread, value)
    }
}

pub unsafe extern "C" fn _map_init(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = thread.call_class("std/core/Object", "__init__", &[this]);
    let this = napi_try_or_exit!(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    this.flags.mark_uninit();
    this.flags.mark_marker();
    let this = this.into();
    exit_ok(frame, &this)
}

pub unsafe extern "C" fn _map_uninit(_thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.0.as_ptr().offset(1);
        let ptr = ptr as *mut Map;
        std::ptr::drop_in_place(ptr);
    }
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _map_mark(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let value = map_native_data(&this);
    for (key, value) in value {
        if let Some(key) = key.try_deref() {
            let key = ObjectSmartRefNN::new(key);
            if let Err(err) = VMHeapGC::gc_mark(thread.into(), &key) {
                return exit_err(err);
            }
        }
        if let Some(value) = value.try_deref() {
            let value = ObjectSmartRefNN::new(value);
            if let Err(err) = VMHeapGC::gc_mark(thread.into(), &value) {
                return exit_err(err);
            }
        }
    }
    exit_ok(frame, &ObjectSmartRef::null())
}

pub unsafe extern "C" fn _map_eq(mut thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let value = thread.call_class("std/core/Object", "__eq__", &[this.clone(), other.clone()]);
    let value = napi_try_or_exit!(value);
    let value = bool_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value =
        if !value {
            false
        } else if this.is_null() {
            true
        } else {
            // SAFETY: Проверка is_null.
            let this = unsafe { this.deref().unwrap_unchecked() };
            let this = map_native_data(&this);
            // SAFETY: Проверка is_null + вызов __eq__.
            let other = unsafe { other.deref().unwrap_unchecked() };
            let other = map_native_data(&other);
            if this.len() == other.len() {
                let mut result = true;
                for (key, value) in this {
                    if let Some(other) = napi_try_or_exit!(other.get(thread, key)) {
                        let value = ObjectSmartRef::new(value.clone());
                        let other = ObjectSmartRef::new(other);
                        match call_eq_or_eq(thread, value, other) {
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
        };
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _map_set(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let map = map_native_data(&this);
    let index = frame.locals.get_global("index");
    let value = frame.locals.get_global("value");
    napi_try_or_exit!(map.insert(thread, index, value));
    let value = ObjectSmartRef::new(value);
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _map_get(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let map = map_native_data(&this);
    let index = frame.locals.get_global("index");
    let value = map.get(thread, index);
    let value = napi_try_or_exit!(value);
    let value = value.map_or_else(ObjectSmartRef::null, |x| ObjectSmartRef::new(x));
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _map_to_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let map = map_native_data(&this);
    let mut stringified = Vec::new();
    for (key, value) in map {
        let key = ObjectSmartRef::new(key);
        let key = call_to_string_or_text_null(thread, key);
        let key = napi_try_or_exit!(key);
        let value = ObjectSmartRef::new(value.clone());
        let value = call_to_string_or_text_null(thread, value);
        let value = napi_try_or_exit!(value);
        stringified.push("(".to_string() + key.as_str() + ", " + value.as_str() + ")");
    }
    let text = "[".to_owned() + stringified.join(", ").as_str() + "]";
    let text = alloc_string(thread, text);
    let text = napi_try_or_exit!(text);
    let text = Into::<ObjectSmartRef>::into(text);
    exit_ok(frame, &text)
}

pub unsafe extern "C" fn _map_to_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = this.deref();
    let this = napi_try_or_exit!(this);
    let value = map_native_data(&this);
    let mut map = Vec::new();
    for (key, value) in value {
        let key = ObjectSmartRef::new(key);
        let key = json_element_to_native(thread, key);
        let key = napi_try_or_exit!(key);
        let value = ObjectSmartRef::new(value.clone());
        let value = json_element_to_native(thread, value);
        let value = napi_try_or_exit!(value);
        map.push(Value::Array(vec![key, value]));
    }
    let value = Value::Array(map);
    let value = alloc_json_element(thread, "std/core/Map".to_owned(), value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _map_from_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = json_element_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value =
        match value {
            Value::Array(value) => value,
            _ => {
                let exception = alloc_exception(thread, "Map from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception)
            }
        };
    let mut map = Vec::new();
    for element in value {
        if let Value::Array(element) = element && element.len() == 2 {
            // SAFETY: Проверка len == 2 выше.
            let key = unsafe { element.get_unchecked(0) };
            let key = json_element_to_object(thread, key.clone());
            let key = napi_try_or_exit!(key);
            // SAFETY: Проверка len == 2 выше.
            let value = unsafe { element.get_unchecked(1) };
            let value = json_element_to_object(thread, value.clone());
            let value = napi_try_or_exit!(value);
            map.push((key, value))
        } else {
            let exception = alloc_exception(thread, "Map from json parsing error".to_owned());
            let exception = napi_try_or_exit!(exception);
            return exit_throw(exception)
        }
    }
    let value = alloc_map(thread, map);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

fn map_native_data(this: &ObjectSmartRefNN) -> &'static mut Map {
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = this.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut Map;
        let ptr = &mut *ptr;
        ptr
    }
}