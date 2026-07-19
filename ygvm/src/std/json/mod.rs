pub mod json_element;

use crate::napi::alloc::alloc_string;
use crate::napi::control::{exit_err, exit_ok};
use crate::napi::convert::string_to_native;
use crate::napi::module::{ClassDef, FunctionBodyDef, FunctionDef, ModuleDef};
use crate::napi::ptr::ObjectSmartRef;
use crate::napi_try_or_exit;
use crate::std::core::exception::alloc_exception;
use crate::std::json::json_element::{_json_element_init, _json_element_to_json, _json_element_uninit, json_element_to_native, json_element_to_object};
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::{VMError, VMRef};
use serde_json::Value;

pub fn load(vm: VMRef) -> Result<(), VMError> {
    VMModuleManager::load_napi_module(vm, &ModuleDef {
        path: "std/json".to_owned(),
        uses: vec![],
        functions: vec![
            FunctionDef {
                name: "to_string".to_owned(),
                params: vec!["value".to_owned()],
                body: FunctionBodyDef::Native(_to_string),
            },
            FunctionDef {
                name: "from_string".to_owned(),
                params: vec!["value".to_owned()],
                body: FunctionBodyDef::Native(_from_string),
            },
        ],
        classes: vec![
            ClassDef {
                name: "JsonElement".to_owned(),
                extends: vec!["std/core/Object".to_owned()],
                methods: vec![
                    FunctionDef {
                        name: "__init__".to_owned(),
                        params: vec![],
                        body: FunctionBodyDef::Native(_json_element_init)
                    },
                    FunctionDef {
                        name: "__uninit__".to_owned(),
                        params: vec![],
                        body: FunctionBodyDef::Native(_json_element_uninit)
                    },
                    FunctionDef {
                        name: "__to_json__".to_owned(),
                        params: vec![],
                        body: FunctionBodyDef::Native(_json_element_to_json)
                    },
                ],
                allocation: size_of::<Value>()
            }
        ],
        objects: vec![],
    })
}

pub fn unload(_vm: VMRef) -> Result<(), VMError> {
    Ok(())
}

unsafe extern "C" fn _to_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = serialize_to_json(thread, value);
    let value = napi_try_or_exit!(value);
    let value = alloc_string(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}

unsafe extern "C" fn _from_string(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let value = frame.locals.get_global("value");
    let value = ObjectSmartRef::new(value);
    let value = string_to_native(thread, value);
    let value = napi_try_or_exit!(value);
    let value = deserialize_from_json(thread, value);
    let value = napi_try_or_exit!(value);
    exit_ok(frame, &value)
}

pub fn serialize_to_json(thread: VMThreadRef, value: ObjectSmartRef) -> Result<String, VMError> {
    let value = json_element_to_native(thread, value)?;
    let value = serde_json::to_string(&value);
    let value =
        match value {
            Ok(value) => value,
            Err(err) => {
                let exception = alloc_exception(thread, err.to_string())?;
                return Err(VMError::__Throwing__(exception))
            }
        };
    Ok(value)
}

pub fn deserialize_from_json(thread: VMThreadRef, value: String) -> Result<ObjectSmartRef, VMError> {
    let value = serde_json::from_str::<Value>(&value);
    let value =
        match value {
            Ok(value) => value,
            Err(err) => {
                let exception = alloc_exception(thread, err.to_string())?;
                return Err(VMError::__Throwing__(exception))
            }
        };
    json_element_to_object(thread, value)
}