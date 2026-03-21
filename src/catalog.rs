// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Catalog entry lookup (`DuckDB` 1.5.0+).
//!
//! Provides read-only access to catalog entries (tables, views, types, etc.)
//! from within extension callbacks.
//!
//! # Example
//!
//! ```rust,no_run
//! use quack_rs::catalog::{CatalogEntryType, CatalogEntry};
//! ```

use std::ffi::CStr;

use libduckdb_sys::{
    duckdb_catalog, duckdb_catalog_entry, duckdb_catalog_entry_get_name,
    duckdb_catalog_entry_get_type, duckdb_catalog_entry_type,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_COLLATION,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_DATABASE,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_INDEX,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_INVALID,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_PREPARED_STATEMENT,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SCHEMA,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SEQUENCE,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TABLE,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TYPE,
    duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_VIEW, duckdb_catalog_get_entry,
    duckdb_client_context, duckdb_destroy_catalog, duckdb_destroy_catalog_entry,
};

/// Types of entries in the `DuckDB` catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogEntryType {
    /// Invalid catalog entry.
    Invalid,
    /// A table.
    Table,
    /// A view.
    View,
    /// An index.
    Index,
    /// A schema.
    Schema,
    /// A prepared statement.
    PreparedStatement,
    /// A sequence.
    Sequence,
    /// A collation.
    Collation,
    /// A user-defined type.
    Type,
    /// A database.
    Database,
}

impl CatalogEntryType {
    /// Converts to the `DuckDB` C API constant.
    #[must_use]
    pub(crate) const fn to_raw(self) -> duckdb_catalog_entry_type {
        match self {
            Self::Invalid => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_INVALID,
            Self::Table => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TABLE,
            Self::View => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_VIEW,
            Self::Index => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_INDEX,
            Self::Schema => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SCHEMA,
            Self::PreparedStatement => {
                duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_PREPARED_STATEMENT
            }
            Self::Sequence => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SEQUENCE,
            Self::Collation => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_COLLATION,
            Self::Type => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TYPE,
            Self::Database => duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_DATABASE,
        }
    }

    /// Converts from the `DuckDB` C API constant.
    #[must_use]
    pub(crate) const fn from_raw(raw: duckdb_catalog_entry_type) -> Self {
        match raw {
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TABLE => Self::Table,
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_VIEW => Self::View,
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_INDEX => Self::Index,
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SCHEMA => Self::Schema,
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_PREPARED_STATEMENT => {
                Self::PreparedStatement
            }
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_SEQUENCE => {
                Self::Sequence
            }
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_COLLATION => {
                Self::Collation
            }
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_TYPE => Self::Type,
            x if x == duckdb_catalog_entry_type_DUCKDB_CATALOG_ENTRY_TYPE_DATABASE => {
                Self::Database
            }
            _ => Self::Invalid,
        }
    }
}

/// RAII wrapper for a `duckdb_catalog_entry`.
///
/// Automatically destroyed when dropped.
pub struct CatalogEntry {
    entry: duckdb_catalog_entry,
}

impl CatalogEntry {
    /// Look up a catalog entry by type, schema, and name.
    ///
    /// # Safety
    ///
    /// - `catalog` must be a valid `duckdb_catalog` handle.
    /// - `context` must be a valid `duckdb_client_context` handle.
    /// - Must be called from within an active transaction.
    pub unsafe fn lookup(
        catalog: duckdb_catalog,
        context: duckdb_client_context,
        schema: &CStr,
        name: &CStr,
        entry_type: CatalogEntryType,
    ) -> Option<Self> {
        // SAFETY: catalog, context, schema, and name are valid per caller's contract.
        let entry = unsafe {
            duckdb_catalog_get_entry(
                catalog,
                context,
                entry_type.to_raw(),
                schema.as_ptr(),
                name.as_ptr(),
            )
        };
        if entry.is_null() {
            None
        } else {
            Some(Self { entry })
        }
    }

    /// Returns the name of this catalog entry.
    ///
    /// Returns `None` if the name is not valid UTF-8.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        // SAFETY: self.entry is valid.
        let ptr = unsafe { duckdb_catalog_entry_get_name(self.entry) };
        if ptr.is_null() {
            return None;
        }
        // SAFETY: `DuckDB` returns a null-terminated UTF-8 string.
        unsafe { CStr::from_ptr(ptr) }.to_str().ok()
    }

    /// Returns the type of this catalog entry.
    #[must_use]
    pub fn entry_type(&self) -> CatalogEntryType {
        // SAFETY: self.entry is valid.
        let raw = unsafe { duckdb_catalog_entry_get_type(self.entry) };
        CatalogEntryType::from_raw(raw)
    }
}

impl Drop for CatalogEntry {
    fn drop(&mut self) {
        // SAFETY: self.entry was obtained from duckdb_catalog_get_entry.
        unsafe {
            duckdb_destroy_catalog_entry(&raw mut self.entry);
        }
    }
}

/// RAII wrapper for a `duckdb_catalog`.
///
/// Automatically destroyed when dropped.
pub struct Catalog {
    catalog: duckdb_catalog,
}

impl Catalog {
    /// Creates a `Catalog` from a raw handle.
    ///
    /// # Safety
    ///
    /// `catalog` must be a valid, non-null `duckdb_catalog` handle.
    pub(crate) const unsafe fn from_raw(catalog: duckdb_catalog) -> Self {
        Self { catalog }
    }

    /// Returns the raw handle for use with [`CatalogEntry::lookup`].
    #[must_use]
    pub const fn as_raw(&self) -> duckdb_catalog {
        self.catalog
    }

    /// Look up a catalog entry by type, schema, and name.
    ///
    /// # Safety
    ///
    /// - `context` must be a valid `duckdb_client_context`.
    /// - Must be called from within an active transaction context.
    pub unsafe fn get_entry(
        &self,
        context: duckdb_client_context,
        schema: &CStr,
        name: &CStr,
        entry_type: CatalogEntryType,
    ) -> Option<CatalogEntry> {
        // SAFETY: self.catalog and context are valid, caller ensures active transaction.
        unsafe { CatalogEntry::lookup(self.catalog, context, schema, name, entry_type) }
    }
}

impl Drop for Catalog {
    fn drop(&mut self) {
        // SAFETY: self.catalog was obtained from duckdb_client_context_get_catalog.
        unsafe {
            duckdb_destroy_catalog(&raw mut self.catalog);
        }
    }
}
