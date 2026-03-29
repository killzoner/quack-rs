// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Auto-sizing chunk writer for table function scan callbacks.
//!
//! [`ChunkWriter`] wraps a `duckdb_data_chunk` and tracks rows written. On drop,
//! it automatically calls `duckdb_data_chunk_set_size` with the number of rows
//! written. This eliminates the error-prone pattern of manually calling
//! `set_size` after writing rows — a common source of bugs in table functions.
//!
//! # Example
//!
//! ```rust,no_run
//! use quack_rs::chunk_writer::ChunkWriter;
//! use libduckdb_sys::{duckdb_function_info, duckdb_data_chunk};
//!
//! unsafe extern "C" fn my_scan(info: duckdb_function_info, output: duckdb_data_chunk) {
//!     let mut cw = unsafe { ChunkWriter::new(output) };
//!     // Write some rows
//!     if let Some(row) = cw.next_row() {
//!         unsafe { cw.writer(0).write_varchar(row, "hello") };
//!         unsafe { cw.writer(1).write_i64(row, 42) };
//!     }
//!     // set_size is called automatically when `cw` is dropped
//! }
//! ```
//!
//! # Estimated impact
//!
//! Eliminates ~15 manual `set_size` calls and prevents off-by-one errors.

use libduckdb_sys::{duckdb_data_chunk, duckdb_data_chunk_set_size, idx_t};

use crate::vector::VectorWriter;

/// The default maximum number of rows per chunk in `DuckDB`.
const STANDARD_VECTOR_SIZE: usize = 2048;

/// A row-tracking writer for a `DuckDB` output data chunk.
///
/// Tracks the number of rows written via [`next_row`][Self::next_row] and
/// automatically calls `duckdb_data_chunk_set_size` on drop.
pub struct ChunkWriter {
    raw: duckdb_data_chunk,
    row_count: usize,
    capacity: usize,
}

impl ChunkWriter {
    /// Creates a new `ChunkWriter` for the given output data chunk.
    ///
    /// The capacity defaults to [`STANDARD_VECTOR_SIZE`] (2048).
    ///
    /// # Safety
    ///
    /// `chunk` must be a valid, writable `duckdb_data_chunk` obtained from a
    /// `DuckDB` table function scan callback.
    #[inline]
    #[must_use]
    pub const unsafe fn new(chunk: duckdb_data_chunk) -> Self {
        Self {
            raw: chunk,
            row_count: 0,
            capacity: STANDARD_VECTOR_SIZE,
        }
    }

    /// Creates a new `ChunkWriter` with a custom capacity.
    ///
    /// # Safety
    ///
    /// Same as [`new`][Self::new]. `capacity` must not exceed the chunk's
    /// actual capacity.
    #[inline]
    #[must_use]
    pub const unsafe fn with_capacity(chunk: duckdb_data_chunk, capacity: usize) -> Self {
        Self {
            raw: chunk,
            row_count: 0,
            capacity,
        }
    }

    /// Returns `true` if the chunk has reached its capacity.
    #[inline]
    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.row_count >= self.capacity
    }

    /// Returns the next available row index, or `None` if the chunk is full.
    ///
    /// Each call increments the internal row counter.
    #[inline]
    pub const fn next_row(&mut self) -> Option<usize> {
        if self.is_full() {
            return None;
        }
        let row = self.row_count;
        self.row_count += 1;
        Some(row)
    }

    /// Returns the number of rows written so far.
    #[inline]
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }

    /// Returns the chunk's capacity (maximum rows).
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Creates a [`VectorWriter`] for the given column index.
    ///
    /// # Safety
    ///
    /// `col_idx` must be less than the chunk's column count.
    pub unsafe fn writer(&self, col_idx: usize) -> VectorWriter {
        // SAFETY: self.raw is valid per constructor. col_idx is in bounds per caller.
        let vec =
            unsafe { libduckdb_sys::duckdb_data_chunk_get_vector(self.raw, col_idx as idx_t) };
        // SAFETY: vec is a valid writable vector from the output chunk.
        unsafe { VectorWriter::from_vector(vec) }
    }

    /// Returns the raw `duckdb_data_chunk` handle.
    #[inline]
    #[must_use]
    pub const fn as_raw(&self) -> duckdb_data_chunk {
        self.raw
    }

    /// Manually sets the chunk size and consumes the writer without auto-setting
    /// size on drop. Use this if you need to override the auto-calculated size.
    ///
    /// # Safety
    ///
    /// - `size` must not exceed the chunk's capacity (typically 2048).
    /// - `size` must match the actual number of rows written.
    pub unsafe fn finish_with_size(self, size: usize) {
        // SAFETY: self.raw is valid per constructor.
        unsafe { duckdb_data_chunk_set_size(self.raw, size as idx_t) };
        // Prevent Drop from setting size again.
        std::mem::forget(self);
    }
}

impl Drop for ChunkWriter {
    fn drop(&mut self) {
        // SAFETY: self.raw is valid per constructor's contract.
        // Set the chunk size to the number of rows written.
        unsafe { duckdb_data_chunk_set_size(self.raw, self.row_count as idx_t) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_chunk_writer() {
        assert_eq!(
            std::mem::size_of::<ChunkWriter>(),
            std::mem::size_of::<usize>() * 3 // pointer + row_count + capacity
        );
    }

    #[test]
    fn next_row_increments() {
        // We can't call Drop safely without a real chunk, so we use forget.
        let mut cw = ChunkWriter {
            raw: std::ptr::null_mut(),
            row_count: 0,
            capacity: 3,
        };
        assert_eq!(cw.next_row(), Some(0));
        assert_eq!(cw.next_row(), Some(1));
        assert_eq!(cw.next_row(), Some(2));
        assert_eq!(cw.next_row(), None);
        assert!(cw.is_full());
        assert_eq!(cw.row_count(), 3);
        // Forget to avoid calling Drop with a null pointer in FFI.
        std::mem::forget(cw);
    }

    #[test]
    fn is_full_at_zero_capacity() {
        let cw = ChunkWriter {
            raw: std::ptr::null_mut(),
            row_count: 0,
            capacity: 0,
        };
        assert!(cw.is_full());
        std::mem::forget(cw);
    }
}
