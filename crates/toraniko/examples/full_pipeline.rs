//! Example: Full Toraniko Factor Model Pipeline
//!
//! This comprehensive example demonstrates the complete toraniko-rs workflow:
//! 1. Fetching real market data from Yahoo Finance
//! 2. Computing all three style factors (momentum, value, size)
//! 3. Running the factor returns estimator
//! 4. Printing nicely formatted output tables
//!
//! Run with: `cargo run --example full_pipeline --features full`
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

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Number of trading days of historical data to fetch (~2 years)
const TRADING_DAYS: i64 = 504;

/// Stock universe organized by sector
const TECH_STOCKS: &[&str] =
    &["AAPL", "MSFT", "GOOGL", "META", "NVDA", "AMD", "INTC", "CRM", "ADBE", "ORCL"];
const HEALTHCARE_STOCKS: &[&str] =
    &["JNJ", "UNH", "PFE", "MRK", "ABBV", "TMO", "ABT", "LLY", "BMY", "AMGN"];
const FINANCE_STOCKS: &[&str] =
    &["JPM", "BAC", "WFC", "GS", "MS", "C", "BLK", "SCHW", "AXP", "USB"];

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_header();

    // Step 1: Fetch data from Yahoo Finance
    let data = fetch_yahoo_data().await?;
    print_input_data_sample(&data)?;

    // Step 2: Compute style factors
    let style_scores = compute_style_factors(&data)?;
    print_style_scores_sample(&style_scores)?;

    // Step 3: Prepare sector data (one-hot encoded)
    let sector_df = prepare_sector_data(&data)?;

    // Step 4: Run factor returns estimation
    let (factor_returns, residuals) = estimate_factor_returns(&data, &sector_df, &style_scores)?;

    // Step 5: Print results
    print_factor_returns(&factor_returns)?;
    print_residuals_sample(&residuals)?;
    print_summary_statistics(&factor_returns)?;

    print_footer(&data);

    Ok(())
}

// ============================================================================
// DATA FETCHING FROM YAHOO FINANCE
// ============================================================================

/// Combined input data structure
struct InputData {
    returns: LazyFrame,
    market_caps: LazyFrame,
    fundamentals: LazyFrame,
    raw_df: DataFrame,
    start_date: time::Date,
    end_date: time::Date,
    n_stocks: usize,
}

/// Fetch real market data from Yahoo Finance
async fn fetch_yahoo_data() -> Result<InputData, Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         FETCHING DATA FROM YAHOO FINANCE\n\
         ================================================================================\n"
    );

    let provider = yahoo::YahooConnector::new()?;

    // Fetch historical data
    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(TRADING_DAYS);

    println!("[*] Fetching data from {} to {}\n", start.date(), end.date());

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
                        println!("    {} - {} quotes fetched", symbol, quotes.len());
                        stock_data.insert(symbol.to_string(), quotes);
                    } else {
                        println!("    {} - no quotes available", symbol);
                        failed_symbols.push(symbol.to_string());
                    }
                }
            }
            Err(e) => {
                println!("    {} - failed: {}", symbol, e);
                failed_symbols.push(symbol.to_string());
            }
        }
    }

    println!("\n[+] Successfully fetched data for {} stocks", stock_data.len());
    if !failed_symbols.is_empty() {
        println!("[!] Failed to fetch: {:?}\n", failed_symbols);
    }

    // Build DataFrames from fetched data
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

            // Use timestamp as date
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
            let high = quotes[i].high;
            let low = quotes[i].low;
            book_price_proxy.push(low / close); // Low/Close as book proxy
            sales_price_proxy.push((high - low) / close); // Range/Close as sales proxy
            cf_price_proxy.push(volume / (close * 1e6)); // Volume/Price as CF proxy
        }
    }

    println!("[*] Total observations: {}\n", dates.len());

    // Convert timestamps (seconds since epoch) to milliseconds for datetime
    let dates_ms: Vec<i64> = dates.iter().map(|t| t * 1000).collect();
    let dates_series = Series::new("timestamp".into(), &dates_ms);
    let dates_datetime = dates_series.cast(&DataType::Datetime(TimeUnit::Milliseconds, None))?;
    let dates_col = dates_datetime.cast(&DataType::Date)?;

    // Create combined raw DataFrame
    let raw_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("sector".into(), sectors.clone()),
        Column::new("asset_returns".into(), returns.clone()),
        Column::new("market_cap".into(), market_caps.clone()),
        Column::new("book_price".into(), book_price_proxy.clone()),
        Column::new("sales_price".into(), sales_price_proxy.clone()),
        Column::new("cf_price".into(), cf_price_proxy.clone()),
    ])?;

    // Create returns DataFrame
    let returns_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("asset_returns".into(), returns),
    ])?;

    // Create market cap DataFrame
    let mkt_cap_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), symbols.clone()),
        Column::new("market_cap".into(), market_caps),
    ])?;

    // Create fundamentals DataFrame for Value factor
    let fundamentals_df = DataFrame::new(vec![
        dates_col.with_name("date".into()).into(),
        Column::new("symbol".into(), symbols),
        Column::new("book_price".into(), book_price_proxy),
        Column::new("sales_price".into(), sales_price_proxy),
        Column::new("cf_price".into(), cf_price_proxy),
    ])?;

    Ok(InputData {
        returns: returns_df.lazy(),
        market_caps: mkt_cap_df.lazy(),
        fundamentals: fundamentals_df.lazy(),
        raw_df,
        start_date: start.date(),
        end_date: end.date(),
        n_stocks: stock_data.len(),
    })
}

