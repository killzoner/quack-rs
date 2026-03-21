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
