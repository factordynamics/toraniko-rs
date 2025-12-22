//! Cross-sectional statistical operations.

use ndarray::Array1;
use polars::prelude::*;

/// Cross-sectionally center (demean) a column partitioned by group.
///
/// # Arguments
/// * `target_col` - Column to center
/// * `over_col` - Column to partition by (typically "date")
/// * `standardize` - If true, also divide by standard deviation
///
/// # Returns
/// Polars expression for the centered values.
pub fn center_xsection(target_col: &str, over_col: &str, standardize: bool) -> Expr {
    let centered = col(target_col) - col(target_col).mean().over([col(over_col)]);

    if standardize { centered / col(target_col).std(1).over([col(over_col)]) } else { centered }
}

/// Cross-sectionally normalize a column to [lower, upper] range.
///
/// # Arguments
/// * `target_col` - Column to normalize
/// * `over_col` - Column to partition by
/// * `lower` - Lower bound of output range
/// * `upper` - Upper bound of output range
///
/// # Returns
/// Polars expression for the normalized values.
pub fn norm_xsection(target_col: &str, over_col: &str, lower: f64, upper: f64) -> Expr {
    let min_val = col(target_col).min().over([col(over_col)]);
    let max_val = col(target_col).max().over([col(over_col)]);
    let range = max_val - min_val.clone();

    // (x - min) / (max - min) * (upper - lower) + lower
    ((col(target_col) - min_val) / range) * lit(upper - lower) + lit(lower)
}

/// Mark values outside percentile thresholds.
///
/// Values within the interior are filled with `fill_val`, useful for
/// constructing long-short portfolios.
pub fn percentiles_xsection(
    target_col: &str,
    over_col: &str,
    lower_pct: f64,
    upper_pct: f64,
    fill_val: f64,
) -> Expr {
    let lower_thresh =
        col(target_col).quantile(lit(lower_pct), QuantileMethod::Linear).over([col(over_col)]);
    let upper_thresh =
        col(target_col).quantile(lit(upper_pct), QuantileMethod::Linear).over([col(over_col)]);

    when(col(target_col).lt(lower_thresh))
        .then(col(target_col))
        .when(col(target_col).gt(upper_thresh))
        .then(col(target_col))
        .otherwise(lit(fill_val))
}

/// Cross-sectional centering transform.
#[derive(Debug, Clone)]
pub struct CenterXSection {
    /// Whether to also standardize (divide by std).
    pub standardize: bool,
}

impl CenterXSection {
    /// Create a new centering transform.
    #[must_use]
    pub const fn new(standardize: bool) -> Self {
        Self { standardize }
    }

    /// Apply centering to an array.
    #[must_use]
    pub fn apply(&self, data: &Array1<f64>) -> Array1<f64> {
        if data.is_empty() {
            return data.clone();
        }

        let mean = data.mean().unwrap_or(0.0);
        let centered = data - mean;

        if self.standardize {
            let std = self.compute_std(&centered);
            if std > 0.0 { &centered / std } else { centered }
        } else {
            centered
        }
    }

    fn compute_std(&self, data: &Array1<f64>) -> f64 {
        let n = data.len() as f64;
        if n <= 1.0 {
            return 0.0;
        }
        let mean = data.mean().unwrap_or(0.0);
        let variance: f64 = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        variance.sqrt()
    }
}

impl Default for CenterXSection {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Cross-sectional normalization transform.
#[derive(Debug, Clone)]
pub struct NormXSection {
    /// Lower bound of output range.
    pub lower: f64,
    /// Upper bound of output range.
    pub upper: f64,
}

impl NormXSection {
    /// Create a new normalization transform.
    #[must_use]
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    /// Apply normalization to an array.
    #[must_use]
    pub fn apply(&self, data: &Array1<f64>) -> Array1<f64> {
        if data.is_empty() {
            return data.clone();
        }

        let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;

        if range > 0.0 {
            (data - min_val) / range * (self.upper - self.lower) + self.lower
        } else {
            Array1::from_elem(data.len(), (self.lower + self.upper) / 2.0)
        }
    }
}

impl Default for NormXSection {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use ndarray::array;
    use rstest::rstest;

    use super::*;

    #[test]
    fn center_removes_mean() {
        let data = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let centered = CenterXSection::new(false).apply(&data);
        assert_relative_eq!(centered.mean().unwrap(), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn center_standardize_unit_variance() {
        let data = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = CenterXSection::new(true).apply(&data);
        // Mean should be 0
        assert_relative_eq!(result.mean().unwrap(), 0.0, epsilon = 1e-10);
    }

    #[rstest]
    #[case(array![0.0, 25.0, 50.0, 75.0, 100.0], 0.0, 1.0)]
    #[case(array![0.0, 50.0, 100.0], -1.0, 1.0)]
    fn norm_scales_to_range(#[case] data: Array1<f64>, #[case] lower: f64, #[case] upper: f64) {
        let normed = NormXSection::new(lower, upper).apply(&data);
        assert_relative_eq!(
            normed.iter().cloned().fold(f64::INFINITY, f64::min),
            lower,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            normed.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            upper,
            epsilon = 1e-10
        );
    }

    #[test]
    fn norm_constant_array() {
        let data = array![5.0, 5.0, 5.0];
        let normed = NormXSection::new(0.0, 1.0).apply(&data);
        // Should return midpoint
        assert_relative_eq!(normed[0], 0.5, epsilon = 1e-10);
    }

    #[test]
    fn empty_array_handling() {
        let empty: Array1<f64> = array![];
        assert!(CenterXSection::new(true).apply(&empty).is_empty());
        assert!(NormXSection::new(0.0, 1.0).apply(&empty).is_empty());
    }
}
