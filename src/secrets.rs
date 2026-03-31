// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Secrets manager bridge for extensions.
//!
//! Extensions that access external services (HTTP APIs, databases, cloud storage)
//! commonly need credentials. `DuckDB` provides a native secrets API via
//! `CREATE SECRET`, and this module defines the Rust-side traits and types that
//! extensions implement to bridge into that system.
//!
//! [`SecretsManager`] is the trait that extensions implement to provide secret
//! lookup. [`SecretEntry`] is the returned secret value.
//!
//! # Example
//!
//! ```rust
//! use quack_rs::secrets::{SecretEntry, SecretsManager};
//!
//! struct MySecrets {
//!     // In practice, backed by DuckDB's CREATE SECRET storage
//!     entries: Vec<SecretEntry>,
//! }
//!
//! impl SecretsManager for MySecrets {
//!     fn get_secret(&self, name: &str, secret_type: &str) -> Option<SecretEntry> {
//!         self.entries.iter()
//!             .find(|e| e.name == name && e.secret_type == secret_type)
//!             .cloned()
//!     }
//!
//!     fn list_secrets(&self, secret_type: Option<&str>) -> Vec<SecretEntry> {
//!         self.entries.iter()
//!             .filter(|e| secret_type.is_none() || secret_type == Some(e.secret_type.as_str()))
//!             .cloned()
//!             .collect()
//!     }
//!
//!     fn remove_secret(&self, _name: &str, _secret_type: &str) -> bool {
//!         false // read-only example
//!     }
//! }
//! ```

use std::collections::HashMap;

/// A single secret entry retrieved from the secrets manager.
///
/// Contains the secret's metadata and key-value pairs. The `fields` map holds
/// the actual secret data (e.g., `"token"`, `"username"`, `"password"`,
/// `"client_cert_path"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEntry {
    /// The name of the secret (as given in `CREATE SECRET name ...`).
    pub name: String,

    /// The secret type (e.g., `"bearer"`, `"s3"`, `"gcs"`, `"azure"`).
    pub secret_type: String,

    /// The provider that created this secret (e.g., `"config"`, `"credential_chain"`).
    pub provider: String,

    /// The scope/pattern this secret applies to (e.g., `"s3://my-bucket"`).
    /// Empty string if unscoped.
    pub scope: String,

    /// Key-value pairs holding the secret data.
    ///
    /// Common keys include `"token"`, `"key_id"`, `"secret"`, `"region"`,
    /// `"endpoint"`, `"account_name"`, etc. The exact keys depend on the
    /// secret type.
    pub fields: HashMap<String, String>,
}

impl SecretEntry {
    /// Creates a new `SecretEntry` with the given name and type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use quack_rs::secrets::SecretEntry;
    ///
    /// let entry = SecretEntry::new("my_api_key", "bearer")
    ///     .with_provider("config")
    ///     .with_field("token", "sk-abc123");
    /// assert_eq!(entry.name, "my_api_key");
    /// assert_eq!(entry.fields.get("token").unwrap(), "sk-abc123");
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>, secret_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            secret_type: secret_type.into(),
            provider: String::new(),
            scope: String::new(),
            fields: HashMap::new(),
        }
    }

    /// Sets the provider for this secret entry.
    #[must_use]
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Sets the scope for this secret entry.
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = scope.into();
        self
    }

    /// Adds a key-value field to this secret entry.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Returns the value of a field, if present.
    #[must_use]
    pub fn get_field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

/// Trait for accessing `DuckDB`'s secrets management system.
///
/// Extensions implement this trait to provide a safe Rust interface over
/// `DuckDB`'s native `CREATE SECRET` / `DROP SECRET` storage. A typical
/// implementation wraps `DuckDB`'s C API or maintains an in-memory cache
/// synchronized with the `DuckDB` catalog.
///
/// # Thread safety
///
/// Implementations must be safe to call from multiple threads. `DuckDB` may
/// invoke extension callbacks concurrently.
pub trait SecretsManager: Send + Sync {
    /// Retrieves a secret by name and type.
    ///
    /// Returns `None` if no matching secret exists.
    fn get_secret(&self, name: &str, secret_type: &str) -> Option<SecretEntry>;

    /// Lists all secrets, optionally filtered by type.
    ///
    /// If `secret_type` is `None`, all secrets are returned.
    fn list_secrets(&self, secret_type: Option<&str>) -> Vec<SecretEntry>;

    /// Removes a secret by name and type.
    ///
    /// Returns `true` if the secret was found and removed, `false` otherwise.
    fn remove_secret(&self, name: &str, secret_type: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_entry_builder() {
        let entry = SecretEntry::new("test", "bearer")
            .with_provider("config")
            .with_scope("https://api.example.com")
            .with_field("token", "abc123")
            .with_field("refresh_token", "xyz789");

        assert_eq!(entry.name, "test");
        assert_eq!(entry.secret_type, "bearer");
        assert_eq!(entry.provider, "config");
        assert_eq!(entry.scope, "https://api.example.com");
        assert_eq!(entry.get_field("token"), Some("abc123"));
        assert_eq!(entry.get_field("refresh_token"), Some("xyz789"));
        assert_eq!(entry.get_field("nonexistent"), None);
    }

    #[test]
    fn secret_entry_new_defaults() {
        let entry = SecretEntry::new("s", "s3");
        assert_eq!(entry.provider, "");
        assert_eq!(entry.scope, "");
        assert!(entry.fields.is_empty());
    }

    #[test]
    fn secret_entry_clone_eq() {
        let e1 = SecretEntry::new("a", "b").with_field("k", "v");
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    struct InMemorySecrets {
        entries: Vec<SecretEntry>,
    }

    impl SecretsManager for InMemorySecrets {
        fn get_secret(&self, name: &str, secret_type: &str) -> Option<SecretEntry> {
            self.entries
                .iter()
                .find(|e| e.name == name && e.secret_type == secret_type)
                .cloned()
        }

        fn list_secrets(&self, secret_type: Option<&str>) -> Vec<SecretEntry> {
            self.entries
                .iter()
                .filter(|e| secret_type.is_none() || secret_type == Some(e.secret_type.as_str()))
                .cloned()
                .collect()
        }

        fn remove_secret(&self, _name: &str, _secret_type: &str) -> bool {
            false
        }
    }

    #[test]
    fn in_memory_secrets_manager() {
        let mgr = InMemorySecrets {
            entries: vec![
                SecretEntry::new("api_key", "bearer").with_field("token", "t1"),
                SecretEntry::new("bucket", "s3").with_field("key_id", "k1"),
            ],
        };

        assert!(mgr.get_secret("api_key", "bearer").is_some());
        assert!(mgr.get_secret("api_key", "s3").is_none());
        assert!(mgr.get_secret("missing", "bearer").is_none());

        let all = mgr.list_secrets(None);
        assert_eq!(all.len(), 2);

        let s3_only = mgr.list_secrets(Some("s3"));
        assert_eq!(s3_only.len(), 1);
        assert_eq!(s3_only[0].name, "bucket");
    }

    #[test]
    fn secrets_manager_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InMemorySecrets>();
    }
}
