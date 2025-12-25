//! Factor attribution analysis CLI tool.
//!
//! Analyzes which factors attribute to an individual equity's performance.
//!
//! Usage: `cargo run --bin analyze -- SYMBOL [--years N]`
//! Example: `cargo run --bin analyze -- UNH --years 5`

use std::{collections::HashMap, env};

use chrono::NaiveDate;
use factors::FactorRegistry;
use polars::prelude::*;
use time::{Duration, OffsetDateTime};
use toraniko::{
    model::{EstimatorConfig, FactorReturnsEstimator, compute_attribution},
    traits::ReturnsEstimator,
};
use yahoo_finance_api as yahoo;

/// Default analysis period in years.
const DEFAULT_YEARS: i64 = 5;

/// Trading days per year.
const TRADING_DAYS_PER_YEAR: i64 = 252;

/// Reference universe organized by sector (expanded for better factor estimation).
const TECH_STOCKS: &[&str] =
    &["AAPL", "MSFT", "GOOGL", "META", "NVDA", "AMD", "INTC", "CRM", "ADBE", "ORCL"];
const HEALTHCARE_STOCKS: &[&str] =
    &["JNJ", "UNH", "PFE", "MRK", "ABBV", "TMO", "ABT", "LLY", "BMY", "AMGN"];
