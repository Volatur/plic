use crate::napi::control::{exit_err, exit_ok, exit_throw};
use crate::napi::ptr::{ObjectSmartRef, ObjectSmartRefNN};
use crate::napi_try_or_exit;
use crate::std::core::exception::alloc_exception;
use crate::std::json::json_element::{alloc_json_element, json_element_to_native};
use crate::vm::heap::VMHeap;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::VMError;
use serde_json::{Number, Value};
use crate::napi::alloc::{alloc_bool, alloc_i64, alloc_string};

pub fn alloc_f64(mut thread: VMThreadRef, value: f64) -> Result<ObjectSmartRefNN, VMError> {
    let class = VMModuleManager::find_class(thread.vm, "std/core/F64")?;
    let object = VMHeap::alloc(thread.vm, class)?;
    // SAFETY: Гарантия стандарта.
    unsafe {
        let ptr = object.as_raw().0.as_ptr().offset(1);
        let ptr = ptr as *mut f64;
        std::ptr::write(ptr, value);
    }
    let init = class.find_method("__init__")?;
    let object = object.into();
    let object = thread.call_func(&object, init, &[])?;
    let object = object.deref()?;
    Ok(object)
}

pub fn f64_to_native(mut thread: VMThreadRef, value: ObjectSmartRef) -> Result<f64, VMError> {
    if let Some(object) = value.try_deref() {
        if object.class.owner.path == "std/core" && object.class.name == "F64" {
            // SAFETY: Гарантия стандарта.
            unsafe {
                let ptr = object.as_raw().0.as_ptr().offset(1);
                let ptr = ptr as *mut f64;
                let ptr = *ptr;
                Ok(ptr)
            }
        } else {
            let value = thread.call_obj(&object, "__to_f64__", &[])?;
            f64_to_native(thread, value)
        }
    } else {
        Ok(0f64)
    }
}

pub unsafe extern "C" fn _f64_eq(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let other = frame.locals.get_global("other");
    let value =
        if this.0 == other.0 {
            true
        } else {
            let this = ObjectSmartRef::new(this);
            let this = f64_to_native(thread, this);
            let this = napi_try_or_exit!(this);
            let other = ObjectSmartRef::new(other);
            let other = f64_to_native(thread, other);
            let other = napi_try_or_exit!(other);
            this == other
        };
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_add(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this + other;
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_sub(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this - other;
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_mul(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this * other;
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_div(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this / other;
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_neg(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let value = -this;
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}


pub unsafe extern "C" fn _f64_lt(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this < other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_le(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this <= other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_gt(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this > other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_ge(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let this = f64_to_native(thread, this);
    let this = napi_try_or_exit!(this);
    let other = frame.locals.get_global("other");
    let other = ObjectSmartRef::new(other);
    let other = f64_to_native(thread, other);
    let other = napi_try_or_exit!(other);
    let value = this >= other;
    let value = alloc_bool(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_to_f64(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value =
        if let Some(this) = this.try_deref() {
            this
        } else {
            let value = alloc_f64(thread, 0f64);
            let value = napi_try_or_exit!(value);
            value
        };
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_to_i64(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value = f64_to_native(thread, this);
    let value = napi_try_or_exit!(value);
    let value = value as i64;
    let value = alloc_i64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_to_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value = f64_to_native(thread, this.into());
    let value = napi_try_or_exit!(value);
    let text = value.to_string();
    let text = alloc_string(thread, text);
    let text = napi_try_or_exit!(text);
    let text = Into::<ObjectSmartRef>::into(text);
    exit_ok(frame, &text)
}

pub unsafe extern "C" fn _f64_to_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let this = frame.locals.get_global("this");
    let this = ObjectSmartRef::new(this);
    let value = f64_to_native(thread, this);
    let value = napi_try_or_exit!(value);
    let value = Number::from_f64(value);
    let value =
        match value {
            Some(value) => value,
            None => {
                let exception = alloc_exception(thread, "Float formatting error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception);
            },
        };
    let value = Value::Number(value);
    let value = alloc_json_element(thread, "std/core/F64".to_owned(), value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

pub unsafe extern "C" fn _f64_from_json(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = json_element_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value =
        match value {
            Value::Number(value) => value,
            _ => {
                let exception = alloc_exception(thread, "Float from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception)
            }
        };
    let value = value.as_f64();
    let value =
        match value {
            Some(value) => value,
            None => {
                let exception = alloc_exception(thread, "Float from json parsing error".to_owned());
                let exception = napi_try_or_exit!(exception);
                return exit_throw(exception)
            }
        };
    let value = alloc_f64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}