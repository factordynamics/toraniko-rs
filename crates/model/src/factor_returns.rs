//! Factor returns estimation.

use ndarray::{Array1, Array2};
use polars::prelude::*;
use toraniko_primitives::Date;
use toraniko_traits::{EstimatorError, FactorEstimator, ReturnsEstimator};

use crate::{ModelError, WlsConfig, WlsFactorEstimator};

/// Configuration for factor returns estimation.
#[derive(Debug, Clone)]
pub struct EstimatorConfig {
    /// Winsorization percentile for returns (None to disable).
    pub winsor_factor: Option<f64>,
    /// Whether to orthogonalize style returns to sector residuals.
    pub residualize_styles: bool,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self { winsor_factor: Some(0.05), residualize_styles: true }
    }
}

/// Main factor returns estimator.
///
/// Estimates market, sector, and style factor returns using weighted
/// least squares with market cap weighting and sector sum constraint.
#[derive(Debug, Clone)]
pub struct FactorReturnsEstimator {
    config: EstimatorConfig,
    wls: WlsFactorEstimator,
}

impl FactorReturnsEstimator {
    /// Create a new estimator with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(EstimatorConfig::default())
    }

    /// Create a new estimator with custom configuration.
    #[must_use]
    pub fn with_config(config: EstimatorConfig) -> Self {
        let wls_config = WlsConfig {
            winsor_factor: config.winsor_factor,
            residualize_styles: config.residualize_styles,
        };
        Self { config, wls: WlsFactorEstimator::with_config(wls_config) }
    }

    /// Get the configuration.
    #[must_use]
    pub const fn config(&self) -> &EstimatorConfig {
        &self.config
    }

    /// Estimate factor returns for a single time period.
    ///
    /// # Arguments
    /// * `returns` - Asset returns (n_assets,)
    /// * `mkt_caps` - Market capitalizations (n_assets,)
    /// * `sector_scores` - Sector exposure matrix (n_assets x n_sectors)
    /// * `style_scores` - Style score matrix (n_assets x n_styles)
    ///
    /// # Returns
    /// Tuple of (factor_returns array, residual_returns array)
    ///
    /// # Errors
    /// Returns `ModelError` if estimation fails.
    pub fn estimate_single(
        &self,
        returns: &Array1<f64>,
        mkt_caps: &Array1<f64>,
        sector_scores: &Array2<f64>,
        style_scores: &Array2<f64>,
    ) -> Result<(Array1<f64>, Array1<f64>), ModelError> {
        self.wls
            .estimate_single(returns, mkt_caps, sector_scores, style_scores)
            .map_err(ModelError::from)
    }
}

