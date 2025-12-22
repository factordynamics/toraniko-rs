//! Error types for factor estimation.

use toraniko_math::MathError;
use toraniko_traits::EstimatorError;

/// Errors that can occur during factor model estimation.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// Estimator error.
    #[error("estimator error: {0}")]
    Estimator(#[from] EstimatorError),

    /// Math error.
    #[error("math error: {0}")]
    Math(#[from] MathError),

    /// Polars error.
    #[error("data processing error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Missing required column.
    #[error("missing required column: {0}")]
    MissingColumn(String),

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// No data for date.
    #[error("no data for date: {0}")]
    NoDataForDate(String),

    /// Dimension mismatch.
    #[error("dimension mismatch: {0}")]
    DimensionMismatch(String),
}

impl ModelError {
    /// Returns whether this error is recoverable.
    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(self, Self::NoDataForDate(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ModelError::MissingColumn("returns".to_string());
        assert!(err.to_string().contains("returns"));
    }

    #[test]
    fn error_is_recoverable() {
        let err = ModelError::NoDataForDate("2024-01-01".to_string());
        assert!(err.is_recoverable());

        let err = ModelError::MissingColumn("test".to_string());
        assert!(!err.is_recoverable());
    }
}
