use crate::vm::VMError;
use std::ptr::NonNull;

#[repr(transparent)]
pub struct NativeResult(pub Option<NonNull<VMError>>);

#[unsafe(export_name = "ygvm_napi_error_check")]
pub extern "C" fn c_error_check(result: &NativeResult) -> bool {
    result.is_err()
}

#[unsafe(export_name = "ygvm_napi_error_drop")]
pub extern "C" fn c_error_drop(result: &mut NativeResult) {
    if result.0.is_none() { return }
    drop(std::mem::replace(result, NativeResult::ok()));
}


impl NativeResult {
    pub fn ok() -> Self {
        Self(None)
    }

    pub fn err(error: VMError) -> Self {
        Self(NonNull::new(Box::into_raw(Box::new(error))))
    }

    pub fn is_ok(&self) -> bool {
        self.0.is_none()
    }

    pub fn is_err(&self) -> bool {
        self.0.is_some()
    }
}

impl Drop for NativeResult {
    fn drop(&mut self) {
        if let Some(err) = self.0.take() {
            // SAFETY: Гарантия структуры.
            drop(unsafe { Box::from_raw(err.as_ptr()) });
        }
    }
}