const FINANCE_STOCKS: &[&str] =
    &["JPM", "BAC", "WFC", "GS", "MS", "C", "BLK", "SCHW", "AXP", "USB"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: analyze SYMBOL [--years N]");
        eprintln!("Example: analyze UNH --years 5");
        std::process::exit(1);
    }

    let symbol = args[1].to_uppercase();
    let years = parse_years(&args);

    println!("\nAnalyzing {} over {} year(s)...\n", symbol, years);

    // Run the analysis
    match run_attribution_analysis(&symbol, years).await {
        Ok(result) => {
            result.print_summary();
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn parse_years(args: &[String]) -> i64 {
    for i in 0..args.len() {
        if args[i] == "--years" && i + 1 < args.len() {
            if let Ok(y) = args[i + 1].parse::<i64>() {
                return y;
            }
        }
    }
    DEFAULT_YEARS
}

/// Main attribution analysis function.
async fn run_attribution_analysis(
    target_symbol: &str,
    years: i64,
) -> Result<toraniko::model::AttributionResult, Box<dyn std::error::Error>> {
    // Build universe including target symbol
    let mut all_stocks: Vec<&str> = TECH_STOCKS
        .iter()
        .chain(HEALTHCARE_STOCKS.iter())
        .chain(FINANCE_STOCKS.iter())
        .copied()
        .collect();

    // Add target symbol if not already in universe
    let target_in_universe = all_stocks.iter().any(|s| s.eq_ignore_ascii_case(target_symbol));
    if !target_in_universe {
        all_stocks.push(Box::leak(target_symbol.to_string().into_boxed_str()));
    }

    // Determine sector for target
    let target_sector = if TECH_STOCKS.iter().any(|s| s.eq_ignore_ascii_case(target_symbol)) {
        "Technology"
    } else if HEALTHCARE_STOCKS.iter().any(|s| s.eq_ignore_ascii_case(target_symbol)) {
        "Healthcare"
    } else if FINANCE_STOCKS.iter().any(|s| s.eq_ignore_ascii_case(target_symbol)) {
        "Finance"
    } else {
        "Other"
    };

    // Fetch data
    let trading_days = years * TRADING_DAYS_PER_YEAR;
    let data = fetch_data(&all_stocks, trading_days, target_symbol, target_sector).await?;

    // Compute style factors
    let style_scores = compute_style_factors(&data)?;

    // Prepare sector data
    let sector_df = prepare_sector_data(&data)?;

    // Estimate factor returns
    let config = EstimatorConfig { winsor_factor: Some(0.05), residualize_styles: true };
    let estimator = FactorReturnsEstimator::with_config(config);

    let (factor_returns, residuals) = estimator.estimate(
        data.returns.clone(),
        data.market_caps.clone(),
        sector_df.clone().lazy(),
        style_scores.clone().lazy(),
    )?;

    // Compute attribution for target symbol
    let attribution =
        compute_attribution(target_symbol, &factor_returns, &residuals, &style_scores, &sector_df)?;

    Ok(attribution)
}

/// Input data structure.
struct InputData {
    returns: LazyFrame,
    market_caps: LazyFrame,
    fundamentals: LazyFrame,
    raw_df: DataFrame,
}

/// Fetch market data from Yahoo Finance.
async fn fetch_data(
    symbols: &[&str],
    trading_days: i64,
    target_symbol: &str,
    target_sector: &str,
) -> Result<InputData, Box<dyn std::error::Error>> {
    let provider = yahoo::YahooConnector::new()?;

    let end = OffsetDateTime::now_utc();
    let start = end - Duration::days(trading_days);

    print!("Fetching data for {} stocks", symbols.len());

    let mut stock_data: HashMap<String, Vec<yahoo::Quote>> = HashMap::new();
    let mut count = 0;

    for symbol in symbols {
        match provider.get_quote_history(symbol, start, end).await {
            Ok(response) => {
                if let Ok(quotes) = response.quotes() {
                    if !quotes.is_empty() {
                        stock_data.insert(symbol.to_string(), quotes);
                        count += 1;
                        if count % 10 == 0 {
                            print!(".");
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }
    println!(" done ({} stocks loaded)", stock_data.len());

    // Build DataFrames
    let mut dates: Vec<i64> = Vec::new();
    let mut syms: Vec<String> = Vec::new();
    let mut returns: Vec<f64> = Vec::new();
    let mut market_caps: Vec<f64> = Vec::new();
    let mut sectors: Vec<String> = Vec::new();
    let mut book_price_proxy: Vec<f64> = Vec::new();
    let mut sales_price_proxy: Vec<f64> = Vec::new();
    let mut cf_price_proxy: Vec<f64> = Vec::new();

    for (symbol, quotes) in &stock_data {
        let sector = if TECH_STOCKS.contains(&symbol.as_str()) {
            "Technology"
        } else if HEALTHCARE_STOCKS.contains(&symbol.as_str()) {
            "Healthcare"
        } else if FINANCE_STOCKS.contains(&symbol.as_str()) {
            "Finance"
        } else if symbol.eq_ignore_ascii_case(target_symbol) {
            target_sector
        } else {
            "Other"
        };

        for i in 1..quotes.len() {
            let prev_close = quotes[i - 1].adjclose;
            let curr_close = quotes[i].adjclose;
            let daily_return = (curr_close - prev_close) / prev_close;

            dates.push(quotes[i].timestamp);
            syms.push(symbol.clone());
            returns.push(daily_return);
            sectors.push(sector.to_string());

            let volume = quotes[i].volume as f64;
            let close = quotes[i].close;
            market_caps.push(volume * close);

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

    let raw_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), syms.clone()),
        Column::new("sector".into(), sectors.clone()),
        Column::new("asset_returns".into(), returns.clone()),
        Column::new("market_cap".into(), market_caps.clone()),
        Column::new("book_price".into(), book_price_proxy.clone()),
        Column::new("sales_price".into(), sales_price_proxy.clone()),
        Column::new("cf_price".into(), cf_price_proxy.clone()),
    ])?;

    let returns_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), syms.clone()),
        Column::new("asset_returns".into(), returns),
    ])?;

    let mkt_cap_df = DataFrame::new(vec![
        dates_col.clone().with_name("date".into()).into(),
        Column::new("symbol".into(), syms.clone()),
        Column::new("market_cap".into(), market_caps),
    ])?;

    let fundamentals_df = DataFrame::new(vec![
        dates_col.with_name("date".into()).into(),
        Column::new("symbol".into(), syms),
        Column::new("book_price".into(), book_price_proxy),
        Column::new("sales_price".into(), sales_price_proxy),
        Column::new("cf_price".into(), cf_price_proxy),
    ])?;

    Ok(InputData {
        returns: returns_df.lazy(),
        market_caps: mkt_cap_df.lazy(),
        fundamentals: fundamentals_df.lazy(),
        raw_df,
    })
}

/// Compute style factors.
fn compute_style_factors(data: &InputData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    // Initialize factor registry with defaults
    let registry = FactorRegistry::with_defaults();

    // Get a sample date from the data for factor computation
    let dates_col = data.raw_df.column("date")?;
    let first_date = dates_col.get(0)?;
    // Convert from polars Date to chrono NaiveDate
    let date_i32 = first_date.try_extract::<i32>()?;
    let date = NaiveDate::from_num_days_from_ce_opt(date_i32).ok_or("Invalid date")?;

    // Get factors from registry
    let size_factor = registry.get("market_cap").ok_or("Size factor not found")?;
    let value_factor = registry.get("book_to_market").ok_or("Value factor not found")?;
    let momentum_factor = registry.get("long_term_momentum").ok_or("Momentum factor not found")?;

    // Compute factor scores
    let size_scores = size_factor.compute(&data.market_caps.clone(), date)?;
    let value_scores = value_factor.compute(&data.fundamentals.clone(), date)?;
    let momentum_scores = momentum_factor.compute(&data.returns.clone(), date)?;

    // Join all scores
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

    Ok(combined)
}

/// Prepare one-hot encoded sector data.
fn prepare_sector_data(data: &InputData) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let sectors: Vec<String> = data
        .raw_df
        .column("sector")?
        .str()?
        .into_iter()
        .filter_map(|s| s.map(|s| s.to_string()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut lf = data.raw_df.clone().lazy().select([col("date"), col("symbol"), col("sector")]);

    for sector in &sectors {
        lf = lf.with_column(
            when(col("sector").eq(lit(sector.as_str())))
                .then(lit(1.0))
                .otherwise(lit(0.0))
                .alias(format!("sector_{sector}")),
        );
    }

    let sector_df = lf.select([col("*").exclude(["sector"])]).collect()?;

    Ok(sector_df)
}