// ============================================================================
// STYLE FACTOR COMPUTATION
// ============================================================================

/// Compute all style factor scores
fn compute_style_factors(data: &InputData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         STEP 2: COMPUTING STYLE FACTORS\n\
         ================================================================================\n"
    );

    // Initialize factor registry
    let registry = FactorRegistry::with_defaults();

    // Get a reference date from the data for factor computation
    let dates_col = data.raw_df.column("date")?;
    let first_date = dates_col.get(0)?;
    // Convert from polars Date to chrono NaiveDate
    let date_i32 = first_date.try_extract::<i32>()?;
    let date = NaiveDate::from_num_days_from_ce_opt(date_i32).ok_or("Invalid date")?;

    // SIZE FACTOR
    println!("[*] Computing SIZE factor (Small-Minus-Big)...");
    let size_factor = registry.get("market_cap").ok_or("Size factor not found")?;
    println!("    Factor: {}", size_factor.name());
    println!("    Description: {}", size_factor.description());

    let size_scores = size_factor.compute(&data.market_caps.clone(), date)?;
    println!("    Computed {} size scores\n", size_scores.height());

    // VALUE FACTOR
    println!("[*] Computing VALUE factor (composite of B/P, S/P, CF/P proxies)...");
    let value_factor = registry.get("book_to_market").ok_or("Value factor not found")?;
    println!("    Factor: {}", value_factor.name());
    println!("    Description: {}", value_factor.description());

    let value_scores = value_factor.compute(&data.fundamentals.clone(), date)?;
    println!("    Computed {} value scores\n", value_scores.height());

    // MOMENTUM FACTOR
    println!("[*] Computing MOMENTUM factor (trailing returns with lag)...");
    let momentum_factor = registry.get("long_term_momentum").ok_or("Momentum factor not found")?;
    println!("    Factor: {}", momentum_factor.name());
    println!("    Description: {}", momentum_factor.description());

    let momentum_scores = momentum_factor.compute(&data.returns.clone(), date)?;
    println!(
        "    Computed {} momentum scores (some null due to lookback)\n",
        momentum_scores.height()
    );

    // Join all scores together
    let combined = size_scores
        .lazy()
        .join(
            value_scores.lazy(),
            [col("date"), col("symbol")],
            [col("date"), col("symbol")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            momentum_scores.lazy(),
            [col("date"), col("symbol")],
            [col("date"), col("symbol")],
            JoinArgs::new(JoinType::Inner),
        )
        .collect()?;

    println!(
        "[+] Combined style scores: {} rows x {} columns\n",
        combined.height(),
        combined.width()
    );

    Ok(combined)
}

