// SPDX-License-Identifier: MIT
// Copyright 2026 Tom F. <https://github.com/tomtom215/>
// My way of giving something small back to the open source community
// and encouraging more Rust development!

//! Batched, typed writer for STRUCT output vectors.
//!
//! [`StructWriter`] pre-creates [`VectorWriter`]s for every field at construction,
//! then exposes typed `write_*` methods that take `(row, field_idx, value)`.
//! This eliminates the repetitive `duckdb_struct_vector_get_child` + manual
//! `VectorWriter` creation that extension authors currently need for every field.
//!
//! # Example
//!
//! ```rust,no_run
//! use quack_rs::vector::StructWriter;
//! use libduckdb_sys::duckdb_vector;
//!
//! // Inside a scan callback, given a STRUCT output vector with 5 fields:
//! // let mut sw = unsafe { StructWriter::new(struct_vec, 5) };
//! // unsafe {
//! //     sw.write_bool(0, 0, result.success);
//! //     sw.write_varchar(0, 1, &result.data);
//! //     sw.write_i64(0, 2, result.lease);
//! //     sw.write_bool(0, 3, result.renewable);
//! //     sw.write_varchar(0, 4, &result.message);
//! // }
//! ```
//!
//! # Estimated impact
//!
//! Eliminates ~120 raw `duckdb_struct_vector_get_child` calls across typical
//! extensions, reducing unsafe surface area by ~30%.

use libduckdb_sys::duckdb_vector;

use crate::interval::DuckInterval;
use crate::vector::complex::StructVector;
use crate::vector::VectorWriter;

/// A batched writer for STRUCT output vectors.
///
/// Pre-creates a [`VectorWriter`] for every field at construction, allowing
/// direct typed writes without repeated `duckdb_struct_vector_get_child` calls.
pub struct StructWriter {
    fields: Vec<VectorWriter>,
}

impl StructWriter {
    /// Creates a new `StructWriter` for a STRUCT vector with `field_count` fields.
    ///
    /// This pre-creates a [`VectorWriter`] for each field index `0..field_count`.
    ///
    /// # Safety
    ///
    /// - `vector` must be a valid, writable `DuckDB` STRUCT vector.
    /// - `field_count` must match the number of fields in the STRUCT type.
    /// - The vector must remain valid for the lifetime of this writer.
    pub unsafe fn new(vector: duckdb_vector, field_count: usize) -> Self {
        let mut fields = Vec::with_capacity(field_count);
        for idx in 0..field_count {
            // SAFETY: caller guarantees vector is valid STRUCT with field_count fields.
            fields.push(unsafe { StructVector::field_writer(vector, idx) });
        }
        Self { fields }
    }

    /// Returns the number of fields in this struct writer.
    #[must_use]
    #[inline]
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns a mutable reference to the [`VectorWriter`] for the given field.
    ///
    /// # Panics
    ///
    /// Panics if `field_idx >= field_count`.
    #[must_use]
    #[inline]
    pub fn field_mut(&mut self, field_idx: usize) -> &mut VectorWriter {
        &mut self.fields[field_idx]
    }

    /// Writes a `bool` value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// - `row` must be within the vector's capacity.
    /// - The field at `field_idx` must have `BOOLEAN` type.
    ///
    /// # Panics
    ///
    /// Panics if `field_idx >= field_count`.
    #[inline]
    pub unsafe fn write_bool(&mut self, row: usize, field_idx: usize, value: bool) {
        // SAFETY: caller guarantees row is in bounds and field type is BOOLEAN.
        unsafe { self.fields[field_idx].write_bool(row, value) };
    }

    /// Writes a VARCHAR string value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// - `row` must be within the vector's capacity.
    /// - The field at `field_idx` must have `VARCHAR` type.
    ///
    /// # Panics
    ///
    /// Panics if `field_idx >= field_count`.
    #[inline]
    pub unsafe fn write_varchar(&mut self, row: usize, field_idx: usize, value: &str) {
        // SAFETY: caller guarantees row is in bounds and field type is VARCHAR.
        unsafe { self.fields[field_idx].write_varchar(row, value) };
    }

    /// Writes an `i8` (TINYINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// - `row` must be within the vector's capacity.
    /// - The field at `field_idx` must have `TINYINT` type.
    ///
    /// # Panics
    ///
    /// Panics if `field_idx >= field_count`.
    #[inline]
    pub unsafe fn write_i8(&mut self, row: usize, field_idx: usize, value: i8) {
        unsafe { self.fields[field_idx].write_i8(row, value) };
    }

