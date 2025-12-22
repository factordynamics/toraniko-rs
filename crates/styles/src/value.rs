//! Value factor implementation.

use polars::prelude::*;
use toraniko_math::winsorize_xsection;
use toraniko_traits::{Factor, FactorError, FactorKind, StyleFactor};

/// Configuration for value factor.
#[derive(Debug, Clone)]
pub struct ValueConfig {
    /// Winsorization percentile (None to disable).
    pub winsorize_features: Option<f64>,
}

impl Default for ValueConfig {
    fn default() -> Self {
        Self { winsorize_features: Some(0.01) }
    }
}

/// Value style factor.
///
/// Computes value scores from price ratios: book/price, sales/price, cash flow/price.
/// The final score is the average of these z-scored ratios.
#[derive(Debug, Clone)]
pub struct ValueFactor {
    config: ValueConfig,
}

impl ValueFactor {
    /// Create a new value factor with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ValueConfig::default())
    }
}

impl Default for ValueFactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Factor for ValueFactor {
    fn name(&self) -> &str {
        "value"
    }

    fn kind(&self) -> FactorKind {
        FactorKind::Style
    }

    fn compute_scores(&self, data: LazyFrame) -> Result<LazyFrame, FactorError> {
        let value_cols = ["book_price", "sales_price", "cf_price"];

        // Optionally winsorize
        let lf = if let Some(pct) = self.config.winsorize_features {
            winsorize_xsection(data, &value_cols, "date", pct)
        } else {
            data
        };

        // Z-score each value metric cross-sectionally
        let lf = lf.with_column(
            ((col("book_price") - col("book_price").mean().over([col("date")]))
                / col("book_price").std(1).over([col("date")]))
            .alias("bp_z"),
        );

        let lf = lf.with_column(
            ((col("sales_price") - col("sales_price").mean().over([col("date")]))
                / col("sales_price").std(1).over([col("date")]))
            .alias("sp_z"),
        );

        let lf = lf.with_column(
            ((col("cf_price") - col("cf_price").mean().over([col("date")]))
                / col("cf_price").std(1).over([col("date")]))
            .alias("cfp_z"),
        );

        // Composite score is the average
        let lf = lf.with_column(
            ((col("bp_z") + col("sp_z") + col("cfp_z")) / lit(3.0)).alias("val_score"),
        );

        // Select output columns
        let result = lf.select([col("date"), col("symbol"), col("val_score")]);

        Ok(result)
    }

    fn required_columns(&self) -> &[&str] {
        &["date", "symbol", "book_price", "sales_price", "cf_price"]
    }
}

impl StyleFactor for ValueFactor {
    type Config = ValueConfig;

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
    fn value_config_defaults() {
        let config = ValueConfig::default();
        assert_eq!(config.winsorize_features, Some(0.01));
    }

    #[test]
    fn value_factor_name() {
        let factor = ValueFactor::new();
        assert_eq!(factor.name(), "value");
        assert_eq!(factor.kind(), FactorKind::Style);
    }

    #[test]
    fn value_required_columns() {
        let factor = ValueFactor::new();
        let cols = factor.required_columns();
        assert!(cols.contains(&"book_price"));
        assert!(cols.contains(&"sales_price"));
        assert!(cols.contains(&"cf_price"));
    }

    #[test]
    fn value_no_winsorization() {
        let config = ValueConfig { winsorize_features: None };
        let factor = ValueFactor::with_config(config);
        assert!(factor.config().winsorize_features.is_none());
    }
}
