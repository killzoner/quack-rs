# Structured Warnings

Extensions that access external resources (network, files, credentials) should
emit structured warnings when potentially unsafe operations occur. The
[`warning`](https://docs.rs/quack-rs/latest/quack_rs/warning/index.html) module
provides a consistent, thread-safe API for collecting and surfacing security
warnings.

## Core Types

### `ExtensionWarning`

A structured warning with:

| Field | Type | Description |
|-------|------|-------------|
| `code` | `&'static str` | Machine-readable code (e.g., `"TLS_NO_VERIFY"`) |
| `severity` | `WarningSeverity` | Info / Low / Medium / High / Critical |
| `message` | `String` | Human-readable description |
| `cwe` | `Option<u32>` | Optional [CWE](https://cwe.mitre.org/) identifier |

### `WarningSeverity`

Five levels mirroring common security advisory severity:

- **Info** — no security impact, but worth noting
- **Low** — minimal security impact
- **Medium** — potential security concern
- **High** — significant security risk
- **Critical** — immediate action recommended

### `WarningCollector`

A thread-safe collector backed by `Mutex<Vec<ExtensionWarning>>`. Safe to share
across threads via `Arc<WarningCollector>`.

## Usage

```rust
use quack_rs::warning::{ExtensionWarning, WarningSeverity, WarningCollector};

let collector = WarningCollector::new();

// Emit a warning when detecting an insecure configuration
collector.emit(ExtensionWarning {
    code: "TLS_NO_VERIFY",
    severity: WarningSeverity::High,
    message: "TLS certificate verification is disabled".into(),
    cwe: Some(295),
});

// Check warnings
assert_eq!(collector.len(), 1);
assert!(!collector.is_empty());

// Read without clearing
let snapshot = collector.snapshot();
assert_eq!(snapshot.len(), 1);
assert_eq!(collector.len(), 1);  // still there

// Consume all warnings
let warnings = collector.drain();
assert_eq!(warnings.len(), 1);
assert!(collector.is_empty());  // now empty
```

## Display Format

Warnings format as `[SEVERITY] CODE: message (CWE-nnn)`:

```text
[HIGH] TLS_NO_VERIFY: TLS certificate verification is disabled (CWE-295)
[MEDIUM] TLS_DEPRECATED_VERSION: TLS provider allows deprecated TLS 1.0 (CWE-327)
```

## Integration with TLS Auditing

The `audit_tls_provider()` function returns `Vec<ExtensionWarning>` that can be
fed directly into a `WarningCollector`:

```rust,no_run
use quack_rs::tls::audit_tls_provider;
use quack_rs::warning::WarningCollector;

// let warnings = audit_tls_provider(&my_tls_provider);
// let collector = WarningCollector::new();
// for w in warnings {
//     collector.emit(w);
// }
```

## Best Practices

- Create a single `WarningCollector` per extension (typically in global or
  bind-data state)
- Use `snapshot()` for read-only diagnostics; use `drain()` when consuming
  warnings for output
- Always include CWE identifiers for security-related warnings
- Surface collected warnings through a table function
  (e.g., `SELECT * FROM __extension_warnings()`)
