// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Table description metadata (`DuckDB` 1.5.0+).
//!
//! Allows querying table structure (column count, names, and types) at runtime
//! from within an extension. Useful for replacement scans, table functions,
//! and copy functions that need to inspect existing tables.
//!
//! # Example
//!
//! ```rust,no_run
//! use quack_rs::table_description::TableDescription;
//!
//! // From within a function callback with a valid connection:
//! // let desc = unsafe { TableDescription::create(con, "main", "my_table")? };
//! // let col_count = desc.column_count();
//! ```

use std::ffi::{CStr, CString};

use libduckdb_sys::{
    duckdb_connection, duckdb_logical_type, duckdb_table_description,
    duckdb_table_description_create, duckdb_table_description_destroy,
    duckdb_table_description_error, duckdb_table_description_get_column_count,
    duckdb_table_description_get_column_name, duckdb_table_description_get_column_type, idx_t,
};

use crate::error::ExtensionError;

/// RAII wrapper for a `duckdb_table_description`.
///
/// Provides metadata about a table's columns. Automatically destroyed on drop.
pub struct TableDescription {
    desc: duckdb_table_description,
}

impl TableDescription {
    /// Creates a table description for the given schema and table.
    ///
    /// # Errors
    ///
    /// Returns `ExtensionError` if the table does not exist or cannot be described.
    ///
    /// # Safety
    ///
    /// `con` must be a valid, open `duckdb_connection`.
    pub unsafe fn create(
        con: duckdb_connection,
        schema: &str,
        table: &str,
    ) -> Result<Self, ExtensionError> {
        let c_schema = CString::new(schema)
            .map_err(|_| ExtensionError::new("schema name contains null byte"))?;
        let c_table = CString::new(table)
            .map_err(|_| ExtensionError::new("table name contains null byte"))?;

        let mut desc: duckdb_table_description = core::ptr::null_mut();
        // SAFETY: con is valid per caller's contract.
        let rc = unsafe {
            duckdb_table_description_create(con, c_schema.as_ptr(), c_table.as_ptr(), &raw mut desc)
        };

        if rc != libduckdb_sys::DuckDBSuccess || desc.is_null() {
            // Try to get the error message.
            if !desc.is_null() {
                let err_ptr = unsafe { duckdb_table_description_error(desc) };
                if !err_ptr.is_null() {
                    let msg = unsafe { CStr::from_ptr(err_ptr) }
                        .to_str()
                        .unwrap_or("unknown error");
                    let err = ExtensionError::new(format!(
                        "failed to describe table '{schema}.{table}': {msg}"
                    ));
                    unsafe { duckdb_table_description_destroy(&raw mut desc) };
                    return Err(err);
                }
                unsafe { duckdb_table_description_destroy(&raw mut desc) };
            }
            return Err(ExtensionError::new(format!(
                "failed to describe table '{schema}.{table}'"
            )));
        }

        Ok(Self { desc })
    }

    /// Returns the number of columns in the table.
    #[must_use]
    pub fn column_count(&self) -> idx_t {
        // SAFETY: self.desc is valid.
        unsafe { duckdb_table_description_get_column_count(self.desc) }
    }

    /// Returns the name of the column at the given index.
    ///
    /// Returns `None` if the index is out of bounds or the name is not valid UTF-8.
    #[must_use]
    pub fn column_name(&self, index: idx_t) -> Option<String> {
        // SAFETY: self.desc is valid. `DuckDB` returns a newly allocated string.
        let ptr = unsafe { duckdb_table_description_get_column_name(self.desc, index) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: ptr is a valid null-terminated string allocated by `DuckDB`.
        let result = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .ok()
            .map(String::from);
        // Free the string allocated by `DuckDB`.
        unsafe {
            libduckdb_sys::duckdb_free(ptr.cast::<core::ffi::c_void>());
        }
        result
    }

    /// Returns the logical type of the column at the given index.
    ///
    /// Returns `None` if the index is out of bounds. The returned handle must be
    /// destroyed by the caller (it is a raw `duckdb_logical_type`).
    #[must_use]
    pub fn column_type(&self, index: idx_t) -> Option<duckdb_logical_type> {
        // SAFETY: self.desc is valid.
        let lt = unsafe { duckdb_table_description_get_column_type(self.desc, index) };
        if lt.is_null() {
            None
        } else {
            Some(lt)
        }
    }
}

impl Drop for TableDescription {
    fn drop(&mut self) {
        // SAFETY: self.desc was obtained from duckdb_table_description_create.
        unsafe {
            duckdb_table_description_destroy(&raw mut self.desc);
        }
    }
}