/// Prepare one-hot encoded sector data from the raw data
fn prepare_sector_data(data: &InputData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    // Get unique sectors
    let sectors: Vec<String> = data
        .raw_df
        .column("sector")?
        .str()?
        .into_iter()
        .filter_map(|s| s.map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Create one-hot encoded columns
    let mut lf = data.raw_df.clone().lazy().select([col("date"), col("symbol"), col("sector")]);

    for sector in &sectors {
        lf = lf.with_column(
            when(col("sector").eq(lit(sector.as_str())))
                .then(lit(1.0))
                .otherwise(lit(0.0))
                .alias(format!("sector_{sector}")),
        );
    }

    // Drop the original sector column
    let sector_df = lf.select([col("*").exclude(["sector"])]).collect()?;

    Ok(sector_df)
}

// ============================================================================
// FACTOR RETURNS ESTIMATION
// ============================================================================

/// Run the factor returns estimation
fn estimate_factor_returns(
    data: &InputData,
    sector_df: &DataFrame,
    style_scores: &DataFrame,
) -> Result<(DataFrame, DataFrame), Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         STEP 3: ESTIMATING FACTOR RETURNS\n\
         ================================================================================\n"
    );

    // Configure the estimator
    let config = EstimatorConfig { winsor_factor: Some(0.05), residualize_styles: true };

    let estimator = FactorReturnsEstimator::with_config(config);

    println!("[*] Factor Returns Estimator Configuration:");
    println!("    - Winsorization: {:?}", estimator.winsor_factor());
    println!("    - Residualize styles to sectors: {}\n", estimator.residualize_styles());

    // Run estimation
    println!("[*] Running cross-sectional regression for each date...");

    let (factor_returns, residuals) = estimator.estimate(
        data.returns.clone(),
        data.market_caps.clone(),
        sector_df.clone().lazy(),
        style_scores.clone().lazy(),
    )?;

    let n_dates = factor_returns.column("date")?.unique()?.len();

    println!("[+] Estimation complete: {} dates processed\n", n_dates);

    Ok((factor_returns, residuals))
}

// ============================================================================
// OUTPUT FORMATTING
// ============================================================================

fn print_header() {
    println!(
        "\n\
        ################################################################################\n\
        #                                                                              #\n\
        #                    TORANIKO-RS: FULL PIPELINE EXAMPLE                        #\n\
        #                                                                              #\n\
        #  A Rust implementation of the toraniko factor model for equity risk          #\n\
        #  Using real market data from Yahoo Finance                                   #\n\
        #                                                                              #\n\
        ################################################################################\n"
    );
}

fn print_footer(data: &InputData) {
    println!(
        "\n\
        ################################################################################\n\
        #                                                                              #\n\
        #                         PIPELINE COMPLETE                                    #\n\
        #                                                                              #\n\
        #  The factor model has decomposed asset returns into:                         #\n\
        #    - Market return (cap-weighted average)                                    #\n\
        #    - Sector returns (constrained to sum to zero)                             #\n\
        #    - Style factor returns (momentum, value, size)                            #\n\
        #    - Idiosyncratic residuals (stock-specific returns)                        #\n\
        #                                                                              #\n\
        #  Use cases:                                                                  #\n\
        #    - Factor covariance matrix for portfolio optimization                     #\n\
        #    - Risk attribution and factor exposure analysis                           #\n\
        #    - Market-neutral portfolio construction                                   #\n\
        #                                                                              #\n\
        ################################################################################\n"
    );
    println!("=== Data Source ===");
    println!("All data fetched from Yahoo Finance");
    println!("Universe: {} stocks across 3 sectors", data.n_stocks);
    println!("Period: {} to {}\n", data.start_date, data.end_date);
}

fn print_input_data_sample(data: &InputData) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         STEP 1: INPUT DATA\n\
         ================================================================================\n"
    );

    let sample = data.raw_df.head(Some(10));

    println!("[*] Raw input data sample (first 10 rows):\n");
    println!("{sample}\n");

    println!(
        "[*] Data dimensions: {} rows x {} columns",
        data.raw_df.height(),
        data.raw_df.width()
    );

    let n_dates = data.raw_df.column("date")?.unique()?.len();
    let n_symbols = data.raw_df.column("symbol")?.unique()?.len();

    println!("[*] Universe: {} unique dates, {} unique symbols\n", n_dates, n_symbols);

    Ok(())
}

fn print_style_scores_sample(scores: &DataFrame) -> Result<(), Box<dyn std::error::Error>> {
    // Get a sample: first 15 rows
    let sample = scores.head(Some(15));

    println!("[*] Style factor scores sample (first 15 rows):\n");
    println!("{sample}\n");

    // Show score distributions
    println!("[*] Style score summary statistics:\n");

    for col_name in ["market_cap", "book_to_market", "long_term_momentum"] {
        if let Ok(col) = scores.column(col_name)
            && let Ok(f64_col) = col.f64()
        {
            let mean = f64_col.mean().unwrap_or(f64::NAN);
            let std = f64_col.std(1).unwrap_or(f64::NAN);
            let min = f64_col.min().unwrap_or(f64::NAN);
            let max = f64_col.max().unwrap_or(f64::NAN);
            let null_count = f64_col.null_count();

            println!(
                "    {col_name:20} | mean: {:>8.4} | std: {:>7.4} | min: {:>8.4} | max: {:>7.4} | nulls: {}",
                mean, std, min, max, null_count
            );
        }
    }
    println!();

    Ok(())
}

