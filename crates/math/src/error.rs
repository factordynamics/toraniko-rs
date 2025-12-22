//! Error types for mathematical operations.

/// Errors that can occur during mathematical operations.
#[derive(Debug, thiserror::Error)]
pub enum MathError {
    /// Invalid percentile value.
    #[error("invalid percentile: {0} (must be in (0, 0.5))")]
    InvalidPercentile(f64),

    /// Dimension mismatch.
    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected dimension.
        expected: usize,
        /// Actual dimension.
        actual: usize,
    },

    /// Linear algebra error.
    #[error("linear algebra error: {0}")]
    LinearAlgebra(String),

    /// Empty data.
    #[error("empty data provided")]
    EmptyData,

    /// Numerical instability (NaN or Inf).
    #[error("numerical instability: {0}")]
    NumericalInstability(String),

    /// Polars error.
    #[error("polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = MathError::InvalidPercentile(0.6);
        assert!(err.to_string().contains("0.6"));

        let err = MathError::DimensionMismatch { expected: 10, actual: 5 };
        assert!(err.to_string().contains("10") && err.to_string().contains("5"));
    }
}
