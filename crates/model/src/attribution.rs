//! Factor attribution analysis for individual stocks.
//!
//! This module provides tools for decomposing an individual stock's
//! returns into factor contributions.

use polars::prelude::*;

use crate::ModelError;

/// Helper to extract cumulative factor return from a DataFrame.
fn extract_factor_return_sum(df: &DataFrame) -> f64 {
    df.column("factor_return")
        .ok()
        .and_then(|c| c.f64().ok())
        .map(|f| f.iter().flatten().sum())
        .unwrap_or(0.0)
}

/// Factor contribution to stock returns.
#[derive(Debug, Clone)]
pub struct FactorContribution {
    /// Factor name.
    pub factor: String,
    /// Factor exposure (loading/beta).
    pub exposure: f64,
    /// Factor return over the period.
    pub factor_return: f64,
    /// Contribution to total return (exposure * factor_return).
    pub contribution: f64,
}

/// Attribution result for a stock over a period.
#[derive(Debug, Clone)]
pub struct AttributionResult {
    /// Stock symbol.
    pub symbol: String,
    /// Start date of the analysis period.
    pub start_date: String,
    /// End date of the analysis period.
    pub end_date: String,
    /// Total stock return over the period.
    pub total_return: f64,
    /// Market contribution.
    pub market_contribution: f64,
    /// Sector contributions.
    pub sector_contributions: Vec<FactorContribution>,
    /// Style factor contributions.
    pub style_contributions: Vec<FactorContribution>,
    /// Idiosyncratic (residual) contribution.
    pub idiosyncratic_contribution: f64,
    /// R-squared: percentage of return explained by factors.
    pub r_squared: f64,
}

impl AttributionResult {
    /// Total return explained by factors (excluding idiosyncratic).
    #[must_use]
    pub fn factor_explained_return(&self) -> f64 {
        self.market_contribution
            + self.sector_contributions.iter().map(|c| c.contribution).sum::<f64>()
            + self.style_contributions.iter().map(|c| c.contribution).sum::<f64>()
    }

    /// Print a concise summary of the attribution.
    pub fn print_summary(&self) {
        println!(
            "\n================================================================================"
        );
        println!("FACTOR ATTRIBUTION ANALYSIS: {}", self.symbol);
        println!(
            "================================================================================"
        );
        println!("Period: {} to {}", self.start_date, self.end_date);
        println!("Total Return: {:>+8.2}%", self.total_return * 100.0);
        println!(
            "--------------------------------------------------------------------------------"
        );
        println!("\nFACTOR CONTRIBUTIONS:");
        println!("{:<20} {:>12} {:>14} {:>14}", "Factor", "Exposure", "Factor Ret", "Contribution");
        println!("{:-<20} {:-^12} {:-^14} {:-^14}", "", "", "", "");

        // Market
        println!(
            "{:<20} {:>12.3} {:>13.2}% {:>13.2}%",
            "Market",
            1.0,
            self.market_contribution * 100.0,
            self.market_contribution * 100.0
        );

        // Sectors
        for contrib in &self.sector_contributions {
            if contrib.exposure.abs() > 0.001 {
                println!(
                    "{:<20} {:>12.3} {:>13.2}% {:>13.2}%",
                    contrib.factor.replace("sector_", ""),
                    contrib.exposure,
                    contrib.factor_return * 100.0,
                    contrib.contribution * 100.0
                );
            }
        }

        // Styles
        for contrib in &self.style_contributions {
            let name = contrib.factor.replace("_score", "");
            let display_name = match name.as_str() {
                "mom" => "Momentum",
                "val" => "Value",
                "sze" => "Size",
                _ => &name,
            };
            println!(
                "{:<20} {:>12.3} {:>13.2}% {:>13.2}%",
                display_name,
                contrib.exposure,
                contrib.factor_return * 100.0,
                contrib.contribution * 100.0
            );
        }

        // Idiosyncratic
        println!(
            "{:<20} {:>12} {:>14} {:>13.2}%",
            "Idiosyncratic",
            "-",
            "-",
            self.idiosyncratic_contribution * 100.0
        );

        println!("{:-<20} {:-^12} {:-^14} {:-^14}", "", "", "", "");
        println!("{:<20} {:>12} {:>14} {:>13.2}%", "TOTAL", "", "", self.total_return * 100.0);

        println!(
            "\n--------------------------------------------------------------------------------"
        );
        println!("SUMMARY:");
        println!("  Factor-Explained Return: {:>+8.2}%", self.factor_explained_return() * 100.0);
        println!("  Idiosyncratic Return:    {:>+8.2}%", self.idiosyncratic_contribution * 100.0);
        println!("  R-squared:               {:>8.1}%", self.r_squared * 100.0);
        println!(
            "================================================================================\n"
        );
    }
}

