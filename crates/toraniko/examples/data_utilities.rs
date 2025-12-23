//! Example: Data Utility Functions with Real Market Data
//!
//! This example demonstrates the data utility functions available in toraniko
//! using real Yahoo Finance data:
//! - `fill_features`: Forward-fill missing values within groups
//! - `smooth_features`: Apply rolling mean smoothing
//! - `top_n_by_group`: Select top N assets by a metric per group
//!
//! Based on the original Python toraniko implementation:
//! <https://github.com/0xfdf/toraniko>

use std::collections::HashMap;

use polars::prelude::*;
use time::{Duration, OffsetDateTime};
use toraniko::utils::{fill_features, smooth_features, top_n_by_group};
use yahoo_finance_api as yahoo;

/// Broad stock universe for demonstrating filtering
const STOCKS: &[&str] = &[
    "AAPL", "MSFT", "GOOGL", "AMZN", "NVDA", "META", "TSLA", "AMD", "INTC", "CRM", "JPM", "BAC",
    "GS", "JNJ", "PFE", "UNH",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Data Utilities with Yahoo Finance Data ===\n");

    // =========================================================================
    // FETCH DATA FROM YAHOO FINANCE
    // =========================================================================

    let provider = yahoo::YahooConnector::new()?;

    // Fetch 3 months of daily data
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(90);

    println!("Fetching data from {} to {}\n", start.date(), end.date());

    let mut stock_data: HashMap<String, Vec<yahoo::Quote>> = HashMap::new();

    for symbol in STOCKS {
        match provider.get_quote_history(symbol, start, end).await {
            Ok(response) => {
                if let Ok(quotes) = response.quotes()
                    && !quotes.is_empty()
                {
                    println!("  {} - {} quotes", symbol, quotes.len());
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
    // BUILD DATAFRAME
    // =========================================================================

    let mut dates: Vec<i64> = Vec::new();
    let mut symbols: Vec<String> = Vec::new();
    let mut market_caps: Vec<f64> = Vec::new();
    let mut prices: Vec<f64> = Vec::new();
    let mut volumes: Vec<f64> = Vec::new();
    let mut daily_ranges: Vec<f64> = Vec::new();

    for (symbol, quotes) in &stock_data {
        for quote in quotes {
            dates.push(quote.timestamp);
            symbols.push(symbol.clone());

            let close = quote.close;
            let volume = quote.volume as f64;
            prices.push(close);
            volumes.push(volume);
            market_caps.push(volume * close); // Proxy for market cap
            daily_ranges.push((quote.high - quote.low) / close); // Normalized range
        }
    }

    // Convert timestamps to dates
    let dates_ms: Vec<i64> = dates.iter().map(|t| t * 1000).collect();
    let dates_series = Series::new("timestamp".into(), &dates_ms);
    let dates_datetime = dates_series.cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?;
    let dates_col = dates_datetime.cast(&DataType::Date)?;

    let full_df = DataFrame::new(vec![
        dates_col.with_name("date".into()).into(),
        Column::new("symbol".into(), symbols),
        Column::new("market_cap".into(), market_caps),
        Column::new("price".into(), prices),
        Column::new("volume".into(), volumes),
        Column::new("daily_range".into(), daily_ranges),
    ])?;

    println!("Built DataFrame with {} rows\n", full_df.height());

    // =========================================================================
    // UNIVERSE SELECTION: top_n_by_group
    // =========================================================================
    println!("=== Universe Selection: top_n_by_group ===\n");

    println!("Full universe sample (first 10 rows):");
    println!("{}\n", full_df.head(Some(10)));

    // Select top 5 stocks by market cap for each date
    // This is useful for defining a tradeable universe (e.g., top 1000, 3000 stocks)
    let top_5 = top_n_by_group(full_df.clone().lazy(), 5, "market_cap", &["date"], true);

    let top_5_df = top_5.collect()?;
    println!("Top 5 by market cap per date (first 15 rows):");
    println!("{}\n", top_5_df.head(Some(15)));

    // Count unique symbols in filtered universe
    let unique_in_top5 = top_5_df.column("symbol")?.unique()?.len();
    println!(
        "Unique symbols in top-5 universe: {} (out of {} total)\n",
        unique_in_top5,
        stock_data.len()
    );

    // =========================================================================
    // FORWARD FILL: fill_features
    // =========================================================================
    println!("=== Forward Fill: fill_features ===\n");

    // Create data with some artificial missing values for demonstration
    // In real usage, this handles gaps in fundamental data (earnings, book value, etc.)
    let with_nulls = full_df
        .clone()
        .lazy()
        .with_row_index("row_idx", None)
        .with_column(
            // Simulate missing volume data (set every 5th row to null)
            when(col("row_idx").cast(DataType::Int64) % lit(5i64).eq(lit(0i64)))
                .then(lit(NULL).cast(DataType::Float64))
                .otherwise(col("volume"))
                .alias("volume_with_gaps"),
        )
        .select([col("*").exclude(["row_idx"])])
        .collect()?;

    println!("Data with simulated gaps (first 15 rows):");
    println!(
        "{}\n",
        with_nulls
            .clone()
            .lazy()
            .select([col("date"), col("symbol"), col("volume"), col("volume_with_gaps")])
            .limit(15)
            .collect()?
    );

    // Count nulls before fill
    let null_count_before = with_nulls.column("volume_with_gaps")?.null_count();
    println!("Null values before fill: {}", null_count_before);

    // Forward-fill the features within each symbol group
    let filled =
        fill_features(with_nulls.lazy(), &["volume_with_gaps"], "date", "symbol").collect()?;

    let null_count_after = filled.column("volume_with_gaps")?.null_count();
    println!("Null values after fill: {}\n", null_count_after);

    println!("After forward-fill (first 15 rows):");
    println!(
        "{}\n",
        filled
            .lazy()
            .select([col("date"), col("symbol"), col("volume"), col("volume_with_gaps")])
            .limit(15)
            .collect()?
    );

    // =========================================================================
    // SMOOTHING: smooth_features
    // =========================================================================
    println!("=== Rolling Mean Smoothing: smooth_features ===\n");

    // Smooth the daily range (which can be noisy) using a 5-day rolling mean
    println!("Applying 5-day rolling mean smoothing to daily_range...\n");

    let smoothed = smooth_features(
        full_df.lazy(),
        &["daily_range"],
        "date",
        "symbol",
        5, // 5-day window
    )
    .collect()?;

    // Show before/after for a single symbol
    let aapl_comparison = smoothed
        .clone()
        .lazy()
        .filter(col("symbol").eq(lit("AAPL")))
        .select([
            col("date"),
            col("symbol"),
            col("daily_range").alias("raw_range"),
            col("daily_range_smooth").alias("smoothed_range"),
        ])
        .sort(["date"], Default::default())
        .limit(15)
        .collect()?;

    println!("AAPL daily range: raw vs smoothed (first 15 days):");
    println!("{}\n", aapl_comparison);

    // Show statistics
    if let (Ok(raw_col), Ok(smooth_col)) =
        (smoothed.column("daily_range"), smoothed.column("daily_range_smooth"))
        && let (Ok(raw_f64), Ok(smooth_f64)) = (raw_col.f64(), smooth_col.f64())
    {
        let raw_std = raw_f64.std(1).unwrap_or(0.0);
        let smooth_std = smooth_f64.std(1).unwrap_or(0.0);
        println!(
            "Volatility reduction: raw std={:.6}, smoothed std={:.6} ({:.1}% reduction)\n",
            raw_std,
            smooth_std,
            (1.0 - smooth_std / raw_std) * 100.0
        );
    }

    println!("=== Data Source ===");
    println!("All data fetched from Yahoo Finance");
    println!("Period: {} to {}", start.date(), end.date());

    Ok(())
}
