//! Winsorization operations for outlier handling.

use ndarray::Array1;
use polars::prelude::*;

use crate::MathError;

/// Winsorize a 1D array to symmetric percentiles.
///
/// Values below the lower percentile are clipped to that value.
/// Values above the upper percentile are clipped to that value.
///
/// # Arguments
/// * `data` - Input array
/// * `percentile` - Percentile threshold (e.g., 0.05 for 5th/95th percentiles)
///
/// # Returns
/// Winsorized array.
///
/// # Errors
/// Returns `MathError::InvalidPercentile` if percentile is not in (0, 0.5).
pub fn winsorize(data: &Array1<f64>, percentile: f64) -> Result<Array1<f64>, MathError> {
    if percentile <= 0.0 || percentile >= 0.5 {
        return Err(MathError::InvalidPercentile(percentile));
    }

    if data.is_empty() {
        return Ok(data.clone());
    }

    // Filter out NaN values for sorting
    let mut valid_values: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();

    if valid_values.is_empty() {
        return Ok(data.clone());
    }

    valid_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = valid_values.len();
    let lower_idx = ((n as f64) * percentile).floor() as usize;
    let upper_idx = ((n as f64) * (1.0 - percentile)).ceil() as usize;

    let lower_bound = valid_values[lower_idx];
    let upper_bound = valid_values[upper_idx.saturating_sub(1).min(n - 1)];

    Ok(data.mapv(|x| if x.is_nan() { x } else { x.clamp(lower_bound, upper_bound) }))
}

/// Winsorize columns of a LazyFrame cross-sectionally by group.
///
/// # Arguments
/// * `df` - Input LazyFrame
/// * `data_cols` - Columns to winsorize
/// * `group_col` - Column to group by (typically "date")
/// * `percentile` - Percentile threshold
///
/// # Returns
/// LazyFrame with winsorized columns.
pub fn winsorize_xsection(
    df: LazyFrame,
    data_cols: &[&str],
    group_col: &str,
    percentile: f64,
) -> LazyFrame {
    let lower_q = percentile;
    let upper_q = 1.0 - percentile;

    let mut lf = df;

    for &col_name in data_cols {
        let lower_bound =
            col(col_name).quantile(lit(lower_q), QuantileMethod::Linear).over([col(group_col)]);
        let upper_bound =
            col(col_name).quantile(lit(upper_q), QuantileMethod::Linear).over([col(group_col)]);

        lf = lf.with_column(
            when(col(col_name).lt(lower_bound.clone()))
                .then(lower_bound)
                .when(col(col_name).gt(upper_bound.clone()))
                .then(upper_bound)
                .otherwise(col(col_name))
                .alias(col_name),
        );
    }

    lf
}

/// Winsorization configuration and transform.
#[derive(Debug, Clone)]
pub struct Winsorizer {
    /// Percentile threshold (e.g., 0.05).
    percentile: f64,
}

impl Winsorizer {
    /// Create a new winsorizer.
    ///
    /// # Arguments
    /// * `percentile` - Must be in (0, 0.5)
    ///
    /// # Errors
    /// Returns `MathError::InvalidPercentile` if percentile is not in valid range.
    pub fn new(percentile: f64) -> Result<Self, MathError> {
        if percentile <= 0.0 || percentile >= 0.5 {
            return Err(MathError::InvalidPercentile(percentile));
        }
        Ok(Self { percentile })
    }

    /// Get the percentile.
    #[must_use]
    pub const fn percentile(&self) -> f64 {
        self.percentile
    }

    /// Apply winsorization to an array.
    ///
    /// # Errors
    /// Returns error if winsorization fails.
    pub fn apply(&self, data: &Array1<f64>) -> Result<Array1<f64>, MathError> {
        winsorize(data, self.percentile)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use ndarray::array;
    use rstest::rstest;

    use super::*;

    #[test]
    fn winsorize_clips_extremes() {
        let data = array![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0];
        let result = winsorize(&data, 0.1).unwrap();

        // 100.0 should be clipped down
        assert!(result[9] < 100.0);
        // 1.0 should be preserved or clipped up
        assert!(result[0] >= 1.0);
    }

    #[test]
    fn winsorize_preserves_middle() {
        let data = array![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = winsorize(&data, 0.1).unwrap();

        // Middle values should be unchanged
        assert_relative_eq!(result[4], 5.0, epsilon = 1e-10);
        assert_relative_eq!(result[5], 6.0, epsilon = 1e-10);
    }

    #[rstest]
    #[case(0.0)]
    #[case(0.5)]
    #[case(0.6)]
    #[case(-0.1)]
    fn invalid_percentile_errors(#[case] pct: f64) {
        let data = array![1.0, 2.0, 3.0];
        assert!(winsorize(&data, pct).is_err());
    }

    #[test]
    fn winsorize_handles_nan() {
        let data = array![1.0, f64::NAN, 3.0, 4.0, 5.0];
        let result = winsorize(&data, 0.1).unwrap();
        assert!(result[1].is_nan());
    }

    #[test]
    fn winsorize_empty_array() {
        let data: Array1<f64> = array![];
        let result = winsorize(&data, 0.1).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn winsorizer_creation() {
        assert!(Winsorizer::new(0.05).is_ok());
        assert!(Winsorizer::new(0.0).is_err());
        assert!(Winsorizer::new(0.5).is_err());
    }

    #[test]
    fn winsorizer_apply() {
        let w = Winsorizer::new(0.1).unwrap();
        let data = array![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = w.apply(&data).unwrap();
        assert_eq!(result.len(), 10);
    }
}
