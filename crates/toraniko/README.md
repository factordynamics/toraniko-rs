# Toraniko: Institutional-Scale Risk Model

A high-performance Rust implementation of a characteristic factor model suitable for quantitative and systematic trading at institutional scale. In particular, it is a characteristic factor model in the same vein as Barra and Axioma (in fact, given the same datasets, it approximately reproduces Barra's estimated factor returns).

[![Crates.io](https://img.shields.io/crates/v/toraniko.svg)](https://crates.io/crates/toraniko)
[![Documentation](https://docs.rs/toraniko/badge.svg)](https://docs.rs/toraniko)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

Using this library, you can create new custom factors and estimate their returns. Then you can estimate a factor covariance matrix suitable for portfolio optimization with factor exposure constraints (e.g. to maintain a market neutral portfolio).

The library supports market, sector and style factors; three styles are included: value, size and momentum. It also comes with generalizable math and data cleaning utility functions you'd want to have for constructing more style factors (or custom fundamental factors of any kind).

### Mathematical Model

The factor model decomposes asset returns as:

```text
r_asset = beta_market * r_market + sum(beta_sector * r_sector) + sum(beta_style * r_style) + epsilon
```

Where:
- `r_market` is the market factor return (cap-weighted average)
- `r_sector` are sector factor returns (constrained to sum to zero, Barra-style)
- `r_style` are style factor returns (momentum, value, size)
- `epsilon` is the idiosyncratic/residual return

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
toraniko = "0.1"
```

Or with specific features:

```toml
[dependencies]
toraniko = { version = "0.1", default-features = false, features = ["model"] }
factors = "0.1"  # For style factor implementations
```

### Available Features

| Feature | Description |
|---------|-------------|
| `full` (default) | All features enabled |
| `primitives` | Core type definitions |
| `traits` | Trait abstractions (Factor, StyleFactor, ReturnsEstimator) |
| `math` | Mathematical operations (WLS, winsorization, weights) |
| `model` | Factor return estimation |
| `utils` | Data utilities (fill, smooth, rank) |

Note: Style factor implementations (Momentum, Size, Value) are now provided by the separate `factors` crate.

## Quick Start with Real Market Data

The `yahoo_finance` example demonstrates fetching real market data and running the full factor model pipeline:

```bash
cargo run --package toraniko --example yahoo_finance
```

This fetches ~500 days of daily data for 30 stocks across Technology, Healthcare, and Finance sectors directly from Yahoo Finance.

### Example Output (Real Data from Yahoo Finance)

```text
=== Toraniko Factor Model with Real Yahoo Finance Data ===

Fetching data from 2024-08-06 to 2025-12-23
  AAPL - 347 quotes fetched
  MSFT - 347 quotes fetched
  GOOGL - 347 quotes fetched
  META - 347 quotes fetched
  ...

Successfully fetched data for 30 stocks
```

**Returns DataFrame (real stock returns):**

```text
┌────────────┬────────┬────────────┬───────────────┐
│ date       ┆ symbol ┆ sector     ┆ asset_returns │
│ ---        ┆ ---    ┆ ---        ┆ ---           │
│ date       ┆ str    ┆ str        ┆ f64           │
╞════════════╪════════╪════════════╪═══════════════╡
│ 2024-08-07 ┆ UNH    ┆ Healthcare ┆ -0.003994     │
│ 2024-08-08 ┆ UNH    ┆ Healthcare ┆ 0.000283      │
│ 2024-08-09 ┆ UNH    ┆ Healthcare ┆ -0.01321      │
│ 2024-08-12 ┆ UNH    ┆ Healthcare ┆ 0.011687      │
│ 2024-08-13 ┆ UNH    ┆ Healthcare ┆ 0.015832      │
└────────────┴────────┴────────────┴───────────────┘
```

**Computed Style Scores (cross-sectionally standardized):**

```text
┌────────────┬────────┬───────────┬───────────┐
│ date       ┆ symbol ┆ sze_score ┆ val_score │
│ ---        ┆ ---    ┆ ---       ┆ ---       │
│ date       ┆ str    ┆ f64       ┆ f64       │
╞════════════╪════════╪═══════════╪═══════════╡
│ 2024-08-07 ┆ UNH    ┆ 0.165245  ┆ -1.065573 │
│ 2024-08-08 ┆ UNH    ┆ 0.25773   ┆ -0.047803 │
│ 2024-08-09 ┆ UNH    ┆ 0.212605  ┆ 0.13597   │
│ 2024-08-12 ┆ UNH    ┆ 0.218459  ┆ -0.306406 │
│ 2024-08-13 ┆ UNH    ┆ 0.25663   ┆ -0.153037 │
└────────────┴────────┴───────────┴───────────┘

sze_score    | mean:  -0.0000 | std:  0.9832 | min:  -5.1927 | max:  0.6928
val_score    | mean:  -0.0000 | std:  0.4858 | min:  -1.6171 | max:  3.1439
```

**Annualized Factor Performance (real market data, Aug 2024 - Dec 2025):**

```text
Factor                    |  Ann. Return |     Ann. Vol |       Sharpe
----------------------------------------------------------------------
market                    |       21.26% |       18.64% |         1.14
sector_Finance            |       21.86% |       13.20% |         1.66
sector_Healthcare         |       -4.83% |       14.85% |        -0.33
sector_Technology         |      -17.03% |       12.17% |        -1.40
sze_score                 |      -44.93% |        6.65% |        -6.75
val_score                 |     -191.69% |       19.64% |        -9.76
```

## Required Data Inputs

You'll need the following data to run a complete model estimation:

### 1. Daily Asset Returns and Market Data

Symbol-by-symbol daily data including returns, market caps, and valuation metrics:

```text
┌────────────┬────────┬────────────┬───────────────┐
│ date       ┆ symbol ┆ sector     ┆ asset_returns │
│ ---        ┆ ---    ┆ ---        ┆ ---           │
│ date       ┆ str    ┆ str        ┆ f64           │
╞════════════╪════════╪════════════╪═══════════════╡
│ 2024-08-07 ┆ AAPL   ┆ Technology ┆ -0.004912     │
│ 2024-08-07 ┆ MSFT   ┆ Technology ┆ 0.012543      │
│ 2024-08-07 ┆ JNJ    ┆ Healthcare ┆ 0.003211      │
│ 2024-08-07 ┆ JPM    ┆ Finance    ┆ 0.008734      │
└────────────┴────────┴────────────┴───────────────┘
```

### 2. Sector Memberships (One-Hot Encoded)

One-hot encoded sector classifications for the factor model regression:

```text
┌────────────┬────────┬───────────────────┬───────────────────┬────────────────┐
│ date       ┆ symbol ┆ sector_Technology ┆ sector_Healthcare ┆ sector_Finance │
│ ---        ┆ ---    ┆ ---               ┆ ---               ┆ ---            │
│ date       ┆ str    ┆ f64               ┆ f64               ┆ f64            │
╞════════════╪════════╪═══════════════════╪═══════════════════╪════════════════╡
│ 2024-08-07 ┆ AAPL   ┆ 1.0               ┆ 0.0               ┆ 0.0            │
│ 2024-08-07 ┆ MSFT   ┆ 1.0               ┆ 0.0               ┆ 0.0            │
│ 2024-08-07 ┆ JNJ    ┆ 0.0               ┆ 1.0               ┆ 0.0            │
│ 2024-08-07 ┆ JPM    ┆ 0.0               ┆ 0.0               ┆ 1.0            │
└────────────┴────────┴───────────────────┴───────────────────┴────────────────┘
```

## Usage Examples

### Computing Style Factor Scores

```rust,ignore
use factors::{Factor, FactorRegistry, FactorConfig};
use polars::prelude::*;
use chrono::NaiveDate;

// Create registry with default factors
let registry = FactorRegistry::with_defaults();

// Get factors by name
let momentum = registry.get("long_term_momentum").unwrap();
let size = registry.get("log_market_cap").unwrap();
let value = registry.get("book_to_market").unwrap();

// Or create a custom registry with custom configuration
let mut custom_registry = FactorRegistry::new();
let custom_config = FactorConfig {
    trailing_days: 252,   // 1 year lookback
    half_life: 63,        // 3 month exponential decay
    lag: 20,              // Skip most recent month
    winsor_factor: 0.01,  // 1% winsorization
    ..Default::default()
};
custom_registry.register_with_config("custom_momentum", custom_config);

// Compute scores from your data
let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
let mom_scores = momentum.compute(&data.lazy(), date)?;
let sze_scores = size.compute(&data.lazy(), date)?;
let val_scores = value.compute(&data.lazy(), date)?;
```

### Estimating Factor Returns

```rust,ignore
use toraniko::model::{FactorReturnsEstimator, EstimatorConfig};
use toraniko::traits::ReturnsEstimator;
use toraniko::utils::top_n_by_group;

// Select top 3000 stocks by market cap for each date
let universe = top_n_by_group(
    merged_df.lazy(),
    3000,           // top N
    "market_cap",   // ranking variable
    &["date"],      // group by
    true,           // filter (vs. add mask column)
);

// Configure the estimator
let config = EstimatorConfig {
    winsor_factor: Some(0.05),     // 5% winsorization on returns
    residualize_styles: true,      // Orthogonalize styles to sectors
};

let estimator = FactorReturnsEstimator::with_config(config);

// Estimate factor returns
let (factor_returns, residuals) = estimator.estimate(
    returns_df.lazy(),
    mkt_cap_df.lazy(),
    sector_df.lazy(),
    style_df.lazy(),
)?;
```

### Data Utilities

```rust,ignore
use toraniko::utils::{fill_features, smooth_features, top_n_by_group};

// Forward-fill missing values within each symbol
let filled = fill_features(
    df.lazy(),
    &["price", "volume"],  // columns to fill
    "date",                // sort column
    "symbol",              // partition column
);

// Smooth with rolling mean
let smoothed = smooth_features(
    df.lazy(),
    &["returns"],   // columns to smooth
    "date",         // sort column
    "symbol",       // partition column
    5,              // window size
);
```

## Performance

On an M1 MacBook, this estimates 10+ years of daily market, sector and style factor returns in under a minute.

Single-day WLS factor estimation benchmarks:

| Assets | Single-Day WLS Estimation |
|--------|---------------------------|
| 1,000 | ~74 microseconds |
| 3,000 | ~222 microseconds |
| 5,000 | ~538 microseconds |

Run benchmarks with:

```bash
cargo bench
```

## Running the Examples

The crate includes several examples demonstrating key functionality:

```bash
# Fetch real market data from Yahoo Finance and run the full pipeline
cargo run --package toraniko --example yahoo_finance

# Style factor computation
cargo run --package toraniko --example style_factors

# Factor return estimation
cargo run --package toraniko --example factor_returns

# Data utility functions
cargo run --package toraniko --example data_utilities

# Generate synthetic market data (for offline testing)
cargo run --package toraniko --example generate_data

# Full end-to-end pipeline with synthetic data
cargo run --package toraniko --example full_pipeline
```

## Attribution

This is a Rust port of the original Python [toraniko](https://github.com/0xfdf/toraniko) library by [0xfdf](https://github.com/0xfdf). The original implementation provides the mathematical foundation and algorithmic approach that this crate follows.

## License

MIT License - see [LICENSE](../../LICENSE) for details.