    /// Writes an `i16` (SMALLINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_i16(&mut self, row: usize, field_idx: usize, value: i16) {
        unsafe { self.fields[field_idx].write_i16(row, value) };
    }

    /// Writes an `i32` (INTEGER) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_i32(&mut self, row: usize, field_idx: usize, value: i32) {
        unsafe { self.fields[field_idx].write_i32(row, value) };
    }

    /// Writes an `i64` (BIGINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_i64(&mut self, row: usize, field_idx: usize, value: i64) {
        unsafe { self.fields[field_idx].write_i64(row, value) };
    }

    /// Writes an `i128` (HUGEINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_i128(&mut self, row: usize, field_idx: usize, value: i128) {
        unsafe { self.fields[field_idx].write_i128(row, value) };
    }

    /// Writes a `u8` (UTINYINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_u8(&mut self, row: usize, field_idx: usize, value: u8) {
        unsafe { self.fields[field_idx].write_u8(row, value) };
    }

    /// Writes a `u16` (USMALLINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_u16(&mut self, row: usize, field_idx: usize, value: u16) {
        unsafe { self.fields[field_idx].write_u16(row, value) };
    }

    /// Writes a `u32` (UINTEGER) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_u32(&mut self, row: usize, field_idx: usize, value: u32) {
        unsafe { self.fields[field_idx].write_u32(row, value) };
    }

    /// Writes a `u64` (UBIGINT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_u64(&mut self, row: usize, field_idx: usize, value: u64) {
        unsafe { self.fields[field_idx].write_u64(row, value) };
    }

    /// Writes an `f32` (FLOAT) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_f32(&mut self, row: usize, field_idx: usize, value: f32) {
        unsafe { self.fields[field_idx].write_f32(row, value) };
    }

    /// Writes an `f64` (DOUBLE) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_f64(&mut self, row: usize, field_idx: usize, value: f64) {
        unsafe { self.fields[field_idx].write_f64(row, value) };
    }

    /// Writes an INTERVAL value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_interval(&mut self, row: usize, field_idx: usize, value: DuckInterval) {
        unsafe { self.fields[field_idx].write_interval(row, value) };
    }

    /// Writes a `BLOB` (binary) value to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_blob(&mut self, row: usize, field_idx: usize, value: &[u8]) {
        unsafe { self.fields[field_idx].write_blob(row, value) };
    }

    /// Writes a `UUID` value (as i128) to field `field_idx` at row `row`.
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_uuid(&mut self, row: usize, field_idx: usize, value: i128) {
        unsafe { self.fields[field_idx].write_uuid(row, value) };
    }

    /// Writes a VARCHAR string value to field `field_idx` at row `row`.
    ///
    /// Alias for [`write_varchar`][Self::write_varchar].
    ///
    /// # Safety
    ///
    /// See [`write_varchar`][Self::write_varchar].
    #[inline]
    pub unsafe fn write_str(&mut self, row: usize, field_idx: usize, value: &str) {
        unsafe { self.write_varchar(row, field_idx, value) };
    }

    /// Writes a `DATE` value (days since epoch) to field `field_idx` at row `row`.
    ///
    /// Semantic alias for [`write_i32`][Self::write_i32].
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_date(&mut self, row: usize, field_idx: usize, days_since_epoch: i32) {
        unsafe { self.write_i32(row, field_idx, days_since_epoch) };
    }

    /// Writes a `TIMESTAMP` value (microseconds since epoch) to field `field_idx` at row `row`.
    ///
    /// Semantic alias for [`write_i64`][Self::write_i64].
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_timestamp(
        &mut self,
        row: usize,
        field_idx: usize,
        micros_since_epoch: i64,
    ) {
        unsafe { self.write_i64(row, field_idx, micros_since_epoch) };
    }

    /// Writes a `TIME` value (microseconds since midnight) to field `field_idx` at row `row`.
    ///
    /// Semantic alias for [`write_i64`][Self::write_i64].
    ///
    /// # Safety
    ///
    /// See [`write_i8`][Self::write_i8].
    #[inline]
    pub unsafe fn write_time(&mut self, row: usize, field_idx: usize, micros_since_midnight: i64) {
        unsafe { self.write_i64(row, field_idx, micros_since_midnight) };
    }

    /// Marks field `field_idx` at row `row` as NULL.
    ///
    /// # Safety
    ///
    /// - `row` must be within the vector's capacity.
    ///
    /// # Panics
    ///
    /// Panics if `field_idx >= field_count`.
    #[inline]
    pub unsafe fn set_null(&mut self, row: usize, field_idx: usize) {
        // SAFETY: caller guarantees row is in bounds.
        unsafe { self.fields[field_idx].set_null(row) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_writer_field_count() {
        // We can't create a real StructWriter without DuckDB, but we can verify
        // the Vec-based field storage works correctly.
        let sw = StructWriter { fields: Vec::new() };
        assert_eq!(sw.field_count(), 0);
    }

    #[test]
    fn size_of_struct_writer() {
        // StructWriter is a Vec<VectorWriter> = 3 * usize (ptr, len, cap)
        assert_eq!(
            std::mem::size_of::<StructWriter>(),
            3 * std::mem::size_of::<usize>()
        );
    }
}
