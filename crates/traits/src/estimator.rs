//! Factor return estimation trait definitions.

use ndarray::{Array1, Array2};
use polars::prelude::*;

/// Errors that can occur during estimation.
#[derive(Debug, thiserror::Error)]
pub enum EstimatorError {
    /// Dimension mismatch in input data.
    #[error("dimension mismatch for {context}: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        actual: usize,
        /// Context description.
        context: String,
    },

    /// Insufficient data for estimation.
    #[error("insufficient data: need at least {required} observations, got {actual}")]
    InsufficientData {
        /// Required number of observations.
        required: usize,
        /// Actual number of observations.
        actual: usize,
    },

    /// Rank deficiency in design matrix.
    #[error("rank deficient design matrix: rank {rank} < columns {columns}")]
    RankDeficient {
        /// Actual rank.
        rank: usize,
        /// Number of columns.
        columns: usize,
    },

    /// Polars error.
    #[error("data processing error: {0}")]
    Polars(#[from] PolarsError),

    /// Missing required column.
    #[error("missing required column: {0}")]
    MissingColumn(String),

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Linear algebra error.
    #[error("linear algebra error: {0}")]
    LinearAlgebra(String),
}

impl EstimatorError {
    /// Returns whether this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::InsufficientData { .. } | Self::RankDeficient { .. })
    }
}

/// Trait for estimating factor returns from cross-sectional data.
pub trait FactorEstimator: Send + Sync {
    /// Configuration type for this estimator.
    type Config: Default + Clone + Send + Sync;

    /// Create a new estimator with the given configuration.
    fn with_config(config: Self::Config) -> Self;

    /// Estimate factor returns for a single time period.
    ///
    /// # Arguments
    /// * `returns` - Asset returns (n_assets,)
    /// * `weights` - Market cap weights (n_assets,)
    /// * `sector_scores` - Sector exposure matrix (n_assets x n_sectors)
    /// * `style_scores` - Style exposure matrix (n_assets x n_styles)
    ///
    /// # Returns
    /// Tuple of (factor_returns, residual_returns)
    ///
    /// # Errors
    /// Returns `EstimatorError` if dimensions mismatch or computation fails.
    fn estimate_single(
        &self,
        returns: &Array1<f64>,
        weights: &Array1<f64>,
        sector_scores: &Array2<f64>,
        style_scores: &Array2<f64>,
    ) -> Result<(Array1<f64>, Array1<f64>), EstimatorError>;
}

/// Trait for estimating factor returns across multiple time periods.
pub trait ReturnsEstimator: Send + Sync {
    /// Estimate factor returns across all dates in the input data.
    ///
    /// # Arguments
    /// * `returns_df` - DataFrame with | date | symbol | asset_returns |
    /// * `mkt_cap_df` - DataFrame with | date | symbol | market_cap |
    /// * `sector_df` - DataFrame with date, symbol, and sector columns
    /// * `style_df` - DataFrame with date, symbol, and style columns
    ///
    /// # Returns
    /// Tuple of (factor_returns_df, residual_returns_df)
    ///
    /// # Errors
    /// Returns `EstimatorError` if required columns are missing or estimation fails.
    fn estimate(
        &self,
        returns_df: LazyFrame,
        mkt_cap_df: LazyFrame,
        sector_df: LazyFrame,
        style_df: LazyFrame,
    ) -> Result<(DataFrame, DataFrame), EstimatorError>;

    /// Returns the winsorization factor, if any.
    fn winsor_factor(&self) -> Option<f64>;

    /// Returns whether styles are residualized to sectors.
    fn residualize_styles(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_error_is_recoverable() {
        let err = EstimatorError::InsufficientData { required: 10, actual: 5 };
        assert!(err.is_recoverable());

        let err = EstimatorError::MissingColumn("test".to_string());
        assert!(!err.is_recoverable());
    }

    #[test]
    fn estimator_error_display() {
        let err = EstimatorError::DimensionMismatch {
            expected: 100,
            actual: 50,
            context: "returns".to_string(),
        };
        assert_eq!(err.to_string(), "dimension mismatch for returns: expected 100, got 50");
    }
}
