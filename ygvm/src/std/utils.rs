use crate::napi::control::{exit_err, exit_ok};
use crate::napi::module::{FunctionBodyDef, FunctionDef, ModuleDef};
use crate::napi::ptr::ObjectSmartRef;
use crate::vm::module::VMModuleManager;
use crate::vm::thread::{VMStackFrameRef, VMThreadRef};
use crate::vm::{VMError, VMRef};
use crate::napi_try_or_exit;
use rand::random_range;
use crate::napi::alloc::alloc_i64;
use crate::napi::convert::i64_to_native;

pub fn load(vm: VMRef) -> Result<(), VMError> {
    VMModuleManager::load_napi_module(vm, &ModuleDef {
        path: "std/utils".to_owned(),
        uses: vec![],
        functions: vec![
            FunctionDef {
                name: "random".to_owned(),
                params: vec!["from".to_owned(), "to".to_owned()],
                body: FunctionBodyDef::Native(_random),
            },
        ],
        classes: vec![],
        objects: vec![],
    })
}

pub fn unload(_vm: VMRef) -> Result<(), VMError> {
    Ok(())
}

unsafe extern "C" fn _random(thread: VMThreadRef, frame: VMStackFrameRef) -> *mut Result<(), VMError> {
    let from = frame.locals.get_global("from");
    let from = ObjectSmartRef::new(from);
    let from = i64_to_native(thread, from);
    let from = napi_try_or_exit!(from);
    let to = frame.locals.get_global("to");
    let to = ObjectSmartRef::new(to);
    let to = i64_to_native(thread, to);
    let to = napi_try_or_exit!(to);
    let value = random_range(from..to);
    let value = alloc_i64(thread, value);
    let value = napi_try_or_exit!(value);
    let value = value.into();
    exit_ok(frame, &value)
}