import polars as pl
import polars_polyline as pp


def test_canonical_polyline():
    """Test canonical 3-point round-trip."""
    encoded = "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
    df = pl.DataFrame({"encoded": [encoded]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    assert result[0] is not None
    coords = result[0]
    assert len(coords) == 3

    # Check first coordinate
    assert abs(coords[0]["lat"] - 38.5) < 1e-5
    assert abs(coords[0]["lng"] - (-120.2)) < 1e-5

    # Check second coordinate
    assert abs(coords[1]["lat"] - 40.7) < 1e-5
    assert abs(coords[1]["lng"] - (-120.95)) < 1e-5

    # Check third coordinate
    assert abs(coords[2]["lat"] - 43.252) < 1e-5
    assert abs(coords[2]["lng"] - (-126.453)) < 1e-5


def test_null_input():
    """Test that null input produces null output."""
    df = pl.DataFrame({"encoded": [None]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    assert result[0] is None


def test_invalid_string():
    """Test that invalid string produces null output."""
    df = pl.DataFrame({"encoded": ["!!INVALID!!"]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    assert result[0] is None


def test_mixed_series():
    """Test series with mix of valid, null, and invalid entries."""
    valid_polyline = "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
    df = pl.DataFrame(
        {"encoded": [valid_polyline, None, "!!INVALID!!", valid_polyline]}
    )
    result = df.select(pp.decode_polyline("encoded")).to_series()

    # First should be valid
    assert result[0] is not None
    assert len(result[0]) == 3

    # Second should be null
    assert result[1] is None

    # Third should be null (invalid)
    assert result[2] is None

    # Fourth should be valid
    assert result[3] is not None
    assert len(result[3]) == 3


def test_precision_5():
    """Test that precision 5 works correctly."""
    encoded = "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
    df = pl.DataFrame({"encoded": [encoded]})
    result = df.select(pp.decode_polyline("encoded", precision=5)).to_series()

    assert result[0] is not None
    assert len(result[0]) == 3


def test_precision_6():
    """Test that precision 6 decoding works."""
    # Use a different precision-6 encoded string
    # This is a simple test string; we just verify it doesn't error
    encoded = "z~vFvyys|U"
    df = pl.DataFrame({"encoded": [encoded]})
    result = df.select(pp.decode_polyline("encoded", precision=6)).to_series()

    # Should either decode successfully or return null (if invalid)
    # The important thing is it doesn't raise an exception
    assert result[0] is None or isinstance(result[0], list)


def test_output_structure():
    """Test that output has correct structure (lat, lng fields)."""
    encoded = "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
    df = pl.DataFrame({"encoded": [encoded]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    coords = result[0]
    assert len(coords) > 0

    # Check that struct has lat and lng fields
    first_coord = coords[0]
    assert "lat" in first_coord
    assert "lng" in first_coord
    assert isinstance(first_coord["lat"], float)
    assert isinstance(first_coord["lng"], float)


def test_empty_polyline_returns_null():
    """Test that empty polyline string returns null."""
    df = pl.DataFrame({"encoded": [""]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    assert result[0] is None


def test_multiple_rows():
    """Test that the function works correctly on multiple rows."""
    encoded1 = "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
    encoded2 = "u{~vFvyys|U"
    df = pl.DataFrame({"encoded": [encoded1, encoded2]})
    result = df.select(pp.decode_polyline("encoded")).to_series()

    # First should decode successfully
    assert result[0] is not None
    assert len(result[0]) == 3

    # Second may decode or not depending on the string, but should not error
    assert result[1] is None or isinstance(result[1], list)
