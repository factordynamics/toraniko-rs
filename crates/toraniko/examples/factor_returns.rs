//! Example: Estimating Factor Returns with Real Market Data
//!
//! This example demonstrates the full factor return estimation pipeline:
//! 1. Fetch real market data from Yahoo Finance
//! 2. Prepare returns, market cap, sector, and style data
//! 3. Configure and run the factor returns estimator
//! 4. Analyze the estimated factor returns and residuals
//!
//! Based on the original Python toraniko implementation:
//! <https://github.com/0xfdf/toraniko>

use std::collections::HashMap;

use chrono::NaiveDate;
use factors::FactorRegistry;
use polars::prelude::*;
use time::{Duration, OffsetDateTime};
use toraniko::{
    model::{EstimatorConfig, FactorReturnsEstimator},
    traits::ReturnsEstimator,
};
use yahoo_finance_api as yahoo;

/// Stock universe organized by sector
const TECH_STOCKS: &[&str] = &["AAPL", "MSFT", "GOOGL", "NVDA"];
const HEALTHCARE_STOCKS: &[&str] = &["JNJ", "PFE", "UNH", "MRK"];
const FINANCE_STOCKS: &[&str] = &["JPM", "BAC", "GS", "C"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Factor Returns Estimation with Yahoo Finance Data ===\n");

    // =========================================================================
    // FETCH DATA FROM YAHOO FINANCE
    // =========================================================================

    let provider = yahoo::YahooConnector::new()?;

    // Fetch 6 months of daily data
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(180);

    println!("Fetching data from {} to {}\n", start.date(), end.date());

    let all_stocks: Vec<(&str, &str)> = TECH_STOCKS
        .iter()
        .map(|s| (*s, "Technology"))
        .chain(HEALTHCARE_STOCKS.iter().map(|s| (*s, "Healthcare")))
        .chain(FINANCE_STOCKS.iter().map(|s| (*s, "Finance")))
        .collect();

    let mut stock_data: HashMap<String, (Vec<yahoo::Quote>, String)> = HashMap::new();

    for (symbol, sector) in &all_stocks {
        match provider.get_quote_history(symbol, start, end).await {
            Ok(response) => {
                if let Ok(quotes) = response.quotes()
                    && !quotes.is_empty()
                {
                    println!("  {} ({}) - {} quotes", symbol, sector, quotes.len());
                    stock_data.insert(symbol.to_string(), (quotes, sector.to_string()));
                }
            }
            Err(e) => {
                println!("  {} - failed: {}", symbol, e);
            }
        }
    }

    println!("\nSuccessfully fetched data for {} stocks\n", stock_data.len());

    // =========================================================================
    // BUILD DATAFRAMES
    // =========================================================================

    let mut dates: Vec<i64> = Vec::new();
    let mut symbols: Vec<String> = Vec::new();
    let mut sectors: Vec<String> = Vec::new();
    let mut returns: Vec<f64> = Vec::new();
    let mut market_caps: Vec<f64> = Vec::new();
    let mut book_price_proxy: Vec<f64> = Vec::new();
    let mut sales_price_proxy: Vec<f64> = Vec::new();
    let mut cf_price_proxy: Vec<f64> = Vec::new();

    for (symbol, (quotes, sector)) in &stock_data {
        for i in 1..quotes.len() {
            let prev_close = quotes[i - 1].adjclose;
            let curr_close = quotes[i].adjclose;
            let daily_return = (curr_close - prev_close) / prev_close;

            dates.push(quotes[i].timestamp);
            symbols.push(symbol.clone());
            sectors.push(sector.clone());
            returns.push(daily_return);

            // Market cap proxy
            let volume = quotes[i].volume as f64;
            let close = quotes[i].close;
            market_caps.push(volume * close);

            // Value proxies
            let high = quotes[i].high;
            let low = quotes[i].low;
            book_price_proxy.push(low / close);
            sales_price_proxy.push((high - low) / close);
            cf_price_proxy.push(volume / (close * 1e6));
        }
    }

    // Convert timestamps to dates
    let dates_ms: Vec<i64> = dates.iter().map(|t| t * 1000).collect();
    let dates_series = Series::new("timestamp".into(), &dates_ms);
    let dates_datetime = dates_series.cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?;
    let dates_col = dates_datetime.cast(&DataType::Date)?;

    // Daily returns DataFrame
    let returns_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("asset_returns".into(), returns),
    ])?;

    // Market capitalizations DataFrame
    let mkt_cap_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("market_cap".into(), market_caps),
    ])?;

    // Valuation DataFrame for computing value scores
    let value_input_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("book_price".into(), book_price_proxy),
        Column::new("sales_price".into(), sales_price_proxy),
        Column::new("cf_price".into(), cf_price_proxy),
    ])?;

    // Sector memberships (one-hot encoded)
    let sector_df = DataFrame::new(vec![
        dates_col.with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("sector".into(), sectors),
    ])?
    .lazy()
    .with_columns([
        col("sector").eq(lit("Technology")).cast(DataType::Float64).alias("sector_Technology"),
        col("sector").eq(lit("Healthcare")).cast(DataType::Float64).alias("sector_Healthcare"),
        col("sector").eq(lit("Finance")).cast(DataType::Float64).alias("sector_Finance"),
    ])
    .select([col("*").exclude(["sector"])])
    .collect()?;

    println!("Built DataFrames:");
    println!("  - Returns: {} rows", returns_df.height());
    println!("  - Market caps: {} rows", mkt_cap_df.height());
    println!("  - Sectors: {} rows\n", sector_df.height());

    // =========================================================================
    // COMPUTE STYLE FACTOR SCORES
    // =========================================================================

    println!("=== Computing Style Factor Scores ===\n");

    // Initialize factor registry
    let registry = FactorRegistry::with_defaults();

    // Get a reference date from the data for factor computation
    let dates_col = mkt_cap_df.column("date")?;
    let first_date = dates_col.get(0)?;
    // Convert from polars Date to chrono NaiveDate
    let date_i32 = first_date.try_extract::<i32>()?;
    let date = NaiveDate::from_num_days_from_ce_opt(date_i32).ok_or("Invalid date")?;

    // Size factor
    let size = registry.get("market_cap").ok_or("Size factor not found")?;
    let size_scores = size.compute(&mkt_cap_df.clone().lazy(), date)?;
    println!("Computed {} size scores", size_scores.height());

    // Value factor
    let value = registry.get("book_to_market").ok_or("Value factor not found")?;
    let value_scores = value.compute(&value_input_df.lazy(), date)?;
    println!("Computed {} value scores\n", value_scores.height());

    // Combine style scores
    let style_df = size_scores
        .lazy()
        .join(
            value_scores.lazy(),
            [col("date"), col("symbol")],
            [col("date"), col("symbol")],
            JoinArgs::new(JoinType::Inner),
        )
        .collect()?;

    println!("Combined style scores sample:");
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

    let (factor_returns, residuals) = estimator.estimate(
        returns_df.lazy(),
        mkt_cap_df.lazy(),
        sector_df.lazy(),
        style_df.lazy(),
    )?;

    // =========================================================================
    // ANALYZE RESULTS
    // =========================================================================

    println!("\n=== Estimated Factor Returns ===\n");
    println!("{}\n", factor_returns.head(Some(20)));

    println!("=== Asset Residual Returns (sample) ===\n");
    println!("{}\n", residuals.head(Some(10)));

    // Compute summary statistics
    println!("=== Factor Return Statistics ===\n");

    let stats = factor_returns
        .lazy()
        .group_by([col("factor")])
        .agg([
            col("factor_return").mean().alias("mean"),
            col("factor_return").std(1).alias("std"),
            col("factor_return").count().alias("n_obs"),
        ])
        .sort(["factor"], Default::default())
        .collect()?;

    println!("{}\n", stats);

    // Annualized metrics
    println!("=== Annualized Performance ===\n");
    println!("{:<20} | {:>12} | {:>12} | {:>10}", "Factor", "Ann. Return", "Ann. Vol", "Sharpe");
    println!("{}", "-".repeat(60));

    let factors = stats.column("factor")?.str()?;
    let means = stats.column("mean")?.f64()?;
    let stds = stats.column("std")?.f64()?;

    for i in 0..stats.height() {
        let factor = factors.get(i).unwrap_or("?");
        let mean = means.get(i).unwrap_or(0.0);
        let std = stds.get(i).unwrap_or(0.0);

        let ann_return = mean * 252.0;
        let ann_vol = std * (252.0_f64).sqrt();
        let sharpe = if ann_vol > 0.0 { ann_return / ann_vol } else { 0.0 };

        println!(
            "{:<20} | {:>10.2}% | {:>10.2}% | {:>10.2}",
            factor,
            ann_return * 100.0,
            ann_vol * 100.0,
            sharpe
        );
    }

    // Interpretation
    println!("\n=== Interpretation ===\n");
    println!("The factor model decomposes asset returns as:");
    println!(
        "  r_asset = beta_market * r_market + sum(beta_sector * r_sector) + sum(beta_style * r_style) + epsilon"
    );
    println!();
    println!("Key uses:");
    println!("  - Portfolio optimization (factor covariance matrix)");
    println!("  - Risk attribution (factor exposures)");
    println!("  - Market-neutral portfolio construction (zero market/sector exposure)");

    println!("\n=== Data Source ===");
    println!("All data fetched from Yahoo Finance");
    println!("Period: {} to {}", start.date(), end.date());

    Ok(())
}