impl Default for FactorReturnsEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReturnsEstimator for FactorReturnsEstimator {
    fn estimate(
        &self,
        returns_df: LazyFrame,
        mkt_cap_df: LazyFrame,
        sector_df: LazyFrame,
        style_df: LazyFrame,
    ) -> Result<(DataFrame, DataFrame), EstimatorError> {
        // Join all data on date and symbol
        let joined = returns_df
            .join(
                mkt_cap_df,
                [col("date"), col("symbol")],
                [col("date"), col("symbol")],
                JoinArgs::new(JoinType::Inner),
            )
            .join(
                sector_df,
                [col("date"), col("symbol")],
                [col("date"), col("symbol")],
                JoinArgs::new(JoinType::Inner),
            )
            .join(
                style_df,
                [col("date"), col("symbol")],
                [col("date"), col("symbol")],
                JoinArgs::new(JoinType::Inner),
            )
            .collect()?;

        if joined.height() == 0 {
            return Err(EstimatorError::InsufficientData { required: 1, actual: 0 });
        }

        // Identify sector and style columns
        let all_columns: Vec<String> =
            joined.get_column_names().iter().map(|s| s.to_string()).collect();

        let sector_cols: Vec<String> =
            all_columns.iter().filter(|c| c.starts_with("sector_")).cloned().collect();

        let style_cols: Vec<String> = all_columns
            .iter()
            .filter(|c| c.ends_with("_score") && *c != "asset_returns")
            .cloned()
            .collect();

        if sector_cols.is_empty() {
            return Err(EstimatorError::MissingColumn("sector_* columns".to_string()));
        }

        // Build result vectors
        let mut factor_dates: Vec<Date> = Vec::new();
        let mut factor_names: Vec<String> = Vec::new();
        let mut factor_values: Vec<f64> = Vec::new();

        let mut residual_dates: Vec<Date> = Vec::new();
        let mut residual_symbols: Vec<String> = Vec::new();
        let mut residual_values: Vec<f64> = Vec::new();

        // Group by date and process each group
        let grouped = joined.clone().lazy().group_by([col("date")]).agg([col("*")]).collect()?;

        for row_idx in 0..grouped.height() {
            // Get the date for this group
            let date_series = grouped.column("date")?;
            let date_i32 = match date_series.get(row_idx)? {
                AnyValue::Date(d) => d,
                _ => continue,
            };
            let date_val = Date::from_num_days_from_ce_opt(date_i32).unwrap_or_default();

            // Filter the original data for this date using the raw i32 value
            let date_filter = joined
                .clone()
                .lazy()
                .filter(col("date").eq(lit(date_i32).cast(DataType::Date)))
                .collect()?;

            let n = date_filter.height();
            if n < sector_cols.len() + style_cols.len() + 2 {
                continue;
            }

            // Extract arrays
            let returns = extract_array(&date_filter, "asset_returns")?;
            let mkt_caps = extract_array(&date_filter, "market_cap")?;

            // Build sector matrix
            let mut sector_matrix = Array2::zeros((n, sector_cols.len()));
            for (j, col_name) in sector_cols.iter().enumerate() {
                let col_data = extract_array(&date_filter, col_name)?;
                for i in 0..n {
                    sector_matrix[[i, j]] = col_data[i];
                }
            }

            // Build style matrix
            let mut style_matrix = Array2::zeros((n, style_cols.len().max(1)));
            if !style_cols.is_empty() {
                for (j, col_name) in style_cols.iter().enumerate() {
                    let col_data = extract_array(&date_filter, col_name)?;
                    for i in 0..n {
                        style_matrix[[i, j]] = col_data[i];
                    }
                }
            }

            // Estimate
            let (factor_rets, residuals) =
                match self.estimate_single(&returns, &mkt_caps, &sector_matrix, &style_matrix) {
                    Ok(result) => result,
                    Err(_) => continue,
                };

            // Store factor returns
            // Market
            factor_dates.push(date_val);
            factor_names.push("market".to_string());
            factor_values.push(factor_rets[0]);

            // Sectors
            for (i, name) in sector_cols.iter().enumerate() {
                factor_dates.push(date_val);
                factor_names.push(name.clone());
                factor_values.push(factor_rets[1 + i]);
            }

            // Styles
            for (i, name) in style_cols.iter().enumerate() {
                factor_dates.push(date_val);
                factor_names.push(name.clone());
                factor_values.push(factor_rets[1 + sector_cols.len() + i]);
            }

            // Store residuals
            let symbols = date_filter.column("symbol")?.str()?;
            for i in 0..n {
                residual_dates.push(date_val);
                residual_symbols.push(symbols.get(i).unwrap_or("").to_string());
                residual_values.push(residuals[i]);
            }
        }

        // Build output DataFrames
        let factor_df = DataFrame::new(vec![
            Column::new("date".into(), factor_dates.clone()),
            Column::new("factor".into(), factor_names),
            Column::new("factor_return".into(), factor_values),
        ])?;

        let residual_df = DataFrame::new(vec![
            Column::new("date".into(), residual_dates),
            Column::new("symbol".into(), residual_symbols),
            Column::new("residual_return".into(), residual_values),
        ])?;

        Ok((factor_df, residual_df))
    }

    fn winsor_factor(&self) -> Option<f64> {
        self.config.winsor_factor
    }

    fn residualize_styles(&self) -> bool {
        self.config.residualize_styles
    }
}

fn extract_array(df: &DataFrame, col_name: &str) -> Result<Array1<f64>, EstimatorError> {
    let series =
        df.column(col_name).map_err(|_| EstimatorError::MissingColumn(col_name.to_string()))?;

    let chunked = series
        .f64()
        .map_err(|_| EstimatorError::InvalidConfig(format!("column {col_name} is not f64")))?;

    let values: Vec<f64> = chunked.into_iter().map(|opt| opt.unwrap_or(0.0)).collect();

    Ok(Array1::from_vec(values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimator_config_defaults() {
        let config = EstimatorConfig::default();
        assert_eq!(config.winsor_factor, Some(0.05));
        assert!(config.residualize_styles);
    }

    #[test]
    fn estimator_creation() {
        let estimator = FactorReturnsEstimator::new();
        assert_eq!(estimator.winsor_factor(), Some(0.05));
        assert!(estimator.residualize_styles());
    }

    #[test]
    fn estimator_custom_config() {
        let config = EstimatorConfig { winsor_factor: Some(0.10), residualize_styles: false };
        let estimator = FactorReturnsEstimator::with_config(config);
        assert_eq!(estimator.winsor_factor(), Some(0.10));
        assert!(!estimator.residualize_styles());
    }
}
