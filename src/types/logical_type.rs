// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! RAII wrapper for `duckdb_logical_type`.
//!
//! # Pitfall L7: `LogicalType` memory leak
//!
//! Every `duckdb_create_logical_type` call allocates memory that must be freed
//! with `duckdb_destroy_logical_type`. Forgetting to call the destructor leaks
//! memory. [`LogicalType`] implements `Drop` to prevent this.

use crate::types::TypeId;
use libduckdb_sys::{
    duckdb_create_list_type, duckdb_create_logical_type, duckdb_create_map_type,
    duckdb_create_struct_type, duckdb_destroy_logical_type, duckdb_logical_type,
};
use std::fmt;

/// Error returned by fallible [`LogicalType`] constructors when the underlying
/// `DuckDB` C API returns a null pointer.
#[derive(Debug, Clone)]
pub struct LogicalTypeError {
    api_func: &'static str,
}

impl fmt::Display for LogicalTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} returned null", self.api_func)
    }
}

impl std::error::Error for LogicalTypeError {}

/// An RAII wrapper around a `duckdb_logical_type` handle.
///
/// Created from a [`TypeId`], this type ensures `duckdb_destroy_logical_type`
/// is called when it is dropped. This prevents the memory leak described in
/// [Pitfall L7](https://github.com/tomtom215/quack-rs/blob/main/LESSONS.md).
///
/// # Example
///
/// ```rust,no_run
/// use quack_rs::types::{LogicalType, TypeId};
///
/// // Requires DuckDB runtime to be initialized (i.e., loaded as an extension).
/// let lt = LogicalType::new(TypeId::BigInt);
/// // `lt` is automatically destroyed when it goes out of scope
/// ```
pub struct LogicalType {
    inner: duckdb_logical_type,
}

impl LogicalType {
    /// Creates a `LogicalType` from an existing raw `duckdb_logical_type` handle.
    ///
    /// The returned `LogicalType` takes ownership of the handle and will call
    /// `duckdb_destroy_logical_type` when dropped.
    ///
    /// # Safety
    ///
    /// - `ptr` must be a valid, non-null `duckdb_logical_type` handle returned by
    ///   a `duckdb_create_*` function (e.g. `duckdb_create_logical_type`,
    ///   `duckdb_create_list_type`, `duckdb_create_struct_type`, etc.).
    /// - The caller must not call `duckdb_destroy_logical_type` on the handle
    ///   after passing it to this function.
    /// - The handle must not be used after this call except through the returned
    ///   `LogicalType`.
    #[must_use]
    pub unsafe fn from_raw(ptr: duckdb_logical_type) -> Self {
        assert!(!ptr.is_null(), "LogicalType::from_raw called with null pointer");
        Self { inner: ptr }
    }

    /// Creates a new `LogicalType` for the given `TypeId`.
    ///
    /// Calls `duckdb_create_logical_type` internally.
    ///
    /// # Panics
    ///
    /// Panics if `duckdb_create_logical_type` returns a null pointer (should never
    /// happen for supported types, but is checked defensively).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use quack_rs::types::{LogicalType, TypeId};
    ///
    /// // Requires DuckDB runtime (called from within a loaded extension).
    /// let lt = LogicalType::new(TypeId::Timestamp);
    /// assert!(!lt.as_raw().is_null());
    /// ```
    #[must_use]
    pub fn new(type_id: TypeId) -> Self {
        // SAFETY: `duckdb_create_logical_type` is safe to call with any valid DUCKDB_TYPE.
        // It returns a heap-allocated handle that must be freed with duckdb_destroy_logical_type.
        let inner = unsafe { duckdb_create_logical_type(type_id.to_duckdb_type()) };
        assert!(!inner.is_null(), "duckdb_create_logical_type returned null");
        Self { inner }
    }

    /// Creates a `LIST<element_type>` logical type.
    ///
    /// Lists are variable-length sequences of the given element type.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use quack_rs::types::{LogicalType, TypeId};
    ///
    /// // Requires DuckDB runtime.
    /// let list_of_int = LogicalType::list(TypeId::Integer);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `duckdb_create_list_type` returns null (should never happen).
    #[must_use]
    pub fn list(element_type: TypeId) -> Self {
        let element_lt = Self::new(element_type);
        // SAFETY: element_lt.as_raw() is a valid logical type.
        let inner = unsafe { duckdb_create_list_type(element_lt.as_raw()) };
        assert!(!inner.is_null(), "duckdb_create_list_type returned null");
        Self { inner }
    }

    /// Creates a `MAP<key_type, value_type>` logical type.
    ///
    /// `DuckDB` maps are stored as `LIST<STRUCT{key: K, value: V}>`.
    ///
    /// # Panics
    ///
    /// Panics if `duckdb_create_map_type` returns null.
    #[must_use]
    pub fn map(key_type: TypeId, value_type: TypeId) -> Self {
        let key_lt = Self::new(key_type);
        let val_lt = Self::new(value_type);
        // SAFETY: both logical types are valid.
        let inner = unsafe { duckdb_create_map_type(key_lt.as_raw(), val_lt.as_raw()) };
        assert!(!inner.is_null(), "duckdb_create_map_type returned null");
        Self { inner }
    }

