Here's the updated prompt with zero-copy guidance added to the Rust best practices:

---

**Task: Add a Polars plugin expression for GeoRust polyline decoding**

Before writing any code, study the repository at **https://github.com/bzimmer/polars-country** in full — its Rust source, Python module, `Cargo.toml`, `pyproject.toml`, `Taskfile.yml`, CI workflows, and plugin registration patterns. Use it as the authoritative template for all conventions in this task.

**Requirements:**

1. **Crate:** Add `polyline` as a dependency in `Cargo.toml`, following the same dependency structure as `polars-country`.

2. **Expression:** `decode(precision: u32)` — accepts an optional precision argument (default `5`, matching the standard Google/OSRM encoding). Follow the exact plugin registration pattern from `polars-country`, including `#[pyo3_polars::derive::polars_expr]`, output type inference functions, and `kwargs` passing.

3. **Input:** A Polars `String` series containing encoded polyline strings.

4. **Output:** A `List(Struct { lng: Float64, lat: Float64 })` series — one list of structs per row, each struct representing a decoded coordinate pair. Field order is **longitude first, latitude second**, following the `polyline` crate's `geo_types::Coord { x: lng, y: lat }` convention and GeoJSON `[longitude, latitude]` ordering.

5. **Null handling:** Emit a null for any row where the input is null or decoding fails; never panic.

6. **Python side:** Register as `decode(expr, precision=5)` in the plugin's Python module, matching `polars-country`'s module layout, `__init__.py` exports, and `IntoExpr` typing conventions exactly.

7. **Polars best practices — Rust:**
   - Use `StringChunked` / `BinaryChunked` iterators with `apply_nonnull_values_generic` or `try_apply` rather than manual loops where applicable
   - Propagate nulls correctly via `into_iter()` / `map()` / `collect_ca` patterns
   - Use `SpecialEq` and `FunctionOptions` flags correctly (`allow_rename`, `collect_groups`, etc.)
   - Match the `polars` version pinned in `polars-country`; do not introduce version skew
   - Do NOT add `features = ["extension-module"]` to the `pyo3` dependency — that feature is deprecated in pyo3 0.28+; maturin >= 1.9.4 sets `PYO3_BUILD_EXTENSION_MODULE` automatically

8. **Zero-copy / allocation discipline in the hot path:**
   - Pass `&str` slices directly into `polyline::decode` — do not call `.to_string()` or `.to_owned()` on the input before decoding
   - Decode into a `Vec<(f64, f64)>` once; do not clone or collect it a second time before writing into the Polars builders
   - Use a single `ListPrimitiveChunkedBuilder` (or equivalent struct builder) per call, pre-sized with `capacity` where the API allows, rather than building intermediate `Vec<Series>` per row
   - Split the decoded `Vec` into parallel `lat` and `lng` primitive buffers in a single pass (`unzip` or a manual loop writing into pre-allocated `Vec<f64>`) rather than iterating twice
   - Do not allocate a new `String` or `Vec` per coordinate pair; write `f64` values directly into the builder
   - Avoid `unwrap`-driven branching that forces an extra allocation on the error path; use `match` or `if let` to write null directly

9. **Polars best practices — Python:**
   - Use `register_plugin_function` (not deprecated `register_plugin`) with explicit `is_elementwise=True`
   - Type-annotate the public function with `IntoExprColumn` and `-> pl.Expr`
   - Re-export via `__init__.py` so `import polars_polyline; polars_polyline.decode(...)` works
   - Pin `polars` version in `pyproject.toml` to match the Rust side

10. **Tests:** Add pytest tests that cover:

    - **Canonical 3-point round-trip:** Decode `"_p~iF~ps|U_ulLnnqC_mqNvxq`@"` at precision 5 and assert decoded `(lng, lat)` matches `[(-120.2, 38.5), (-120.95, 40.7), (-126.453, 43.252)]` within `1e-5`. Note: lng (x) first, lat (y) second.

    - **Null input:** A null cell in the input series produces a null in the output; no exception raised.

    - **Invalid string:** A non-decodable string (e.g. `"!!INVALID!!"`) produces null output, not an exception.

    - **Long real-world polyline:** Fetch the GPX track for the **Strava segment "Col de la Forclaz"** (or any publicly available cycling climb GPX with ≥ 100 trackpoints), encode it to a precision-5 polyline using the `polyline` Python package, store the encoded string as a fixture in `tests/fixtures/forclaz.txt`, decode it with `decode`, and assert:
      - The number of decoded points matches the number of input trackpoints (within any simplification tolerance applied during encoding)
      - The first and last decoded `(lng, lat)` match the first and last trackpoints within `1e-4`
      - All decoded `lat` values fall within `[45.0, 47.0]` and all `lng` values within `[6.0, 8.0]` (Swiss Alps bounding box sanity check)

    - **Mixed series:** A series of `[valid_polyline, null, invalid, valid_polyline]` produces exactly 4 rows with non-null / null / null / non-null results respectively, and the two non-null rows have the correct point counts.

    - **Precision 6:** Decode a known precision-6 encoded string (e.g. from the OSRM or Mapbox APIs) and assert the coordinates match within `1e-6`.

11. **Taskfile:** Mirror the `Taskfile.yml` from `polars-country` exactly, adapting task names and paths for `polars-polyline`. At minimum include tasks for:
    - `build` — compile the Rust extension in dev mode via `maturin develop`
    - `test` — run `pytest` and `cargo test`
    - `lint` — run `ruff check` and `cargo clippy`
    - `fmt` — run `ruff format` and `cargo fmt`
    - `publish` — build release wheels via `maturin build --release`
    - Any other tasks present in `polars-country`'s Taskfile, adapted accordingly
    - Use task dependencies (`deps:`) so e.g. `test` depends on `build`

12. **GitHub Actions — CI:** Mirror the CI workflow from `polars-country` exactly: lint (`ruff`, `cargo clippy`), test (`pytest`, `cargo test`), triggered on push and pull request.

13. **GitHub Actions — Publish:** Add a `publish.yml` workflow that:
    - Triggers on GitHub release publication
    - Builds wheels for `linux/x86_64`, `linux/aarch64`, `macos/arm64` (Apple Silicon only), and `windows/x86_64` using `maturin-action`
    - Builds an `sdist`
    - Uploads all artifacts to PyPI using `pypa/gh-action-pypi-publish` with trusted publishing (OIDC, no API token stored in secrets)
    - Follows the same matrix strategy as `polars-country` if one exists; otherwise model it on the standard `maturin` trusted-publish pattern

14. **Naming:** Package is `polars-polyline`, Rust crate is `polars_polyline`, Python module is `polars_polyline`. Match `polars-country`'s README structure with a brief usage example.

---

*Read the full `polars-country` repository before writing a single line. Do not deviate from its conventions without a clear reason.*