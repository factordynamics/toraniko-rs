//! Example: Fetching Real Market Data from Yahoo Finance
//!
//! This example demonstrates how to:
//! 1. Fetch real historical stock data from Yahoo Finance
//! 2. Compute style factor scores (size, value proxy)
//! 3. Estimate factor returns using the toraniko model
//!
//! Based on the original Python toraniko implementation:
//! <https://github.com/0xfdf/toraniko>

use std::collections::HashMap;

use polars::prelude::*;
use time::{Duration, OffsetDateTime};
use toraniko::{
    model::{EstimatorConfig, FactorReturnsEstimator},
    styles::{SizeFactor, ValueConfig, ValueFactor},
    traits::{Factor, ReturnsEstimator, StyleFactor},
};
use yahoo_finance_api as yahoo;

/// Stock universe organized by sector
const TECH_STOCKS: &[&str] =
    &["AAPL", "MSFT", "GOOGL", "META", "NVDA", "AMD", "INTC", "CRM", "ADBE", "ORCL"];
const HEALTHCARE_STOCKS: &[&str] =
    &["JNJ", "UNH", "PFE", "MRK", "ABBV", "TMO", "ABT", "LLY", "BMY", "AMGN"];
const FINANCE_STOCKS: &[&str] =
    &["JPM", "BAC", "WFC", "GS", "MS", "C", "BLK", "SCHW", "AXP", "USB"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Toraniko Factor Model with Real Yahoo Finance Data ===\n");

    // =========================================================================
    // FETCH DATA FROM YAHOO FINANCE
    // =========================================================================

    let provider = yahoo::YahooConnector::new()?;

    // Fetch 2 years of daily data
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(504); // ~2 years of trading days

    println!("Fetching data from {} to {}", start.date(), end.date());

    let all_stocks: Vec<&str> = TECH_STOCKS
        .iter()
        .chain(HEALTHCARE_STOCKS.iter())
        .chain(FINANCE_STOCKS.iter())
        .copied()
        .collect();

    // Fetch quotes for all stocks
    let mut stock_data: HashMap<String, Vec<yahoo::Quote>> = HashMap::new();
    let mut failed_symbols: Vec<String> = Vec::new();

    for symbol in &all_stocks {
        match provider.get_quote_history(symbol, start, end).await {
            Ok(response) => {
                if let Ok(quotes) = response.quotes() {
                    if !quotes.is_empty() {
                        println!("  {} - {} quotes fetched", symbol, quotes.len());
                        stock_data.insert(symbol.to_string(), quotes);
                    } else {
                        println!("  {} - no quotes available", symbol);
                        failed_symbols.push(symbol.to_string());
                    }
                }
            }
            Err(e) => {
                println!("  {} - failed: {}", symbol, e);
                failed_symbols.push(symbol.to_string());
            }
        }
    }

    println!("\nSuccessfully fetched data for {} stocks", stock_data.len());
    if !failed_symbols.is_empty() {
        println!("Failed to fetch: {:?}", failed_symbols);
    }

    // =========================================================================
    // BUILD POLARS DATAFRAMES
    // =========================================================================

    println!("\n=== Building DataFrames ===\n");

    // Build daily returns and market cap DataFrames
    let mut dates: Vec<i64> = Vec::new();
    let mut symbols: Vec<String> = Vec::new();
    let mut returns: Vec<f64> = Vec::new();
    let mut market_caps: Vec<f64> = Vec::new();
    let mut sectors: Vec<String> = Vec::new();

    // Valuation proxies (using price ratios as proxies since Yahoo doesn't provide B/P directly)
    let mut book_price_proxy: Vec<f64> = Vec::new();
    let mut sales_price_proxy: Vec<f64> = Vec::new();
    let mut cf_price_proxy: Vec<f64> = Vec::new();

    for (symbol, quotes) in &stock_data {
        let sector = if TECH_STOCKS.contains(&symbol.as_str()) {
            "Technology"
        } else if HEALTHCARE_STOCKS.contains(&symbol.as_str()) {
            "Healthcare"
        } else {
            "Finance"
        };

        // Compute daily returns from adjusted close
        for i in 1..quotes.len() {
            let prev_close = quotes[i - 1].adjclose;
            let curr_close = quotes[i].adjclose;
            let daily_return = (curr_close - prev_close) / prev_close;

            // Use timestamp as date (convert from seconds to days since epoch)
            let timestamp = quotes[i].timestamp;

            dates.push(timestamp);
            symbols.push(symbol.clone());
            returns.push(daily_return);
            sectors.push(sector.to_string());

            // Market cap proxy: use volume * close as a rough proxy
            // (Real market cap requires shares outstanding which isn't in quote data)
            let volume = quotes[i].volume as f64;
            let close = quotes[i].close;
            market_caps.push(volume * close);

            // Value proxies: use inverse of price-based ratios
            // Higher values = "cheaper" stock (value characteristic)
            // Using 52-week high/low ratios as value proxies
            let high = quotes[i].high;
            let low = quotes[i].low;
            book_price_proxy.push(low / close); // Low/Close as book proxy
            sales_price_proxy.push((high - low) / close); // Range/Close as sales proxy
            cf_price_proxy.push(volume / (close * 1e6)); // Volume/Price as CF proxy
        }
    }

    println!("Total observations: {}", dates.len());

    // Convert timestamps (seconds since epoch) to milliseconds for datetime
    let dates_ms: Vec<i64> = dates.iter().map(|t| t * 1000).collect();
    let dates_series = Series::new("timestamp".into(), &dates_ms);
    let dates_datetime = dates_series.cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?;
    // Convert to date for the factor model
    let dates_col = dates_datetime.cast(&DataType::Date)?;

    // Create returns DataFrame
    let returns_df = df! {
        "date" => dates_col.clone(),
        "symbol" => &symbols,
        "sector" => &sectors,
        "asset_returns" => &returns,
    }?;

    println!("\nReturns DataFrame:");
    println!("{}\n", returns_df.head(Some(10)));

    // Create market cap DataFrame
    let mkt_cap_df = df! {
        "date" => dates_col.clone(),
        "symbol" => &symbols,
        "market_cap" => &market_caps,
    }?;

    // Create valuation DataFrame for Value factor
    let value_df = df! {
        "date" => dates_col,
        "symbol" => &symbols,
        "book_price" => &book_price_proxy,
        "sales_price" => &sales_price_proxy,
        "cf_price" => &cf_price_proxy,
    }?;

    // =========================================================================
    // COMPUTE STYLE FACTORS
    // =========================================================================

    println!("=== Computing Style Factors ===\n");

    // Size factor (SMB - Small Minus Big)
    let size = SizeFactor::new();
    println!("Computing {} scores...", size.name());
    let size_scores = size.compute_scores(mkt_cap_df.clone().lazy())?.collect()?;
    println!("Size scores sample:");
    println!("{}\n", size_scores.head(Some(5)));

    // Value factor (composite of valuation ratios)
    let value = ValueFactor::with_config(ValueConfig { winsorize_features: Some(0.01) });
    println!("Computing {} scores...", value.name());
    let value_scores = value.compute_scores(value_df.lazy())?.collect()?;
    println!("Value scores sample:");
    println!("{}\n", value_scores.head(Some(5)));

    // Print summary statistics
    println!("=== Style Score Statistics ===\n");
    for (name, df) in [("sze_score", &size_scores), ("val_score", &value_scores)] {
        let score_col = df.column(name)?.f64()?;
        let mean = score_col.mean().unwrap_or(0.0);
        let std_val = score_col.std(1).unwrap_or(0.0);
        let min = score_col.min().unwrap_or(0.0);
        let max = score_col.max().unwrap_or(0.0);
        println!(
            "{:12} | mean: {:8.4} | std: {:7.4} | min: {:8.4} | max: {:7.4}",
            name, mean, std_val, min, max
        );
    }

    // =========================================================================
    // CREATE SECTOR ONE-HOT ENCODING
    // =========================================================================

    let sector_df = returns_df
        .clone()
        .lazy()
        .select([
            col("date"),
            col("symbol"),
            col("sector").eq(lit("Technology")).cast(DataType::Float64).alias("sector_Technology"),
            col("sector").eq(lit("Healthcare")).cast(DataType::Float64).alias("sector_Healthcare"),
            col("sector").eq(lit("Finance")).cast(DataType::Float64).alias("sector_Finance"),
        ])
        .collect()?;

    println!("\n=== Sector Memberships ===\n");
    println!("{}\n", sector_df.head(Some(5)));

    // =========================================================================
    // MERGE STYLE SCORES
    // =========================================================================

    let style_df = size_scores
        .lazy()
        .join(
            value_scores.lazy(),
            [col("date"), col("symbol")],
            [col("date"), col("symbol")],
            JoinArgs::new(JoinType::Inner),
        )
        .collect()?;

    println!("=== Combined Style Scores ===\n");
    println!("{}\n", style_df.head(Some(5)));

    // =========================================================================
    // ESTIMATE FACTOR RETURNS
    // =========================================================================

    println!("=== Factor Returns Estimation ===\n");

    let config = EstimatorConfig { winsor_factor: Some(0.05), residualize_styles: true };

    let estimator = FactorReturnsEstimator::with_config(config);
    println!("Estimator configuration:");
    println!("  - Winsorization: {:?}", estimator.winsor_factor());
    println!("  - Residualize styles: {}", estimator.residualize_styles());

    // Prepare DataFrames for estimation
    let returns_lazy = returns_df.lazy().select([col("date"), col("symbol"), col("asset_returns")]);

    let mkt_cap_lazy = mkt_cap_df.lazy();
    let sector_lazy = sector_df.lazy();
    let style_lazy = style_df.lazy();

    let (factor_returns, residuals) =
        estimator.estimate(returns_lazy, mkt_cap_lazy, sector_lazy, style_lazy)?;

    println!("\n=== Estimated Factor Returns ===\n");
    println!("{}\n", factor_returns.head(Some(20)));

    println!("=== Asset Residual Returns (sample) ===\n");
    println!("{}\n", residuals.head(Some(10)));

    // =========================================================================
    // COMPUTE ANNUALIZED FACTOR STATISTICS
    // =========================================================================

    println!("=== Annualized Factor Performance ===\n");

    let factor_names: Vec<String> = factor_returns
        .column("factor")?
        .str()?
        .into_iter()
        .filter_map(|s| s.map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    println!("{:<25} | {:>12} | {:>12} | {:>12}", "Factor", "Ann. Return", "Ann. Vol", "Sharpe");
    println!("{}", "-".repeat(70));

    for factor in &factor_names {
        let factor_data = factor_returns
            .clone()
            .lazy()
            .filter(col("factor").eq(lit(factor.as_str())))
            .collect()?;

        let returns_col = factor_data.column("factor_return")?.f64()?;
        let mean_return = returns_col.mean().unwrap_or(0.0);
        let std_return = returns_col.std(1).unwrap_or(0.0);

        let ann_return = mean_return * 252.0;
        let ann_vol = std_return * (252.0_f64).sqrt();
        let sharpe = if ann_vol > 0.0 { ann_return / ann_vol } else { 0.0 };

        println!(
            "{:<25} | {:>11.2}% | {:>11.2}% | {:>12.2}",
            factor,
            ann_return * 100.0,
            ann_vol * 100.0,
            sharpe
        );
    }

    println!("\n=== Data Source ===");
    println!("All data fetched from Yahoo Finance");
    println!("Universe: {} stocks across 3 sectors", stock_data.len());
    println!("Period: {} to {}", start.date(), end.date());

    Ok(())
}
