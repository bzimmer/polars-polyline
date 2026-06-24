use std::sync::OnceLock;

use arrow::array::{Array, PrimitiveArray, StructArray};
use arrow::bitmap::{Bitmap, MutableBitmap};
use arrow::datatypes::{ArrowDataType, Field as ArrowField};
use arrow::offset::OffsetsBuffer;
use polars_core::prelude::*;
use pyo3::prelude::*;
use pyo3_polars::derive::polars_expr;
use serde::Deserialize;

#[derive(Deserialize)]
struct DecodePolylineKwargs {
    precision: u32,
}

static STRUCT_DTYPE: OnceLock<ArrowDataType> = OnceLock::new();

fn struct_dtype() -> &'static ArrowDataType {
    STRUCT_DTYPE.get_or_init(|| {
        ArrowDataType::Struct(vec![
            ArrowField::new("lng".into(), ArrowDataType::Float64, false),
            ArrowField::new("lat".into(), ArrowDataType::Float64, false),
        ])
    })
}

// Pushes decoded coords (lng=x, lat=y) into the caller's buffers; returns the coord count.
fn decode_into(
    encoded: &str,
    precision: u32,
    lngs: &mut Vec<f64>,
    lats: &mut Vec<f64>,
) -> PolarsResult<usize> {
    if encoded.is_empty() {
        return Err(polars_err!(InvalidOperation: "empty polyline string"));
    }
    let ls = polyline::decode_polyline(encoded, precision)
        .map_err(|e| polars_err!(InvalidOperation: "polyline decode error: {}", e))?;
    let n = ls.0.len();
    for c in ls.0 {
        lngs.push(c.x);
        lats.push(c.y);
    }
    Ok(n)
}

fn output_type_decode(_: &[Field]) -> PolarsResult<Field> {
    let fields = vec![
        Field::new("lng".into(), DataType::Float64),
        Field::new("lat".into(), DataType::Float64),
    ];
    Ok(Field::new(
        "coords".into(),
        DataType::List(Box::new(DataType::Struct(fields))),
    ))
}

#[polars_expr(output_type_func = output_type_decode)]
fn polars_decode(inputs: &[Series], kwargs: DecodePolylineKwargs) -> PolarsResult<Series> {
    let str_ca = inputs[0].str()?;
    let precision = kwargs.precision;
    let n = str_ca.len();

    let mut all_lngs: Vec<f64> = Vec::with_capacity(n);
    let mut all_lats: Vec<f64> = Vec::with_capacity(n);
    let mut offsets: Vec<i64> = Vec::with_capacity(n + 1);
    let mut validity = MutableBitmap::with_capacity(n);
    let mut any_null = false;

    offsets.push(0);

    for opt_str in str_ca.iter() {
        match opt_str {
            Some(s) => match decode_into(s, precision, &mut all_lngs, &mut all_lats) {
                Ok(_) => {
                    validity.push(true);
                }
                Err(_) => {
                    validity.push(false);
                    any_null = true;
                }
            },
            None => {
                validity.push(false);
                any_null = true;
            }
        }
        offsets.push(all_lngs.len() as i64);
    }

    let n_coords = all_lngs.len();
    let lng_arr = PrimitiveArray::<f64>::from_vec(all_lngs);
    let lat_arr = PrimitiveArray::<f64>::from_vec(all_lats);

    let struct_arr = StructArray::new(
        struct_dtype().clone(),
        n_coords,
        vec![
            Box::new(lng_arr) as Box<dyn Array>,
            Box::new(lat_arr) as Box<dyn Array>,
        ],
        None,
    );

    let list_dtype = LargeListArray::default_datatype(struct_dtype().clone());
    let offsets_buf = OffsetsBuffer::<i64>::try_from(offsets)
        .map_err(|e| polars_err!(InvalidOperation: "invalid offsets: {}", e))?;
    let row_validity: Option<Bitmap> = if any_null {
        Some(validity.into())
    } else {
        None
    };

    let list_arr = LargeListArray::new(
        list_dtype,
        offsets_buf,
        Box::new(struct_arr) as Box<dyn Array>,
        row_validity,
    );

    let name = inputs[0].name().clone();
    Series::from_arrow(name, Box::new(list_arr) as ArrayRef)
}

#[pymodule]
fn _polars_polyline(_m: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_polyline_decoding() {
        let encoded = "_p~iF~ps|U_ulLnnqC_mqNvxq`@";
        let mut lngs = Vec::new();
        let mut lats = Vec::new();
        let n = decode_into(encoded, 5, &mut lngs, &mut lats);
        assert!(n.is_ok());

        let n = n.unwrap();
        assert_eq!(n, 3);

        // Tuples are (lng, lat) — longitude first, latitude second.
        let tolerance = 1e-5;
        assert!((lngs[0] - (-120.2)).abs() < tolerance);
        assert!((lats[0] - 38.5).abs() < tolerance);
        assert!((lngs[1] - (-120.95)).abs() < tolerance);
        assert!((lats[1] - 40.7).abs() < tolerance);
        assert!((lngs[2] - (-126.453)).abs() < tolerance);
        assert!((lats[2] - 43.252).abs() < tolerance);
    }

    #[test]
    fn empty_string_returns_err() {
        let mut lngs = Vec::new();
        let mut lats = Vec::new();
        let result = decode_into("", 5, &mut lngs, &mut lats);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_polyline_returns_err() {
        let mut lngs = Vec::new();
        let mut lats = Vec::new();
        let result = decode_into("!!INVALID!!", 5, &mut lngs, &mut lats);
        assert!(result.is_err());
    }

    #[test]
    fn precision_6_decoding() {
        let encoded = "_p~iF~ps|U";
        let mut lngs = Vec::new();
        let mut lats = Vec::new();
        let result = decode_into(encoded, 6, &mut lngs, &mut lats);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }
}