    /// Creates a `STRUCT` logical type from a slice of `(name, type)` field definitions.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use quack_rs::types::{LogicalType, TypeId};
    ///
    /// // Requires DuckDB runtime.
    /// let point = LogicalType::struct_type(&[
    ///     ("x", TypeId::Double),
    ///     ("y", TypeId::Double),
    /// ]);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if any field name contains an interior null byte, or if
    /// `duckdb_create_struct_type` returns null.
    #[must_use]
    pub fn struct_type(fields: &[(&str, TypeId)]) -> Self {
        use std::ffi::CString;

        // Build arrays of logical type handles and C name pointers.
        // The logical types must outlive the duckdb_create_struct_type call.
        let field_types: Vec<Self> = fields.iter().map(|&(_, t)| Self::new(t)).collect();
        let c_names: Vec<CString> = fields
            .iter()
            .map(|&(n, _)| CString::new(n).expect("field name must not contain null bytes"))
            .collect();

        let mut type_ptrs: Vec<duckdb_logical_type> =
            field_types.iter().map(Self::as_raw).collect();
        let mut name_ptrs: Vec<*const std::os::raw::c_char> =
            c_names.iter().map(|s| s.as_ptr()).collect();

        // SAFETY: type_ptrs and name_ptrs are valid for the duration of this call.
        let inner = unsafe {
            duckdb_create_struct_type(
                type_ptrs.as_mut_ptr(),
                name_ptrs.as_mut_ptr(),
                fields.len() as libduckdb_sys::idx_t,
            )
        };
        assert!(!inner.is_null(), "duckdb_create_struct_type returned null");
        Self { inner }
    }

    /// Creates a `LIST<element>` logical type from an existing [`LogicalType`].
    ///
    /// Use this when the element type is itself a complex type (e.g.
    /// `LIST(STRUCT(...))`) that cannot be expressed as a simple [`TypeId`].
    ///
    /// # Panics
    ///
    /// Panics if `duckdb_create_list_type` returns null.
    #[must_use]
    pub fn list_from_logical(element: &LogicalType) -> Self {
        let inner = unsafe { duckdb_create_list_type(element.as_raw()) };
        assert!(!inner.is_null(), "duckdb_create_list_type returned null");
        Self { inner }
    }

    /// Creates a `MAP<key, value>` logical type from existing [`LogicalType`]s.
    ///
    /// Use this when the key or value types are complex types that cannot be
    /// expressed as simple [`TypeId`] values.
    ///
    /// # Panics
    ///
    /// Panics if `duckdb_create_map_type` returns null.
    #[must_use]
    pub fn map_from_logical(key: &LogicalType, value: &LogicalType) -> Self {
        let inner = unsafe { duckdb_create_map_type(key.as_raw(), value.as_raw()) };
        assert!(!inner.is_null(), "duckdb_create_map_type returned null");
        Self { inner }
    }

    /// Creates a `STRUCT` logical type from a slice of `(name, LogicalType)` fields.
    ///
    /// Use this when struct members have complex types (e.g.
    /// `STRUCT(headers MAP(VARCHAR, VARCHAR), body VARCHAR)`) that cannot be
    /// expressed as simple [`TypeId`] values.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use quack_rs::types::{LogicalType, TypeId};
    ///
    /// // STRUCT(status INTEGER, headers MAP(VARCHAR, VARCHAR), body VARCHAR)
    /// let response = LogicalType::struct_type_from_logical(&[
    ///     ("status", LogicalType::new(TypeId::Integer)),
    ///     ("headers", LogicalType::map(TypeId::Varchar, TypeId::Varchar)),
    ///     ("body", LogicalType::new(TypeId::Varchar)),
    /// ]);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if any field name contains an interior null byte, or if
    /// `duckdb_create_struct_type` returns null.
    #[must_use]
    pub fn struct_type_from_logical(fields: &[(&str, LogicalType)]) -> Self {
        use std::ffi::CString;

        let c_names: Vec<CString> = fields
            .iter()
            .map(|&(n, _)| CString::new(n).expect("field name must not contain null bytes"))
            .collect();

        let mut type_ptrs: Vec<duckdb_logical_type> =
            fields.iter().map(|(_, lt)| lt.as_raw()).collect();
        let mut name_ptrs: Vec<*const std::os::raw::c_char> =
            c_names.iter().map(|s| s.as_ptr()).collect();

        let inner = unsafe {
            duckdb_create_struct_type(
                type_ptrs.as_mut_ptr(),
                name_ptrs.as_mut_ptr(),
                fields.len() as libduckdb_sys::idx_t,
            )
        };
        assert!(!inner.is_null(), "duckdb_create_struct_type returned null");
        Self { inner }
    }