fn print_factor_returns(factor_returns: &DataFrame) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         STEP 4: ESTIMATED FACTOR RETURNS\n\
         ================================================================================\n"
    );

    // Show sample of factor returns
    let sample = factor_returns.head(Some(20));
    println!("[*] Factor returns sample (first 20 rows):\n");
    println!("{sample}\n");

    // Pivot to show by factor
    println!("[*] Factor returns by date (last 5 dates, pivoted view):\n");

    let pivoted = factor_returns
        .clone()
        .lazy()
        .group_by([col("date")])
        .agg([col("factor"), col("factor_return")])
        .sort(["date"], SortMultipleOptions::new().with_order_descending(true))
        .limit(5)
        .collect()?;

    println!("{pivoted}\n");

    Ok(())
}

fn print_residuals_sample(residuals: &DataFrame) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "--------------------------------------------------------------------------------\n\
         IDIOSYNCRATIC RESIDUALS\n\
         --------------------------------------------------------------------------------\n"
    );

    let sample = residuals.head(Some(15));
    println!("[*] Residual returns sample (first 15 rows):\n");
    println!("{sample}\n");

    // Residual statistics
    if let Ok(col) = residuals.column("residual_return")
        && let Ok(f64_col) = col.f64()
    {
        let mean = f64_col.mean().unwrap_or(f64::NAN);
        let std = f64_col.std(1).unwrap_or(f64::NAN);
        let min = f64_col.min().unwrap_or(f64::NAN);
        let max = f64_col.max().unwrap_or(f64::NAN);

        println!("[*] Residual statistics:");
        println!("    mean: {:>10.6} (should be ~0)", mean);
        println!("    std:  {:>10.6}", std);
        println!("    min:  {:>10.6}", min);
        println!("    max:  {:>10.6}\n", max);
    }

    Ok(())
}

fn print_summary_statistics(factor_returns: &DataFrame) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "================================================================================\n\
         STEP 5: SUMMARY STATISTICS\n\
         ================================================================================\n"
    );

    // Compute per-factor statistics
    let stats = factor_returns
        .clone()
        .lazy()
        .group_by([col("factor")])
        .agg([
            col("factor_return").mean().alias("mean_return"),
            col("factor_return").std(1).alias("std_return"),
            col("factor_return").min().alias("min_return"),
            col("factor_return").max().alias("max_return"),
            col("factor_return").count().alias("n_obs"),
        ])
        .sort(["factor"], Default::default())
        .collect()?;

    println!("[*] Factor return statistics (annualized estimates):\n");
    println!("{stats}\n");

    // Compute annualized metrics (assuming daily data, ~252 trading days)
    println!("[*] Annualized factor premiums (assuming 252 trading days/year):\n");
    println!("    {:20} | {:>12} | {:>12} | {:>12}", "Factor", "Ann. Return", "Ann. Vol", "Sharpe");
    println!("    {:-<20}-+-{:-^12}-+-{:-^12}-+-{:-^12}", "", "", "", "");

    let factors = stats.column("factor")?.str()?;
    let means = stats.column("mean_return")?.f64()?;
    let stds = stats.column("std_return")?.f64()?;

    for i in 0..stats.height() {
        let factor = factors.get(i).unwrap_or("?");
        let mean = means.get(i).unwrap_or(f64::NAN);
        let std = stds.get(i).unwrap_or(f64::NAN);

        let ann_return = mean * 252.0 * 100.0; // Annualized, in percent
        let ann_vol = std * (252.0_f64).sqrt() * 100.0; // Annualized volatility
        let sharpe = if ann_vol > 0.0 { ann_return / ann_vol } else { f64::NAN };

        println!(
            "    {:20} | {:>10.2}% | {:>10.2}% | {:>12.2}",
            factor, ann_return, ann_vol, sharpe
        );
    }

    println!();

    Ok(())
}
