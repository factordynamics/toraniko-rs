//! Size factor implementation.

use polars::prelude::*;
use toraniko_traits::{Factor, FactorError, FactorKind, StyleFactor};

/// Configuration for size factor.
#[derive(Debug, Clone)]
pub struct SizeConfig {
    /// Lower decile for small-cap classification.
    pub lower_decile: f64,
    /// Upper decile for large-cap classification.
    pub upper_decile: f64,
}

impl Default for SizeConfig {
    fn default() -> Self {
        Self { lower_decile: 0.2, upper_decile: 0.8 }
    }
}

/// Size style factor.
///
/// Computes size scores based on market capitalization, with
/// small caps getting positive scores and large caps negative
/// (SMB - Small Minus Big style).
#[derive(Debug, Clone)]
pub struct SizeFactor {
    config: SizeConfig,
}

impl SizeFactor {
    /// Create a new size factor with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(SizeConfig::default())
    }
}

impl Default for SizeFactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Factor for SizeFactor {
    fn name(&self) -> &str {
        "size"
    }

    fn kind(&self) -> FactorKind {
        FactorKind::Style
    }

    fn compute_scores(&self, data: LazyFrame) -> Result<LazyFrame, FactorError> {
        // Compute log of market cap using map expression
        // Then compute cross-sectional z-score, negated so small caps have positive scores
        let standardized = data.with_column(
            (-(col("market_cap") - col("market_cap").mean().over([col("date")]))
                / col("market_cap").std(1).over([col("date")]))
            .alias("sze_score"),
        );

        // Select output columns
        let result = standardized.select([col("date"), col("symbol"), col("sze_score")]);

        Ok(result)
    }

    fn required_columns(&self) -> &[&str] {
        &["date", "symbol", "market_cap"]
    }
}

impl StyleFactor for SizeFactor {
    type Config = SizeConfig;

    fn with_config(config: Self::Config) -> Self {
        Self { config }
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn residualize(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_config_defaults() {
        let config = SizeConfig::default();
        assert!((config.lower_decile - 0.2).abs() < 1e-10);
        assert!((config.upper_decile - 0.8).abs() < 1e-10);
    }

    #[test]
    fn size_factor_name() {
        let factor = SizeFactor::new();
        assert_eq!(factor.name(), "size");
        assert_eq!(factor.kind(), FactorKind::Style);
    }

    #[test]
    fn size_required_columns() {
        let factor = SizeFactor::new();
        let cols = factor.required_columns();
        assert!(cols.contains(&"date"));
        assert!(cols.contains(&"symbol"));
        assert!(cols.contains(&"market_cap"));
    }
}
