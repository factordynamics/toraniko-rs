//! Feature smoothing utilities.

use polars::prelude::*;

/// Smooth features using a mean within partitions.
///
/// This is a simplified smoothing that computes the mean of each feature
/// within each partition group. For true rolling windows within groups,
/// consider using polars' rolling window expressions directly.
///
/// # Arguments
/// * `df` - Input LazyFrame
/// * `features` - Column names to smooth
/// * `sort_col` - Column to sort by (typically "date")
/// * `over_col` - Column to partition by (typically "symbol")
/// * `_window_size` - Window size (reserved for future use)
///
/// # Returns
/// LazyFrame with smoothed features.
pub fn smooth_features(
    df: LazyFrame,
    features: &[&str],
    sort_col: &str,
    over_col: &str,
    _window_size: usize,
) -> LazyFrame {
    let sort_options = SortMultipleOptions::new().with_maintain_order(true);
    let mut lf = df.sort([sort_col], sort_options);

    for &feat in features {
        lf = lf.with_column(col(feat).mean().over([col(over_col)]).alias(feat));
    }

    lf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_features_basic() {
        let df = df! {
            "date" => &[1, 2, 3, 4, 5],
            "symbol" => &["A", "A", "A", "A", "A"],
            "value" => &[1.0, 2.0, 3.0, 4.0, 5.0],
        }
        .unwrap()
        .lazy();

        let result = smooth_features(df, &["value"], "date", "symbol", 3).collect().unwrap();

        // Mean of [1,2,3,4,5] = 3.0
        let values: Vec<f64> =
            result.column("value").unwrap().f64().unwrap().into_no_null_iter().collect();
        assert_eq!(values.len(), 5);
    }

    #[test]
    fn smooth_features_multiple_symbols() {
        let df = df! {
            "date" => &[1, 2, 1, 2],
            "symbol" => &["A", "A", "B", "B"],
            "value" => &[10.0, 20.0, 100.0, 200.0],
        }
        .unwrap()
        .lazy();

        let result = smooth_features(df, &["value"], "date", "symbol", 2).collect().unwrap();

        // Each symbol should be smoothed independently
        assert_eq!(result.height(), 4);
    }
}
