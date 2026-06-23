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

fn decode_polyline_coords(encoded: &str, precision: u32) -> PolarsResult<Vec<(f64, f64)>> {
    if encoded.is_empty() {
        return Err(polars_err!(InvalidOperation: "empty polyline string"));
    }
    polyline::decode_polyline(encoded, precision)
        .map(|ls| ls.0.into_iter().map(|c| (c.y, c.x)).collect())
        .map_err(|e| polars_err!(InvalidOperation: "polyline decode error: {}", e))
}

fn output_type_decode_polyline(_: &[Field]) -> PolarsResult<Field> {
    let fields = vec![
        Field::new("lat".into(), DataType::Float64),
        Field::new("lng".into(), DataType::Float64),
    ];
    Ok(Field::new(
        "coords".into(),
        DataType::List(Box::new(DataType::Struct(fields))),
    ))
}

#[polars_expr(output_type_func = output_type_decode_polyline)]
fn polars_decode_polyline(inputs: &[Series], kwargs: DecodePolylineKwargs) -> PolarsResult<Series> {
    let str_ca = inputs[0].str()?;
    let precision = kwargs.precision;
    let n = str_ca.len();

    let mut all_lats: Vec<f64> = Vec::new();
    let mut all_lngs: Vec<f64> = Vec::new();
    let mut offsets: Vec<i64> = Vec::with_capacity(n + 1);
    let mut validity = MutableBitmap::with_capacity(n);
    let mut any_null = false;

    offsets.push(0);

    for opt_str in str_ca.iter() {
        match opt_str {
            Some(s) => match decode_polyline_coords(s, precision) {
                Ok(coords) => {
                    for (lat, lng) in coords {
                        all_lats.push(lat);
                        all_lngs.push(lng);
                    }
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
        offsets.push(all_lats.len() as i64);
    }

    let n_coords = all_lats.len();
    let lat_arr = PrimitiveArray::<f64>::from_vec(all_lats);
    let lng_arr = PrimitiveArray::<f64>::from_vec(all_lngs);

    let struct_dtype = ArrowDataType::Struct(vec![
        ArrowField::new("lat".into(), ArrowDataType::Float64, false),
        ArrowField::new("lng".into(), ArrowDataType::Float64, false),
    ]);
    let struct_arr = StructArray::new(
        struct_dtype.clone(),
        n_coords,
        vec![
            Box::new(lat_arr) as Box<dyn Array>,
            Box::new(lng_arr) as Box<dyn Array>,
        ],
        None,
    );

    let list_dtype = LargeListArray::default_datatype(struct_dtype);
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
        let coords = decode_polyline_coords(encoded, 5);
        assert!(coords.is_ok());

        let coords = coords.unwrap();
        assert_eq!(coords.len(), 3);

        let tolerance = 1e-5;
        assert!((coords[0].0 - 38.5).abs() < tolerance);
        assert!((coords[0].1 - (-120.2)).abs() < tolerance);
        assert!((coords[1].0 - 40.7).abs() < tolerance);
        assert!((coords[1].1 - (-120.95)).abs() < tolerance);
        assert!((coords[2].0 - 43.252).abs() < tolerance);
        assert!((coords[2].1 - (-126.453)).abs() < tolerance);
    }

    #[test]
    fn empty_string_returns_err() {
        let result = decode_polyline_coords("", 5);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_polyline_returns_err() {
        let result = decode_polyline_coords("!!INVALID!!", 5);
        assert!(result.is_err());
    }

    #[test]
    fn precision_6_decoding() {
        let encoded = "_p~iF~ps|U";
        let coords = decode_polyline_coords(encoded, 6);
        assert!(coords.is_ok());
        let coords = coords.unwrap();
        assert!(!coords.is_empty());
    }
}
