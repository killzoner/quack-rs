# Reading & Writing Vectors

DuckDB passes data to and from your extension as **vectors** — columnar arrays of typed
values, with a separate NULL bitmap. `VectorReader` and `VectorWriter` provide safe,
typed access to these vectors.

---

## `VectorReader`

### Construction

```rust
// In a scalar function callback:
let reader = unsafe { VectorReader::new(input, column_index) };

// In an aggregate update callback:
let reader = unsafe { VectorReader::new(input, 0) };   // first column
```

`VectorReader::new` takes the `duckdb_data_chunk` and a zero-based column index. The
reader borrows the chunk — it must not outlive the callback.

### Row count

```rust
let n = reader.row_count();   // number of rows in this chunk
```

Chunk sizes vary. Always loop from `0..reader.row_count()`, never assume a fixed size.

### NULL check

```rust
if unsafe { !reader.is_valid(row) } {
    // row is NULL — skip or propagate NULL to output
    unsafe { writer.set_null(row) };
    continue;
}
```

**Always check `is_valid` before reading.** Reading from a NULL row returns garbage data.

### Reading values

```rust
let i: i8  = unsafe { reader.read_i8(row) };
let i: i16 = unsafe { reader.read_i16(row) };
let i: i32 = unsafe { reader.read_i32(row) };
let i: i64 = unsafe { reader.read_i64(row) };
let u: u8  = unsafe { reader.read_u8(row) };
let u: u16 = unsafe { reader.read_u16(row) };
let u: u32 = unsafe { reader.read_u32(row) };
let u: u64 = unsafe { reader.read_u64(row) };
let f: f32 = unsafe { reader.read_f32(row) };
let f: f64 = unsafe { reader.read_f64(row) };
let b: bool = unsafe { reader.read_bool(row) };   // safe: uses u8 != 0
let s: &str = unsafe { reader.read_str(row) };    // handles inline + pointer format
let iv = unsafe { reader.read_interval(row) };    // returns DuckInterval
```

---

## `VectorWriter`

### Construction

```rust
// In a scalar function callback:
let mut writer = unsafe { VectorWriter::new(output) };

// In an aggregate finalize callback:
let mut writer = unsafe { VectorWriter::new(result) };
```

### Writing values

```rust
unsafe { writer.write_i8(row, value) };
unsafe { writer.write_i16(row, value) };
unsafe { writer.write_i32(row, value) };
unsafe { writer.write_i64(row, value) };
unsafe { writer.write_u8(row, value) };
unsafe { writer.write_u16(row, value) };
unsafe { writer.write_u32(row, value) };
unsafe { writer.write_u64(row, value) };
unsafe { writer.write_f32(row, value) };
unsafe { writer.write_f64(row, value) };
unsafe { writer.write_bool(row, value) };
unsafe { writer.write_varchar(row, s) };   // &str (also available as write_str)
unsafe { writer.write_str(row, s) };       // alias for write_varchar
unsafe { writer.write_interval(row, interval) };  // DuckInterval
```

### Writing NULL

```rust
unsafe { writer.set_null(row) };
```

> **Pitfall L4**: `set_null` calls `duckdb_vector_ensure_validity_writable` automatically
> before accessing the validity bitmap. Calling `duckdb_vector_get_validity` without this
> prerequisite returns an uninitialized pointer → SEGFAULT. `VectorWriter::set_null` handles
> this correctly. See [Pitfall L4](../reference/pitfalls.md#l4-ensure_validity_writable-is-required-before-null-output).

---

## `DataChunk`

`DataChunk` wraps a `duckdb_data_chunk` handle, providing ergonomic access to
vectors and metadata without raw FFI calls:

```rust
use quack_rs::data_chunk::DataChunk;

unsafe extern "C" fn my_scan(info: duckdb_function_info, output: duckdb_data_chunk) {
    let chunk = unsafe { DataChunk::from_raw(output) };
    let mut writer = unsafe { chunk.writer(0) };    // VectorWriter for column 0
    unsafe { writer.write_i64(0, 42) };
    unsafe { chunk.set_size(1) };                   // set output row count
}
```

Methods:
- `size()` — current row count
- `set_size(n)` — set row count (0 = end of stream)
- `column_count()` — number of columns
- `vector(col)` — raw `duckdb_vector` handle
- `writer(col)` — `VectorWriter` for a column
- `reader(col)` — `VectorReader` for a column

---

## `ValidityBitmap`

For advanced NULL handling beyond `VectorWriter::set_null`, use `ValidityBitmap`
directly:

```rust
use quack_rs::vector::ValidityBitmap;

// Writing NULLs:
let mut bitmap = unsafe { ValidityBitmap::ensure_writable(some_vector) };
unsafe { bitmap.set_row_invalid(row as u64) };   // mark as NULL
unsafe { bitmap.set_row_valid(row as u64) };     // mark as non-NULL

// Reading NULLs:
let bitmap = unsafe { ValidityBitmap::get_read_only(some_vector) };
let is_valid = unsafe { bitmap.row_is_valid(row as u64) };
```

`ValidityBitmap` is available in the prelude: `use quack_rs::prelude::*`.

---

## Utility functions

The `quack_rs::vector` module provides two utility functions:

```rust
use quack_rs::vector::{vector_size, vector_get_column_type};

// Returns the default vector size used by DuckDB (typically 2048).
let size: u64 = vector_size();

// Returns the LogicalType of a vector (unsafe — requires a valid duckdb_vector).
let lt = unsafe { vector_get_column_type(some_vector) };
```

---

## Memory layout details

DuckDB stores vector data as flat arrays. `VectorReader` and `VectorWriter` compute
element addresses as `base_ptr + row * stride`:

```
[value0][value1][value2]...[valueN]   ← typed array
[validity bitmap]                      ← separate bit array, 1 bit per row
```

The validity bitmap is lazily allocated — it may be null if no NULLs have been written.
This is why `ensure_validity_writable` must be called before any `get_validity` call
that follows a write path.

---

## Complete scalar function pattern

```rust
unsafe extern "C" fn my_scalar(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    output: duckdb_vector,
) {
    let reader = unsafe { VectorReader::new(input, 0) };
    let mut writer = unsafe { VectorWriter::new(output) };

    for row in 0..reader.row_count() {
        if unsafe { !reader.is_valid(row) } {
            unsafe { writer.set_null(row) };
            continue;
        }
        let value = unsafe { reader.read_i64(row) };
        unsafe { writer.write_i64(row, transform(value)) };
    }
}
```
