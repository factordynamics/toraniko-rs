//! Example: Computing Style Factor Scores with Real Market Data
//!
//! This example demonstrates how to compute momentum, value, and size
//! factor scores for a universe of equities using real Yahoo Finance data.
//!
//! Based on the original Python toraniko implementation:
//! <https://github.com/0xfdf/toraniko>

use std::collections::HashMap;

use chrono::NaiveDate;
use factors::FactorRegistry;
use polars::prelude::*;
use time::{Duration, OffsetDateTime};
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

    // Initialize factor registry
    let registry = FactorRegistry::with_defaults();

    // Get a reference date from the data for factor computation
    let dates_col = returns_df.column("date")?;
    let first_date = dates_col.get(0)?;
    // Convert from polars Date to chrono NaiveDate
    let date_i32 = first_date.try_extract::<i32>()?;
    let date = NaiveDate::from_num_days_from_ce_opt(date_i32).ok_or("Invalid date")?;

    // =========================================================================
    // MOMENTUM FACTOR
    // =========================================================================
    println!("=== Momentum Factor ===\n");

    let momentum = registry.get("long_term_momentum").ok_or("Momentum factor not found")?;

    println!("Factor name: {}", momentum.name());
    println!("Factor category: {:?}", momentum.category());
    println!("Description: {}", momentum.description());

    let momentum_scores = momentum.compute(&returns_df.lazy(), date)?;

    println!("\nMomentum scores (sample - last 10 rows):");
    println!("{}\n", momentum_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = momentum_scores.column("long_term_momentum")
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

    let size = registry.get("market_cap").ok_or("Size factor not found")?;

    println!("Factor name: {}", size.name());
    println!("Factor category: {:?}", size.category());
    println!("Description: {}", size.description());

    let size_scores = size.compute(&mkt_cap_df.lazy(), date)?;

    println!("\nSize scores (sample - last 10 rows):");
    println!("(negative = large cap, positive = small cap)");
    println!("{}\n", size_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = size_scores.column("market_cap")
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

    let value = registry.get("book_to_market").ok_or("Value factor not found")?;

    println!("Factor name: {}", value.name());
    println!("Factor category: {:?}", value.category());
    println!("Description: {}", value.description());

    let value_scores = value.compute(&value_df.lazy(), date)?;

    println!("\nValue scores (sample - last 10 rows):");
    println!("(composite of book/price, sales/price, cf/price proxies)");
    println!("{}\n", value_scores.tail(Some(10)));

    // Print statistics
    if let Ok(score_col) = value_scores.column("book_to_market")
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
