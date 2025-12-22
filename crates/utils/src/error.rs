//! Error types for utility functions.

/// Errors that can occur during utility operations.
#[derive(Debug, thiserror::Error)]
pub enum UtilsError {
    /// Polars error.
    #[error("polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Invalid parameter.
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    /// Missing column.
    #[error("missing column: {0}")]
    MissingColumn(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = UtilsError::InvalidParameter("bad value".to_string());
        assert!(err.to_string().contains("bad value"));
    }
}
