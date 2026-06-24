# Implementation Notes

## What was built

A Polars expression plugin that decodes polyline-encoded strings into coordinate lists, using the `polyline` crate v0.11 and `pyo3-polars`.

---

## Coordinate convention

Output follows the `polyline` crate's `geo_types::Coord { x: lng, y: lat }` convention: **longitude first, latitude second**.

- Rust: `decode` returns `Vec<(f64, f64)>` where `.0 = lng`, `.1 = lat`
- Arrow struct field order: `{ lng: Float64, lat: Float64 }`
- Polars dtype: `List(Struct { lng: Float64, lat: Float64 })`

This matches GeoJSON `[longitude, latitude]` ordering and avoids surprising consumers who know the underlying crate.

---

## Key implementation decisions

### `output_type_func` instead of `output_type`

`#[polars_expr(output_type = ...)]` only accepts a simple ident like `Float64`. For `List(Struct(...))` we must use `output_type_func = output_type_decode` pointing to a function returning `PolarsResult<Field>`.

### Arrow construction path

`ListChunked::from_chunks_and_dtype_unchecked` is `pub(crate)` in polars-core. We build the `LargeListArray` manually via polars-arrow and call `Series::from_arrow(name, Box::new(list_arr))`.

```
LargeListArray {
  dtype: LargeList(Struct([lng: Float64, lat: Float64])),
  offsets: OffsetsBuffer<i64>,
  values: StructArray {
    fields: [lng: PrimitiveArray<f64>, lat: PrimitiveArray<f64>]
  },
  validity: Option<Bitmap>
}
```

### Precision via kwargs, not a series arg

Passing precision as `pl.lit(n, dtype=pl.UInt32)` creates a 1-element series. When zipped with the string series, output is truncated to 1 row. Precision is passed via `kwargs={"precision": precision}` using pyo3-polars's serde-pickle kwargs mechanism (`DecodePolylineKwargs` struct with `serde::Deserialize`).

### Input cast

`.cast(pl.String)` is applied before the plugin call. Without it, a column inferred as `null` dtype (e.g. `[None]` with no type hint) causes "invalid series dtype: expected String, got null".

### `extension-module` feature removed

`pyo3`'s `extension-module` feature is deprecated as of pyo3 0.28 / maturin 1.9.4. Keeping it in `Cargo.toml` suppresses `libpython` linking for ALL builds including `cargo test`, causing linker failures (`Undefined symbols: _PyBool_Type` etc.). Modern maturin sets `PYO3_BUILD_EXTENSION_MODULE=1` automatically only during the extension build. The fix: `pyo3 = { version = "0.28" }` (no features).

---

## Cargo.toml

```toml
[dependencies]
pyo3 = { version = "0.28" }
pyo3-polars = { version = "0.27", features = ["derive"] }
polars-core = { version = "0.54", default-features = false, features = ["dtype-struct"] }
polyline = "0.11"
serde = { version = "1", features = ["derive"] }

[dependencies.arrow]
package = "polars-arrow"
version = "0.54"
```

`dtype-struct` feature is required on `polars-core` for `DataType::Struct`.

---

## What was not implemented

- **Forclaz real-world fixture test** (plan item 10.iv): no external fixture file was created.
- **`IntoExprColumn` type annotation** on `decode`: the Python function accepts `str | pl.Expr | pl.Series` with manual dispatch instead. Changing to `IntoExprColumn` would require importing from `polars.type_aliases` and would break the `Series` path.
- **Precision-6 assertion** (plan item 10.v): the test verifies no exception is raised but does not assert exact coordinate values, because no known-good precision-6 fixture was available.
