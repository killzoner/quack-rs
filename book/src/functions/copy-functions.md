# Copy Functions

> **Requires the `duckdb-1-5` feature flag** (DuckDB 1.5.0+).

Copy functions let you implement custom `COPY TO` file format handlers. When a
user runs `COPY table TO 'file.xyz' (FORMAT my_format)`, DuckDB invokes your
extension's bind, init, sink, and finalize callbacks.

## Lifecycle

1. **Bind** — called once. Inspect output columns, configure the export.
2. **Global init** — called once. Open the output file, allocate global state.
3. **Sink** — called once per data chunk. Write rows to the output.
4. **Finalize** — called once. Flush buffers, close the file.

## Builder API

```rust,no_run
use quack_rs::copy_function::CopyFunctionBuilder;

let builder = CopyFunctionBuilder::try_new("my_format")?
    .bind(my_bind_fn)
    .global_init(my_global_init_fn)
    .sink(my_sink_fn)
    .finalize(my_finalize_fn);

// Register via Connection (inside entry_point_v2! callback):
// unsafe { con.register_copy_function(builder)?; }
# Ok::<(), quack_rs::error::ExtensionError>(())
```

## Callback signatures

| Phase | Signature |
|-------|-----------|
| Bind | `unsafe extern "C" fn(info: duckdb_copy_function_bind_info)` |
| Global init | `unsafe extern "C" fn(info: duckdb_copy_function_global_init_info)` |
| Sink | `unsafe extern "C" fn(info: duckdb_copy_function_sink_info, chunk: duckdb_data_chunk)` |
| Finalize | `unsafe extern "C" fn(info: duckdb_copy_function_finalize_info)` |

## Related modules

- [`config_option`](https://docs.rs/quack-rs/latest/quack_rs/config_option/) — register custom settings for your format
- [`client_context`](https://docs.rs/quack-rs/latest/quack_rs/client_context/) — access the file system and catalog from callbacks
- [`table_description`](https://docs.rs/quack-rs/latest/quack_rs/table_description/) — inspect table metadata
- [`catalog`](https://docs.rs/quack-rs/latest/quack_rs/catalog/) — look up catalog entries
