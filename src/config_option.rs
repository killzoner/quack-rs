// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Extension-defined configuration options (`DuckDB` 1.5.0+).
//!
//! Extensions can register custom settings that users can read and write via
//! `SET` / `RESET` / `current_setting()`. This module wraps the
//! `duckdb_config_option` C API surface behind a safe builder.
//!
//! # Example
//!
//! ```rust,no_run
//! use quack_rs::config_option::{ConfigOptionBuilder, ConfigOptionScope};
//! use quack_rs::types::TypeId;
//!
//! let option = ConfigOptionBuilder::try_new("my_ext_threshold")?
//!     .description("Maximum threshold for my_ext operations")
//!     .option_type(TypeId::BigInt)
//!     .default_value("100")
//!     .scope(ConfigOptionScope::Global);
//! # Ok::<(), quack_rs::error::ExtensionError>(())
//! ```

use std::ffi::CString;

use libduckdb_sys::{
    duckdb_config_option, duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_GLOBAL,
    duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_LOCAL,
    duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_SESSION,
    duckdb_config_option_set_default_scope, duckdb_config_option_set_default_value,
    duckdb_config_option_set_description, duckdb_config_option_set_name,
    duckdb_config_option_set_type, duckdb_connection, duckdb_create_config_option,
    duckdb_create_varchar, duckdb_destroy_config_option, duckdb_destroy_value,
    duckdb_register_config_option, DuckDBSuccess,
};

use crate::error::ExtensionError;
use crate::types::{LogicalType, TypeId};

/// Scope in which a configuration option takes effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOptionScope {
    /// Option is local to the current statement.
    Local,
    /// Option is scoped to the current session.
    Session,
    /// Option applies globally to the database.
    Global,
}

impl ConfigOptionScope {
    /// Converts to the `DuckDB` C API scope constant.
    #[must_use]
    pub(crate) const fn to_raw(self) -> u32 {
        match self {
            Self::Local => duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_LOCAL,
            Self::Session => duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_SESSION,
            Self::Global => duckdb_config_option_scope_DUCKDB_CONFIG_OPTION_SCOPE_GLOBAL,
        }
    }
}

/// Builder for registering extension-defined configuration options.
///
/// After building, call [`register`][Self::register] from your entry point to
/// register the setting with `DuckDB`.
#[must_use]
pub struct ConfigOptionBuilder {
    name: CString,
    description: Option<CString>,
    option_type: Option<TypeId>,
    default_value: Option<CString>,
    scope: ConfigOptionScope,
}

impl ConfigOptionBuilder {
    /// Creates a new config option builder with the given name.
    ///
    /// # Errors
    ///
    /// Returns `ExtensionError` if the name contains a null byte.
    pub fn try_new(name: &str) -> Result<Self, ExtensionError> {
        let c_name = CString::new(name)
            .map_err(|_| ExtensionError::new("config option name contains null byte"))?;
        Ok(Self {
            name: c_name,
            description: None,
            option_type: None,
            default_value: None,
            scope: ConfigOptionScope::Global,
        })
    }

    /// Sets the human-readable description for this option.
    pub fn description(mut self, desc: &str) -> Self {
        self.description = CString::new(desc).ok();
        self
    }

    /// Sets the value type for this option (e.g. `TypeId::BigInt`, `TypeId::Varchar`).
    pub const fn option_type(mut self, type_id: TypeId) -> Self {
        self.option_type = Some(type_id);
        self
    }

    /// Sets the default value as a string representation.
    pub fn default_value(mut self, value: &str) -> Self {
        self.default_value = CString::new(value).ok();
        self
    }

    /// Sets the scope for this option.
    pub const fn scope(mut self, scope: ConfigOptionScope) -> Self {
        self.scope = scope;
        self
    }

    /// Registers this config option with `DuckDB`.
    ///
    /// # Errors
    ///
    /// Returns `ExtensionError` if the option type was not set or registration
    /// fails.
    ///
    /// # Safety
    ///
    /// `con` must be a valid, open `duckdb_connection`.
    pub unsafe fn register(self, con: duckdb_connection) -> Result<(), ExtensionError> {
        let type_id = self
            .option_type
            .ok_or_else(|| ExtensionError::new("config option type not set"))?;
        let lt = LogicalType::new(type_id);

        // SAFETY: duckdb_create_config_option allocates a new handle.
        let option: duckdb_config_option = unsafe { duckdb_create_config_option() };

        // SAFETY: option is a valid newly created handle.
        unsafe {
            duckdb_config_option_set_name(option, self.name.as_ptr());
            duckdb_config_option_set_type(option, lt.as_raw());
            duckdb_config_option_set_default_scope(option, self.scope.to_raw());
        }

        if let Some(ref desc) = self.description {
            // SAFETY: option and desc are valid.
            unsafe {
                duckdb_config_option_set_description(option, desc.as_ptr());
            }
        }

        if let Some(ref val) = self.default_value {
            // SAFETY: duckdb_create_varchar allocates a duckdb_value.
            let dv = unsafe { duckdb_create_varchar(val.as_ptr()) };
            // SAFETY: option and dv are valid.
            unsafe {
                duckdb_config_option_set_default_value(option, dv);
            }
            // SAFETY: dv was created by duckdb_create_varchar.
            let mut dv_mut = dv;
            unsafe {
                duckdb_destroy_value(&raw mut dv_mut);
            }
        }

        // SAFETY: con is valid per caller's contract, option is fully configured.
        let result = unsafe { duckdb_register_config_option(con, option) };

        // SAFETY: option was created above and must be destroyed after registration.
        let mut option_mut = option;
        unsafe {
            duckdb_destroy_config_option(&raw mut option_mut);
        }

        if result == DuckDBSuccess {
            Ok(())
        } else {
            Err(ExtensionError::new(format!(
                "duckdb_register_config_option failed for '{}'",
                self.name.to_string_lossy()
            )))
        }
    }
}