/// Compute factor attribution for a specific stock.
///
/// # Arguments
/// * `symbol` - The stock symbol to analyze
/// * `factor_returns` - DataFrame with columns: date, factor, factor_return
/// * `residuals` - DataFrame with columns: date, symbol, residual_return
/// * `style_scores` - DataFrame with columns: date, symbol, *_score
/// * `sector_df` - DataFrame with columns: date, symbol, sector_*
///
/// # Returns
/// Attribution result showing factor contributions.
///
/// # Errors
/// Returns error if the symbol is not found or data is insufficient.
pub fn compute_attribution(
    symbol: &str,
    factor_returns: &DataFrame,
    residuals: &DataFrame,
    style_scores: &DataFrame,
    sector_df: &DataFrame,
) -> Result<AttributionResult, ModelError> {
    // Filter for the target symbol
    let symbol_residuals =
        residuals.clone().lazy().filter(col("symbol").eq(lit(symbol))).collect()?;

    if symbol_residuals.height() == 0 {
        return Err(ModelError::Polars(PolarsError::NoData(
            format!("No data found for symbol: {symbol}").into(),
        )));
    }

    let symbol_styles =
        style_scores.clone().lazy().filter(col("symbol").eq(lit(symbol))).collect()?;

    let symbol_sectors =
        sector_df.clone().lazy().filter(col("symbol").eq(lit(symbol))).collect()?;

    // Get the dates directly from the original residuals DataFrame
    // using Polars lazy operations which handle date formatting correctly
    let date_range = symbol_residuals
        .clone()
        .lazy()
        .select([
            col("date").min().dt().to_string("%Y-%m-%d").alias("min_date"),
            col("date").max().dt().to_string("%Y-%m-%d").alias("max_date"),
        ])
        .collect()?;

    let start_date = date_range
        .column("min_date")
        .ok()
        .and_then(|c| c.get(0).ok())
        .map(|v| v.to_string().trim_matches('"').to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let end_date = date_range
        .column("max_date")
        .ok()
        .and_then(|c| c.get(0).ok())
        .map(|v| v.to_string().trim_matches('"').to_string())
        .unwrap_or_else(|| "N/A".to_string());

    // Compute cumulative idiosyncratic return
    let residual_col = symbol_residuals.column("residual_return")?.f64()?;
    let idio_return: f64 = residual_col.iter().flatten().sum();

    // Get average style exposures for this stock
    let style_cols: Vec<String> = symbol_styles
        .get_column_names()
        .iter()
        .filter(|c| c.ends_with("_score"))
        .map(|s| s.to_string())
        .collect();

    let mut style_contributions = Vec::new();
    for style_col in &style_cols {
        if let Ok(col_data) = symbol_styles.column(style_col)
            && let Ok(f64_col) = col_data.f64()
        {
            let avg_exposure = f64_col.mean().unwrap_or(0.0);

            // Get cumulative factor return for this style
            let style_factor_returns = factor_returns
                .clone()
                .lazy()
                .filter(col("factor").eq(lit(style_col.as_str())))
                .select([col("factor_return")])
                .collect()?;

            let factor_ret = extract_factor_return_sum(&style_factor_returns);

            style_contributions.push(FactorContribution {
                factor: style_col.clone(),
                exposure: avg_exposure,
                factor_return: factor_ret,
                contribution: avg_exposure * factor_ret,
            });
        }
    }

    // Get sector exposures
    let sector_cols: Vec<String> = symbol_sectors
        .get_column_names()
        .iter()
        .filter(|c| c.starts_with("sector_"))
        .map(|s| s.to_string())
        .collect();

    let mut sector_contributions = Vec::new();
    for sector_col in &sector_cols {
        if let Ok(col_data) = symbol_sectors.column(sector_col)
            && let Ok(f64_col) = col_data.f64()
        {
            let avg_exposure = f64_col.mean().unwrap_or(0.0);

            // Get cumulative factor return for this sector
            let sector_factor_returns = factor_returns
                .clone()
                .lazy()
                .filter(col("factor").eq(lit(sector_col.as_str())))
                .select([col("factor_return")])
                .collect()?;

            let factor_ret = extract_factor_return_sum(&sector_factor_returns);

            sector_contributions.push(FactorContribution {
                factor: sector_col.clone(),
                exposure: avg_exposure,
                factor_return: factor_ret,
                contribution: avg_exposure * factor_ret,
            });
        }
    }

    // Get market contribution
    let market_returns = factor_returns
        .clone()
        .lazy()
        .filter(col("factor").eq(lit("market")))
        .select([col("factor_return")])
        .collect()?;

    let market_contribution = extract_factor_return_sum(&market_returns);

    // Compute total return
    let factor_explained = market_contribution
        + sector_contributions.iter().map(|c| c.contribution).sum::<f64>()
        + style_contributions.iter().map(|c| c.contribution).sum::<f64>();
    let total_return = factor_explained + idio_return;

    // Compute R-squared (simplified)
    let r_squared = if total_return.abs() > 1e-10 {
        (factor_explained / total_return).abs().min(1.0)
    } else {
        0.0
    };

    Ok(AttributionResult {
        symbol: symbol.to_string(),
        start_date,
        end_date,
        total_return,
        market_contribution,
        sector_contributions,
        style_contributions,
        idiosyncratic_contribution: idio_return,
        r_squared,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_contribution_creation() {
        let contrib = FactorContribution {
            factor: "mom_score".to_string(),
            exposure: 1.5,
            factor_return: 0.02,
            contribution: 0.03,
        };
        assert_eq!(contrib.factor, "mom_score");
        assert!((contrib.contribution - 0.03).abs() < 1e-10);
    }

    #[test]
    fn attribution_result_factor_explained() {
        let result = AttributionResult {
            symbol: "TEST".to_string(),
            start_date: "2024-01-01".to_string(),
            end_date: "2024-12-31".to_string(),
            total_return: 0.15,
            market_contribution: 0.10,
            sector_contributions: vec![FactorContribution {
                factor: "sector_Tech".to_string(),
                exposure: 1.0,
                factor_return: 0.02,
                contribution: 0.02,
            }],
            style_contributions: vec![FactorContribution {
                factor: "mom_score".to_string(),
                exposure: 0.5,
                factor_return: 0.04,
                contribution: 0.02,
            }],
            idiosyncratic_contribution: 0.01,
            r_squared: 0.93,
        };

        let explained = result.factor_explained_return();
        assert!((explained - 0.14).abs() < 1e-10);
    }
}
