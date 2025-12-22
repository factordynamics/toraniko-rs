//! Data transformation trait definitions.

use polars::prelude::*;

/// Errors that can occur during transformation.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    /// Empty input data.
    #[error("empty input data")]
    EmptyData,

    /// Invalid parameter.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Numerical error (NaN, Inf).
    #[error("numerical error: {0}")]
    Numerical(String),

    /// Polars error.
    #[error("polars error: {0}")]
    Polars(#[from] PolarsError),
}

/// Cross-sectional data transformation.
///
/// Operates on data partitioned by date, transforming values across assets.
pub trait CrossSectionTransform: Send + Sync {
    /// Transform target column, partitioned by group column.
    ///
    /// # Arguments
    /// * `target_col` - Column to transform
    /// * `group_col` - Column to partition by (typically "date")
    ///
    /// # Returns
    /// Polars expression representing the transformation.
    fn transform(&self, target_col: &str, group_col: &str) -> Expr;

    /// Returns the name of this transformation.
    fn name(&self) -> &str;
}

/// Time-series data transformation.
///
/// Operates on data sorted by time, transforming values within each asset.
pub trait TimeSeriesTransform: Send + Sync {
    /// Transform target column, sorted by time and partitioned by asset.
    ///
    /// # Arguments
    /// * `target_col` - Column to transform
    /// * `time_col` - Column to sort by (typically "date")
    /// * `partition_col` - Column to partition by (typically "symbol")
    ///
    /// # Returns
    /// Polars expression representing the transformation.
    fn transform(&self, target_col: &str, time_col: &str, partition_col: &str) -> Expr;

    /// Returns the name of this transformation.
    fn name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_error_display() {
        let err = TransformError::EmptyData;
        assert_eq!(err.to_string(), "empty input data");

        let err = TransformError::InvalidParameter("bad value".to_string());
        assert_eq!(err.to_string(), "invalid parameter: bad value");
    }
}
