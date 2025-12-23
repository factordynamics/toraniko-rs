//! Example: Computing Style Factor Scores with Real Market Data
//!
//! This example demonstrates how to compute momentum, value, and size
//! factor scores for a universe of equities using real Yahoo Finance data.
//!
//! Based on the original Python toraniko implementation:
//! <https://github.com/0xfdf/toraniko>

use std::collections::HashMap;

use polars::prelude::*;
use time::{Duration, OffsetDateTime};
use toraniko::{
    styles::{MomentumConfig, MomentumFactor, SizeFactor, ValueConfig, ValueFactor},
    traits::{Factor, StyleFactor},
};
use yahoo_finance_api as yahoo;

/// Stock universe
const STOCKS: &[&str] = &["AAPL", "MSFT", "GOOGL", "META", "NVDA", "JPM", "JNJ", "UNH"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Style Factor Scores with Yahoo Finance Data ===\n");

    // =========================================================================
    // FETCH DATA FROM YAHOO FINANCE
    // =========================================================================

    let provider = yahoo::YahooConnector::new()?;

    // Fetch 1 year of daily data (enough for momentum calculation)
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(365);

    println!("Fetching data from {} to {}\n", start.date(), end.date());

    let mut stock_data: HashMap<String, Vec<yahoo::Quote>> = HashMap::new();

    for symbol in STOCKS {
        match provider.get_quote_history(symbol, start, end).await {
            Ok(response) => {
                if let Ok(quotes) = response.quotes()
                    && !quotes.is_empty()
                {
                    println!("  {} - {} quotes fetched", symbol, quotes.len());
                    stock_data.insert(symbol.to_string(), quotes);
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
    let mut returns: Vec<f64> = Vec::new();
    let mut market_caps: Vec<f64> = Vec::new();
    let mut book_price_proxy: Vec<f64> = Vec::new();
    let mut sales_price_proxy: Vec<f64> = Vec::new();
    let mut cf_price_proxy: Vec<f64> = Vec::new();

    for (symbol, quotes) in &stock_data {
        for i in 1..quotes.len() {
            let prev_close = quotes[i - 1].adjclose;
            let curr_close = quotes[i].adjclose;
            let daily_return = (curr_close - prev_close) / prev_close;

            dates.push(quotes[i].timestamp);
            symbols.push(symbol.clone());
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

    // Create returns DataFrame for momentum
    let returns_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("asset_returns".into(), returns),
    ])?;

    // Create market cap DataFrame for size
    let mkt_cap_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("market_cap".into(), market_caps),
    ])?;

    // Create valuation DataFrame for value
    let value_df = DataFrame::new(vec![
        dates_col.with_name("date".into()).into(),
        Column::new("symbol".into(), symbols),
        Column::new("book_price".into(), book_price_proxy),
        Column::new("sales_price".into(), sales_price_proxy),
        Column::new("cf_price".into(), cf_price_proxy),
    ])?;

    println!("Built DataFrames with {} observations\n", returns_df.height());

    // =========================================================================
    // MOMENTUM FACTOR
    // =========================================================================
    println!("=== Momentum Factor ===\n");

    let momentum_config = MomentumConfig {
        trailing_days: 60, // ~3 months lookback
        half_life: 30,     // 1 month exponential decay
        lag: 5,            // Skip most recent week
        winsor_factor: 0.01,
    };
    let momentum = MomentumFactor::with_config(momentum_config);

    println!("Factor name: {}", momentum.name());
    println!("Factor kind: {:?}", momentum.kind());
    println!("Required columns: {:?}", momentum.required_columns());
    println!("Config: trailing_days=60, half_life=30, lag=5");

    let momentum_scores = momentum.compute_scores(returns_df.lazy())?.collect()?;

    println!("\nMomentum scores (sample - last 10 rows):");
    println!("{}\n", momentum_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = momentum_scores.column("mom_score")
        && let Ok(f64_col) = score_col.f64()
    {
        let mean = f64_col.mean().unwrap_or(0.0);
        let std = f64_col.std(1).unwrap_or(0.0);
        let non_null = f64_col.len() - f64_col.null_count();
        println!(
            "Momentum stats: mean={:.4}, std={:.4}, non-null observations={}",
            mean, std, non_null
        );
    }

    // =========================================================================
    // SIZE FACTOR
    // =========================================================================
    println!("\n=== Size Factor ===\n");

    let size = SizeFactor::new();

    println!("Factor name: {}", size.name());
    println!("Factor kind: {:?}", size.kind());
    println!("Required columns: {:?}", size.required_columns());

    let size_scores = size.compute_scores(mkt_cap_df.lazy())?.collect()?;

    println!("\nSize scores (sample - last 10 rows):");
    println!("(negative = large cap, positive = small cap)");
    println!("{}\n", size_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = size_scores.column("sze_score")
        && let Ok(f64_col) = score_col.f64()
    {
        let mean = f64_col.mean().unwrap_or(0.0);
        let std = f64_col.std(1).unwrap_or(0.0);
        println!("Size stats: mean={:.4}, std={:.4}", mean, std);
    }

    // =========================================================================
    // VALUE FACTOR
    // =========================================================================
    println!("\n=== Value Factor ===\n");

    let value_config = ValueConfig { winsorize_features: Some(0.01) };
    let value = ValueFactor::with_config(value_config);

    println!("Factor name: {}", value.name());
    println!("Factor kind: {:?}", value.kind());
    println!("Required columns: {:?}", value.required_columns());

    let value_scores = value.compute_scores(value_df.lazy())?.collect()?;

    println!("\nValue scores (sample - last 10 rows):");
    println!("(composite of book/price, sales/price, cf/price proxies)");
    println!("{}\n", value_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = value_scores.column("val_score")
        && let Ok(f64_col) = score_col.f64()
    {
        let mean = f64_col.mean().unwrap_or(0.0);
        let std = f64_col.std(1).unwrap_or(0.0);
        println!("Value stats: mean={:.4}, std={:.4}", mean, std);
    }

    println!("\n=== Data Source ===");
    println!("All data fetched from Yahoo Finance");
    println!("Period: {} to {}", start.date(), end.date());

    Ok(())
}
