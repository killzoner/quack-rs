# Known Limitations

## Window functions are not available

DuckDB **window functions** (`OVER (...)` clauses) are implemented entirely in
DuckDB's C++ layer and have **no counterpart in the public C extension API**.

This is not a gap in `quack-rs` or in `libduckdb-sys` — the relevant symbol
(`duckdb_create_window_function`) simply does not exist in the C API:

| Symbol | C API (1.4.x)? | C API (1.5.0+)? | C++ API? |
|--------|----------------|-----------------|----------|
| `duckdb_create_window_function` | **No** | **No** | Yes |
| `duckdb_create_copy_function`   | **No** | **Yes** | Yes |
| `duckdb_create_scalar_function` | Yes    | Yes     | Yes |
| `duckdb_create_aggregate_function` | Yes | Yes     | Yes |
| `duckdb_create_table_function`  | Yes    | Yes     | Yes |
| `duckdb_create_cast_function`   | Yes    | Yes     | Yes |

**What this means for your extension:**

If your extension needs window-function semantics, you can approximate them with
aggregate functions in most cases (DuckDB will push down the window logic). True
custom window operator registration requires writing a C++ extension.

If DuckDB exposes window registration in a future C API version, `quack-rs`
will add wrappers in the corresponding release.

## COPY functions (resolved in DuckDB 1.5.0)

DuckDB 1.5.0 added `duckdb_create_copy_function` and related symbols to the public
C extension API. quack-rs wraps these in the `copy_function` module behind the
`duckdb-1-5` feature flag. See `CopyFunctionBuilder` for usage.

This was previously listed as a known limitation (no C API counterpart prior to 1.5.0).

## Callback accessor wrappers (planned for v0.8.0)

quack-rs wraps function **registration** (builders for scalar, aggregate, table,
copy functions) but does not yet wrap all **callback accessor** functions — the
C API functions used *inside* your callbacks to retrieve arguments, set errors,
access bind data, etc.

Specifically, the following accessor function groups are not yet wrapped:

| Category | Missing Functions | Impact |
|----------|------------------|--------|
| **Scalar function callbacks** | `bind_get_argument`, `bind_get_argument_count`, `bind_set_error`, `get_extra_info`, `set_bind_data`, `set_error`, `get_client_context` (14 functions) | Scalar bind/init callbacks cannot access their arguments or report errors through safe wrappers |
| **Copy function callbacks** | `bind_get_column_count`, `bind_get_column_type`, `bind_set_bind_data`, `sink_get_bind_data`, `finalize_get_global_state`, etc. (25 functions) | Copy function callbacks cannot access column info, bind data, or global state |
| **Aggregate function extras** | `get_extra_info`, `set_error`, `set_extra_info` (3 functions) | Cannot set extra info or report errors from aggregate callbacks |
| **Table function introspection** | `bind_get_result_column_count`, `bind_get_result_column_name`, `bind_get_result_column_type`, `get_client_context` (4 functions) | Table functions cannot inspect their result schema during bind |

**Workaround:** Extension authors can call the underlying `libduckdb_sys` functions
directly in their `unsafe` callback implementations. The raw function pointers are
available and fully functional.

**Plan:** Safe wrappers for all callback accessors are planned for v0.8.0.

## Complex type creation

`LogicalType::new(TypeId)` creates simple logical types. Creating complex
parameterized types (decimal, enum, array, union) requires calling the
underlying `libduckdb_sys` functions directly:

| Function | Purpose |
|----------|---------|
| `duckdb_create_decimal_type(width, scale)` | `DECIMAL(p, s)` |
| `duckdb_create_enum_type(members, count)` | `ENUM('a', 'b', ...)` |
| `duckdb_create_array_type(child, size)` | `type[N]` |
| `duckdb_create_union_type(members, ...)` | `UNION(a INT, b VARCHAR)` |

`LIST`, `STRUCT`, and `MAP` creation *is* already supported through
`LogicalType` helper methods. The remaining complex type constructors
are planned for a future release.

## VARIANT type (Iceberg v3)

DuckDB v1.5.1 introduced the `VARIANT` type for Iceberg v3 support.
This type is **not yet exposed** in the DuckDB C Extension API
(`DUCKDB_TYPE_VARIANT` does not exist in libduckdb-sys 1.10501.0).
quack-rs will add `TypeId::Variant` when the C API exposes it.
