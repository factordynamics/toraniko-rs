//! Factor trait definitions.

use polars::prelude::*;

/// The kind of factor in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactorKind {
    /// Market factor (beta to overall market).
    Market,
    /// Sector/industry classification factor.
    Sector,
    /// Style/characteristic factor (momentum, value, size, etc.).
    Style,
}

impl std::fmt::Display for FactorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Market => write!(f, "market"),
            Self::Sector => write!(f, "sector"),
            Self::Style => write!(f, "style"),
        }
    }
}

/// Errors that can occur during factor computation.
#[derive(Debug, thiserror::Error)]
pub enum FactorError {
    /// Missing required column in input data.
    #[error("missing required column: {0}")]
    MissingColumn(String),

    /// Invalid data encountered.
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// Polars computation error.
    #[error("computation error: {0}")]
    Computation(#[from] PolarsError),

    /// Insufficient data for factor calculation.
    #[error("insufficient data: need {required} rows, got {actual}")]
    InsufficientData {
        /// Required number of rows.
        required: usize,
        /// Actual number of rows.
        actual: usize,
    },

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Core trait for all factor types.
///
/// Factors compute exposure scores for assets based on their characteristics.
pub trait Factor: Send + Sync {
    /// Returns the unique name of this factor.
    fn name(&self) -> &str;

    /// Returns the kind of factor.
    fn kind(&self) -> FactorKind;

    /// Compute factor scores for the given asset data.
    ///
    /// # Arguments
    /// * `data` - LazyFrame with required columns for this factor
    ///
    /// # Returns
    /// LazyFrame with columns: | date | symbol | {factor_name}_score |
    ///
    /// # Errors
    /// Returns `FactorError` if required columns are missing or computation fails.
    fn compute_scores(&self, data: LazyFrame) -> Result<LazyFrame, FactorError>;

    /// Returns the required input columns for this factor.
    fn required_columns(&self) -> &[&str];

    /// Validate that the input data has all required columns.
    fn validate_columns(&self, schema: &Schema) -> Result<(), FactorError> {
        for &col in self.required_columns() {
            if schema.get(col).is_none() {
                return Err(FactorError::MissingColumn(col.to_string()));
            }
        }
        Ok(())
    }
}

/// Trait for style factors (momentum, value, size, etc.).
pub trait StyleFactor: Factor {
    /// Configuration type for this style factor.
    type Config: Default + Clone + Send + Sync;

    /// Create a new style factor with the given configuration.
    fn with_config(config: Self::Config) -> Self;

    /// Returns the current configuration.
    fn config(&self) -> &Self::Config;

    /// Returns whether this factor should be orthogonalized to sector factors.
    fn residualize(&self) -> bool {
        true // default behavior
    }
}

/// Trait for sector/industry factors.
pub trait SectorFactor: Factor {
    /// Returns the list of sector names covered by this factor.
    fn sector_names(&self) -> &[String];

    /// Returns whether sector returns should be constrained to sum to zero.
    fn constrained(&self) -> bool {
        true // Barra-style constraint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_kind_display() {
        assert_eq!(FactorKind::Market.to_string(), "market");
        assert_eq!(FactorKind::Sector.to_string(), "sector");
        assert_eq!(FactorKind::Style.to_string(), "style");
    }
}
