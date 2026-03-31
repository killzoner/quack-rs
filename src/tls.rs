// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Type-erased TLS configuration provider for HTTP-capable extensions.
//!
//! Extensions that make outbound HTTPS connections (e.g., `duck_net`) need a way
//! to inject custom TLS configurations — client certificates for mTLS, custom CA
//! bundles, or restricted cipher suites. This module provides the
//! [`TlsConfigProvider`] trait so that extensions can supply their TLS setup
//! through a uniform interface, regardless of which TLS library they use.
//!
//! # Design
//!
//! The trait is **type-erased** via [`std::any::Any`] so that `quack-rs` does not
//! depend on any specific TLS library (e.g., `rustls`, `native-tls`). The
//! implementing extension downcasts the returned `Arc<dyn Any>` to its concrete
//! config type.
//!
//! # Example
//!
//! ```rust
//! use quack_rs::tls::TlsConfigProvider;
//! use std::any::Any;
//! use std::sync::Arc;
//!
//! // In your extension crate (which depends on rustls):
//! struct MyTlsProvider {
//!     // config: Arc<rustls::ClientConfig>,
//!     config: Arc<String>, // placeholder for illustration
//! }
//!
//! impl TlsConfigProvider for MyTlsProvider {
//!     fn client_config(&self) -> Arc<dyn Any + Send + Sync> {
//!         self.config.clone()
//!     }
//!
//!     fn provider_name(&self) -> &str {
//!         "my-extension-tls"
//!     }
//! }
//! ```

use std::any::Any;
use std::sync::Arc;

/// A type-erased provider of TLS client configuration.
///
/// Extensions that make outbound HTTPS connections implement this trait to
/// supply their TLS setup (client certificates, custom CA bundles, etc.)
/// without coupling `quack-rs` to a specific TLS library.
///
/// # Downcasting
///
/// The returned `Arc<dyn Any + Send + Sync>` should be downcast by the
/// consumer to the concrete config type. For example, an extension using
/// `rustls` would do:
///
/// ```rust,no_run
/// use std::any::Any;
/// use std::sync::Arc;
///
/// fn use_config(config: Arc<dyn Any + Send + Sync>) {
///     // let rustls_config = config.downcast_ref::<rustls::ClientConfig>()
///     //     .expect("expected rustls ClientConfig");
/// }
/// ```
pub trait TlsConfigProvider: Send + Sync {
    /// Returns the TLS client configuration as a type-erased `Arc`.
    ///
    /// Implementations should return an `Arc` wrapping their concrete TLS
    /// config type (e.g., `Arc<rustls::ClientConfig>`).
    fn client_config(&self) -> Arc<dyn Any + Send + Sync>;

    /// Returns a human-readable name for this TLS provider.
    ///
    /// Used in diagnostics and warning messages.
    fn provider_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider {
        config: Arc<String>,
    }

    #[allow(clippy::unnecessary_literal_bound)]
    impl TlsConfigProvider for TestProvider {
        fn client_config(&self) -> Arc<dyn Any + Send + Sync> {
            self.config.clone()
        }

        fn provider_name(&self) -> &str {
            "test-tls"
        }
    }

    #[test]
    fn provider_returns_config() {
        let provider = TestProvider {
            config: Arc::new("test-config".to_string()),
        };
        let config = provider.client_config();
        let s = config.downcast_ref::<String>().unwrap();
        assert_eq!(s, "test-config");
    }

    #[test]
    fn provider_name() {
        let provider = TestProvider {
            config: Arc::new(String::new()),
        };
        assert_eq!(provider.provider_name(), "test-tls");
    }

    #[test]
    fn provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestProvider>();
    }

    #[test]
    fn trait_object_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn TlsConfigProvider>();
    }
}
