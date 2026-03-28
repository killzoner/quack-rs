// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Ergonomic wrapper around `duckdb_function_info` for scalar function callbacks.

use std::ffi::CString;
use std::os::raw::c_void;

use libduckdb_sys::{
    duckdb_function_info, duckdb_scalar_function_get_extra_info,
    duckdb_scalar_function_set_error,
};

/// Ergonomic wrapper around the `duckdb_function_info` handle provided to a
/// scalar function callback.
///
/// Provides access to extra info and error reporting.
///
/// # Example
///
/// ```rust,no_run
/// use quack_rs::scalar::ScalarFunctionInfo;
/// use libduckdb_sys::{duckdb_function_info, duckdb_data_chunk, duckdb_vector};
///
/// unsafe extern "C" fn my_func(
///     info: duckdb_function_info,
///     _input: duckdb_data_chunk,
///     _output: duckdb_vector,
/// ) {
///     let info = unsafe { ScalarFunctionInfo::new(info) };
///     let _extra = unsafe { info.get_extra_info() };
///     // ... use extra info ...
/// }
/// ```
pub struct ScalarFunctionInfo {
    info: duckdb_function_info,
}

impl ScalarFunctionInfo {
    /// Wraps a raw `duckdb_function_info` provided by `DuckDB` inside a scalar
    /// function callback.
    ///
    /// # Safety
    ///
    /// `info` must be a valid `duckdb_function_info` passed by `DuckDB` to a
    /// scalar function callback.
    #[inline]
    #[must_use]
    pub const unsafe fn new(info: duckdb_function_info) -> Self {
        Self { info }
    }

    /// Retrieves the extra-info pointer previously set via
    /// [`ScalarFunctionBuilder::extra_info`][crate::scalar::ScalarFunctionBuilder::extra_info].
    ///
    /// Returns a raw `*mut c_void`. Cast it back to your concrete type.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid as long as the scalar function is
    /// registered and `DuckDB` has not yet called the destructor.
    #[must_use]
    pub unsafe fn get_extra_info(&self) -> *mut c_void {
        // SAFETY: self.info is valid per constructor contract.
        unsafe { duckdb_scalar_function_get_extra_info(self.info) }
    }

    /// Reports an error from the scalar function callback, causing `DuckDB`
    /// to abort the current query.
    ///
    /// # Panics
    ///
    /// Panics if `message` contains an interior null byte.
    pub fn set_error(&self, message: &str) {
        let c_msg = CString::new(message).expect("error message must not contain null bytes");
        // SAFETY: self.info is valid per constructor contract.
        unsafe {
            duckdb_scalar_function_set_error(self.info, c_msg.as_ptr());
        }
    }

    /// Returns the raw `duckdb_function_info` handle.
    #[must_use]
    #[inline]
    pub const fn as_raw(&self) -> duckdb_function_info {
        self.info
    }
}
