"""
Polars expression helper for polyline decoding.

Wraps the ``_polars_polyline`` native extension and exposes a ``decode_polyline``
expression that plugs into any Polars query. Decoded coordinates follow the
``polyline`` crate convention: longitude first, latitude second
(``Struct { lng: Float64, lat: Float64 }``).

    import polars as pl
    import polars_polyline as pp

    df = pl.DataFrame({"encoded": ["_p~iF~ps|U_ulLnnqC_mqNvxq`@"]})
    df.with_columns(
        coords=pp.decode_polyline("encoded")
    )
"""

from pathlib import Path

import polars as pl
from polars.plugins import register_plugin_function

__all__ = ["decode_polyline"]

_LIB = Path(__file__).parent


def decode_polyline(expr: str | pl.Expr | pl.Series, precision: int = 5) -> pl.Expr:
    """Return a Polars expression decoding polyline strings to coordinate lists.

    Decodes polyline-encoded strings into lists of ``{lng, lat}`` struct pairs.
    Coordinate order follows the ``polyline`` crate convention: longitude first,
    latitude second — matching ``geo_types::Coord { x: lng, y: lat }`` and GeoJSON.
    Implemented as a native Polars expression plugin: Arrow buffers are passed
    directly to Rust with no Python-level materialisation.

    Null or invalid polyline values in the input produce null output.

    Parameters
    ----------
    expr:
        A column name, ``pl.Expr``, or ``pl.Series`` containing polyline-encoded strings.
    precision:
        The precision used in the polyline encoding. Defaults to 5 (standard Google/OSRM).

    Returns
    -------
    pl.Expr
        A lazy expression of dtype ``List(Struct { lng: Float64, lat: Float64 })``.
        Fields are ordered ``lng`` then ``lat`` (longitude first).

    Examples
    --------
    >>> import polars as pl
    >>> import polars_polyline as pp
    >>> df = pl.DataFrame(
    ...     {"encoded": ["_p~iF~ps|U_ulLnnqC_mqNvxq`@"]}
    ... )
    >>> df.with_columns(coords=pp.decode_polyline("encoded"))
    shape: (1, 2)
    ┌──────────────────────────┬──────────────────────────────────────┐
    │ encoded                  ┆ coords                               │
    │ ---                      ┆ ---                                  │
    │ str                      ┆ list[struct[2]]                      │
    ╞══════════════════════════╪══════════════════════════════════════╡
    │ _p~iF~ps|U_ulLnnqC_mqNv… ┆ [struct({-120.2, 38.5}),...] │
    └──────────────────────────┴──────────────────────────────────────┘
    """
    if isinstance(expr, str):
        expr_polyline = pl.col(expr)
    elif isinstance(expr, pl.Series):
        expr_polyline = pl.lit(expr)
    elif isinstance(expr, pl.Expr):
        expr_polyline = expr
    else:
        raise TypeError(f"expr must be str, pl.Expr, or pl.Series, got {type(expr)}")

    return register_plugin_function(
        plugin_path=_LIB,
        function_name="polars_decode_polyline",
        args=[expr_polyline.cast(pl.String)],
        kwargs={"precision": precision},
        is_elementwise=True,
    )
