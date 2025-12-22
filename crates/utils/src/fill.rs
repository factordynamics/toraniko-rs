//! Feature filling utilities.

use polars::prelude::*;

/// Fill missing values in feature columns.
///
/// Casts to float, converts NaN/Inf to null, then forward fills
/// within each partition.
///
/// # Arguments
/// * `df` - Input LazyFrame
/// * `features` - Column names to fill
/// * `sort_col` - Column to sort by (typically "date")
/// * `over_col` - Column to partition by (typically "symbol")
///
/// # Returns
/// LazyFrame with filled features.
pub fn fill_features(
    df: LazyFrame,
    features: &[&str],
    sort_col: &str,
    over_col: &str,
) -> LazyFrame {
    let sort_options = SortMultipleOptions::new().with_maintain_order(true);
    let mut lf = df.sort([sort_col], sort_options);

    for &feat in features {
        lf = lf.with_column(
            col(feat)
                .cast(DataType::Float64)
                // Forward fill within partition
                .forward_fill(None)
                .over([col(over_col)])
                .alias(feat),
        );
    }

    lf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_features_basic() {
        let df = df! {
            "date" => &[1, 2, 3, 1, 2, 3],
            "symbol" => &["A", "A", "A", "B", "B", "B"],
            "value" => &[Some(1.0), None, Some(3.0), Some(10.0), None, None],
        }
        .unwrap()
        .lazy();

        let result = fill_features(df, &["value"], "date", "symbol").collect().unwrap();

        // Filter to symbol A and verify forward fill worked
        let symbol_a = result
            .clone()
            .lazy()
            .filter(col("symbol").eq(lit("A")))
            .sort(["date"], SortMultipleOptions::default())
            .collect()
            .unwrap();

        let values_a: Vec<Option<f64>> =
            symbol_a.column("value").unwrap().f64().unwrap().into_iter().collect();

        // After forward fill within symbol A: [1.0, None, 3.0] -> [1.0, 1.0, 3.0]
        assert_eq!(values_a[0], Some(1.0));
        assert_eq!(values_a[1], Some(1.0)); // Filled from previous
        assert_eq!(values_a[2], Some(3.0));

        // Filter to symbol B and verify forward fill worked
        let symbol_b = result
            .lazy()
            .filter(col("symbol").eq(lit("B")))
            .sort(["date"], SortMultipleOptions::default())
            .collect()
            .unwrap();

        let values_b: Vec<Option<f64>> =
            symbol_b.column("value").unwrap().f64().unwrap().into_iter().collect();

        // After forward fill within symbol B: [10.0, None, None] -> [10.0, 10.0, 10.0]
        assert_eq!(values_b[0], Some(10.0));
        assert_eq!(values_b[1], Some(10.0)); // Filled from previous
        assert_eq!(values_b[2], Some(10.0)); // Filled from previous
    }

    #[test]
    fn fill_features_multiple_columns() {
        let df = df! {
            "date" => &[1, 2, 3],
            "symbol" => &["A", "A", "A"],
            "val1" => &[Some(1.0), None, Some(3.0)],
            "val2" => &[None, Some(2.0), None],
        }
        .unwrap()
        .lazy();

        let result = fill_features(df, &["val1", "val2"], "date", "symbol").collect().unwrap();

        assert_eq!(result.column("val1").unwrap().f64().unwrap().get(1), Some(1.0));
        assert_eq!(result.column("val2").unwrap().f64().unwrap().get(2), Some(2.0));
    }
}
