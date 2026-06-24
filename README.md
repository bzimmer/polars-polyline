# polars-polyline

Fast polyline decoding for Polars, powered by Rust and the [`polyline`](https://crates.io/crates/polyline) crate.

Decode polyline-encoded strings into lists of `{lng, lat}` coordinate structs directly within Polars queries. Coordinate order follows the [`polyline`](https://crates.io/crates/polyline) crate convention — **longitude first, latitude second** — matching `geo_types::Coord { x: lng, y: lat }` and GeoJSON. Implemented as a native Polars expression plugin with zero-copy coordinate passing.

## Installation

Install from PyPI:

```bash
pip install polars-polyline
```

Or build from source:

```bash
git clone https://github.com/bzimmer/polars-polyline
cd polars-polyline
task build
```

## Usage

```python
import polars as pl
import polars_polyline as pp

# Decode a polyline-encoded string
df = pl.DataFrame({
    "encoded": ["_p~iF~ps|U_ulLnnqC_mqNvxq`@"]
})

result = df.with_columns(
    coords=pp.decode("encoded")
)

print(result)
```

Output:
```
shape: (1, 2)
┌──────────────────────────┬──────────────────────────────────┐
│ encoded                  ┆ coords                           │
│ ---                      ┆ ---                              │
│ str                      ┆ list[struct[2]]                  │
╞══════════════════════════╪══════════════════════════════════╡
│ _p~iF~ps|U_ulLnnqC_mqNv… ┆ [{-120.2, 38.5}, {-120.95, …     │
└──────────────────────────┴──────────────────────────────────┘
```

### Specifying Precision

The Google polyline algorithm uses a precision parameter. The default is 5 (standard for Google Maps and OSRM). Use the `precision` parameter to decode strings encoded with a different precision:

```python
# Decode precision-6 encoded polyline
result = df.with_columns(
    coords=pp.decode("encoded", precision=6)
)
```

### Handling Nulls and Errors

Null inputs and invalid polyline strings produce null output — no exceptions are raised:

```python
df = pl.DataFrame({
    "encoded": [
        "_p~iF~ps|U_ulLnnqC_mqNvxq`@",  # Valid
        None,                             # Null -> produces null
        "!!INVALID!!",                    # Invalid -> produces null
    ]
})

result = df.with_columns(
    coords=pp.decode("encoded")
)
```

### Working with Decoded Coordinates

Decoded coordinates are returned as `List(Struct { lng: Float64, lat: Float64 })` — **longitude first, latitude second**. Access fields by name using Polars struct operations:

```python
result = df.with_columns(
    coords=pp.decode("encoded")
).with_columns(
    num_points=pl.col("coords").list.len(),
    first_lng=pl.col("coords").list.first()["lng"],
    first_lat=pl.col("coords").list.first()["lat"],
)
```

## Development

### Setup

Clone the repository and install development dependencies:

```bash
git clone https://github.com/bzimmer/polars-polyline
cd polars-polyline
task build
```

### Testing

Run the full test suite (Rust unit tests + Python pytest):

```bash
task test
```

### Code Quality

Format and lint code:

```bash
task fmt      # Format code
task lint     # Run linters
```

### Building Wheels

Build release wheels for all supported platforms:

```bash
task release
```

Wheels are created in `target/wheels/`.

## Performance

- **Zero-copy**: Polyline strings are passed directly to Rust; no intermediate Python allocations.
- **Native Rust**: Uses the [`polyline`](https://crates.io/crates/polyline) crate for fast, correct decoding.
- **Expression plugin**: Integrates seamlessly with Polars' lazy evaluation and optimization.

## Requirements

- Python ≥ 3.14
- Polars ≥ 1.0

## License

Licensed under the same terms as the `polyline` crate and Polars.
