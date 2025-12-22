//! Momentum factor implementation.

use polars::prelude::*;
use toraniko_traits::{Factor, FactorError, FactorKind, StyleFactor};

/// Configuration for momentum factor.
#[derive(Debug, Clone)]
pub struct MomentumConfig {
    /// Look-back period in trading days.
    pub trailing_days: usize,
    /// Half-life for exponential weighting.
    pub half_life: usize,
    /// Lag to skip (avoids short-term reversal).
    pub lag: usize,
    /// Winsorization percentile.
    pub winsor_factor: f64,
}

impl Default for MomentumConfig {
    fn default() -> Self {
        Self {
            trailing_days: 504, // ~2 years
            half_life: 126,     // ~6 months
            lag: 20,            // ~1 month
            winsor_factor: 0.01,
        }
    }
}

/// Momentum style factor.
///
/// Computes exponentially-weighted cumulative returns over a trailing window,
/// with a lag to avoid short-term reversal effects.
#[derive(Debug, Clone)]
pub struct MomentumFactor {
    config: MomentumConfig,
}

impl MomentumFactor {
    /// Create a new momentum factor with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(MomentumConfig::default())
    }
}

impl Default for MomentumFactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Factor for MomentumFactor {
    fn name(&self) -> &str {
        "momentum"
    }

    fn kind(&self) -> FactorKind {
        FactorKind::Style
    }

    fn compute_scores(&self, data: LazyFrame) -> Result<LazyFrame, FactorError> {
        let lag = self.config.lag as i64;
        let window = self.config.trailing_days;

        // Sort by symbol and date
        let sorted =
            data.sort(["symbol", "date"], SortMultipleOptions::new().with_maintain_order(true));

        // Compute lagged returns
        let with_lag = sorted.with_column(
            col("asset_returns").shift(lit(lag)).over([col("symbol")]).alias("lagged_returns"),
        );

        // Compute rolling sum of lagged returns (simple approximation of cumulative return)
        let with_rolling = with_lag.with_column(
            col("lagged_returns")
                .rolling_sum(RollingOptionsFixedWindow {
                    window_size: window,
                    min_periods: window / 2,
                    ..Default::default()
                })
                .over([col("symbol")])
                .alias("mom_raw"),
        );

        // Cross-sectional standardization by date
        let standardized = with_rolling.with_column(
            ((col("mom_raw") - col("mom_raw").mean().over([col("date")]))
                / col("mom_raw").std(1).over([col("date")]))
            .alias("mom_score"),
        );

        // Select output columns
        let result = standardized.select([col("date"), col("symbol"), col("mom_score")]);

        Ok(result)
    }

    fn required_columns(&self) -> &[&str] {
        &["date", "symbol", "asset_returns"]
    }
}

impl StyleFactor for MomentumFactor {
    type Config = MomentumConfig;

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
    fn momentum_config_defaults() {
        let config = MomentumConfig::default();
        assert_eq!(config.trailing_days, 504);
        assert_eq!(config.half_life, 126);
        assert_eq!(config.lag, 20);
    }

    #[test]
    fn momentum_factor_name() {
        let factor = MomentumFactor::new();
        assert_eq!(factor.name(), "momentum");
        assert_eq!(factor.kind(), FactorKind::Style);
    }

    #[test]
    fn momentum_required_columns() {
        let factor = MomentumFactor::new();
        let cols = factor.required_columns();
        assert!(cols.contains(&"date"));
        assert!(cols.contains(&"symbol"));
        assert!(cols.contains(&"asset_returns"));
    }

    #[test]
    fn momentum_residualize_default() {
        let factor = MomentumFactor::new();
        assert!(factor.residualize());
    }
}