    /// Fallible version of [`LogicalType::new`]. Returns an error instead of
    /// panicking if the `DuckDB` C API returns a null pointer.
    pub fn try_new(type_id: TypeId) -> Result<Self, LogicalTypeError> {
        let inner = unsafe { duckdb_create_logical_type(type_id.to_duckdb_type()) };
        if inner.is_null() {
            return Err(LogicalTypeError {
                api_func: "duckdb_create_logical_type",
            });
        }
        Ok(Self { inner })
    }

    /// Fallible version of [`LogicalType::list`]. Returns an error instead of
    /// panicking if the `DuckDB` C API returns a null pointer.
    pub fn try_list(element_type: TypeId) -> Result<Self, LogicalTypeError> {
        let element_lt = Self::try_new(element_type)?;
        let inner = unsafe { duckdb_create_list_type(element_lt.as_raw()) };
        if inner.is_null() {
            return Err(LogicalTypeError {
                api_func: "duckdb_create_list_type",
            });
        }
        Ok(Self { inner })
    }

    /// Fallible version of [`LogicalType::map`]. Returns an error instead of
    /// panicking if the `DuckDB` C API returns a null pointer.
    pub fn try_map(key_type: TypeId, value_type: TypeId) -> Result<Self, LogicalTypeError> {
        let key_lt = Self::try_new(key_type)?;
        let val_lt = Self::try_new(value_type)?;
        let inner = unsafe { duckdb_create_map_type(key_lt.as_raw(), val_lt.as_raw()) };
        if inner.is_null() {
            return Err(LogicalTypeError {
                api_func: "duckdb_create_map_type",
            });
        }
        Ok(Self { inner })
    }

    /// Fallible version of [`LogicalType::struct_type`]. Returns an error
    /// instead of panicking if a field name contains an interior null byte or
    /// if the `DuckDB` C API returns a null pointer.
    pub fn try_struct_type(fields: &[(&str, TypeId)]) -> Result<Self, LogicalTypeError> {
        use std::ffi::CString;

        let field_types: Vec<Self> = fields
            .iter()
            .map(|&(_, t)| Self::try_new(t))
            .collect::<Result<_, _>>()?;
        let c_names: Vec<CString> = fields
            .iter()
            .map(|&(n, _)| {
                CString::new(n).map_err(|_| LogicalTypeError {
                    api_func: "CString::new (field name contains null byte)",
                })
            })
            .collect::<Result<_, _>>()?;

        let mut type_ptrs: Vec<duckdb_logical_type> =
            field_types.iter().map(Self::as_raw).collect();
        let mut name_ptrs: Vec<*const std::os::raw::c_char> =
            c_names.iter().map(|s| s.as_ptr()).collect();

        let inner = unsafe {
            duckdb_create_struct_type(
                type_ptrs.as_mut_ptr(),
                name_ptrs.as_mut_ptr(),
                fields.len() as libduckdb_sys::idx_t,
            )
        };
        if inner.is_null() {
            return Err(LogicalTypeError {
                api_func: "duckdb_create_struct_type",
            });
        }
        Ok(Self { inner })
    }

    /// Returns the underlying raw `duckdb_logical_type` handle.
    ///
    /// # Safety note
    ///
    /// Do not call `duckdb_destroy_logical_type` on the returned handle; that is
    /// handled by this type's `Drop` implementation.
    #[must_use]
    #[inline]
    pub const fn as_raw(&self) -> duckdb_logical_type {
        self.inner
    }

    /// Consumes this `LogicalType` and returns the raw handle without destroying it.
    ///
    /// The caller is responsible for calling `duckdb_destroy_logical_type` on the
    /// returned handle.
    #[must_use]
    pub const fn into_raw(self) -> duckdb_logical_type {
        let raw = self.inner;
        // Prevent Drop from running by wrapping in ManuallyDrop
        std::mem::forget(self);
        raw
    }
}

impl Drop for LogicalType {
    fn drop(&mut self) {
        // SAFETY: `self.inner` was created by `duckdb_create_logical_type` and has not
        // been transferred elsewhere. It is safe to destroy exactly once here.
        unsafe {
            duckdb_destroy_logical_type(&raw mut self.inner);
        }
    }
}

impl From<TypeId> for LogicalType {
    /// Creates a `LogicalType` from a `TypeId`.
    ///
    /// This is equivalent to calling [`LogicalType::new`].
    fn from(type_id: TypeId) -> Self {
        Self::new(type_id)
    }
}

// LogicalType is not Clone or Copy because the underlying handle is not reference-counted.
// If you need to pass it to multiple places, use `as_raw()` to borrow the handle temporarily.

#[cfg(test)]
mod tests {
    // Note: LogicalType tests that call DuckDB API (duckdb_create_logical_type)
    // require a running DuckDB runtime and are covered in tests/integration_test.rs.
    // The `loadable-extension` feature uses lazy-initialized function pointers
    // that cannot be called without a prior call to duckdb_rs_extension_api_init.

    #[test]
    fn size_of_logical_type_struct() {
        use super::LogicalType;
        // LogicalType must be pointer-sized (it contains a single pointer).
        assert_eq!(
            std::mem::size_of::<LogicalType>(),
            std::mem::size_of::<*mut ()>()
        );
    }
}
