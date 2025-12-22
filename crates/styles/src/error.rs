//! Error types for style factors.

use toraniko_traits::FactorError;

/// Errors that can occur during style factor computation.
#[derive(Debug, thiserror::Error)]
pub enum StyleError {
    /// Factor computation error.
    #[error("factor error: {0}")]
    Factor(#[from] FactorError),

    /// Math operation error.
    #[error("math error: {0}")]
    Math(#[from] toraniko_math::MathError),

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = StyleError::InvalidConfig("bad parameter".to_string());
        assert!(err.to_string().contains("bad parameter"));
    }
}
