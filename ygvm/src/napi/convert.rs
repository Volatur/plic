use crate::napi::ptr::ObjectSmartRef;
use crate::vm::thread::VMThreadRef;
use crate::vm::VMError;

pub fn bool_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<bool, VMError> {
    crate::std::core::bool::bool_to_native(thread, value)
}

pub fn i64_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<i64, VMError> {
    crate::std::core::i64::i64_to_native(thread, value)
}

pub fn f64_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<f64, VMError> {
    crate::std::core::f64::f64_to_native(thread, value)
}

pub fn string_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<String, VMError> {
    crate::std::core::string::string_to_native(thread, value)
}

pub fn array_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<Vec<ObjectSmartRef>, VMError> {
    crate::std::core::array::array_to_native(thread, value)
}

pub fn map_to_native(thread: VMThreadRef, value: ObjectSmartRef) -> Result<Vec<(ObjectSmartRef, ObjectSmartRef)>, VMError> {
    crate::std::core::map::map_to_native(thread, value)
}