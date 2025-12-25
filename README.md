# toraniko-rs

A high-performance Rust implementation of the Toraniko characteristic factor model for quantitative equity trading.

[![CI](https://github.com/factordynamics/toraniko-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/factordynamics/toraniko-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/toraniko.svg)](https://crates.io/crates/toraniko)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

**toraniko-rs** is a Rust port of the [toraniko](https://github.com/0xfdf/toraniko) Python library, providing institutional-grade factor model estimation similar to Barra and Axioma systems. It estimates daily factor returns across multiple time periods with high performance.

### Mathematical Model

The factor model decomposes asset returns as:

```
r_asset = β_market * r_market + Σ(β_sector * r_sector) + Σ(β_style * r_style) + ε
```

Where:
- `r_market` is the market factor return
- `r_sector` are sector factor returns (constrained to sum to zero)
- `r_style` are style factor returns (momentum, value, size)
- `ε` is the idiosyncratic residual

## Features

- **Factor Return Estimation**: Weighted least squares with market cap weighting
- **Sector Constraint**: Barra-style constraint ensuring sector returns sum to zero
- **Style Factors**: Momentum, Size, and Value factor implementations
- **Cross-Sectional Operations**: Centering, normalization, winsorization
- **Data Utilities**: Forward-fill, smoothing, top-N selection
- **High Performance**: Optimized for large universes (3000+ stocks)

## Performance

Single-day WLS factor estimation (11 sectors, 5 styles, M1 MacBook):

| Assets | Python | Rust | Speedup |
|--------|--------|------|---------|
| 1,000 | 1.2 ms | 74 μs | **16x** |
| 3,000 | 10 ms | 222 μs | **46x** |
| 5,000 | 25 ms | 538 μs | **47x** |

Run `just bench` for full benchmarks.

## Crate Structure

| Crate | Description |
|-------|-------------|
| `toraniko-primitives` | Core type definitions (Asset, Symbol, FactorReturns, etc.) |
| `toraniko-traits` | Trait abstractions (Factor, StyleFactor, ReturnsEstimator) |
| `toraniko-math` | Mathematical operations (WLS, winsorization, weights) |
| `factors` | Style factor implementations (Momentum, Size, Value) with registry |
| `toraniko-model` | Factor return estimation |
| `toraniko-utils` | Data utilities (fill, smooth, rank) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
toraniko-model = "0.1"
factors = "0.1"
toraniko-utils = "0.1"
```

## Usage

### Computing Style Factor Scores

```rust
use factors::{Factor, FactorRegistry};
use polars::prelude::*;
use chrono::NaiveDate;

// Create registry with default factors
let registry = FactorRegistry::with_defaults();

// Get factors by name
let momentum = registry.get("long_term_momentum").unwrap();
let size = registry.get("log_market_cap").unwrap();
let value = registry.get("book_to_market").unwrap();

// Compute scores from your data
let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
let mom_scores = momentum.compute(&data.lazy(), date)?;
let sze_scores = size.compute(&data.lazy(), date)?;
let val_scores = value.compute(&data.lazy(), date)?;
```

### Estimating Factor Returns

```rust
use toraniko_model::{FactorReturnsEstimator, EstimatorConfig};
use toraniko_traits::ReturnsEstimator;

// Configure estimator
let estimator = FactorReturnsEstimator::with_config(EstimatorConfig {
    winsor_factor: Some(0.05),     // 5% winsorization
    residualize_styles: true,      // Orthogonalize styles to sectors
});

// Estimate factor returns
let (factor_returns_df, residuals_df) = estimator.estimate(
    returns_df.lazy(),
    mkt_cap_df.lazy(),
    sector_df.lazy(),
    style_df.lazy(),
)?;
```

### Custom Factor Configuration

```rust
use factors::{Factor, FactorRegistry, FactorConfig};

// Create registry
let mut registry = FactorRegistry::new();

// Add custom configured factor
let config = FactorConfig {
    trailing_days: 252,   // 1 year lookback
    half_life: 63,        // 3 month decay
    lag: 20,              // Skip most recent month
    winsor_factor: 0.01,
    ..Default::default()
};

registry.register_with_config("custom_momentum", config);
let momentum = registry.get("custom_momentum").unwrap();
```

### Data Utilities

```rust
use toraniko_utils::{fill_features, smooth_features, top_n_by_group};

// Forward-fill missing values
let filled = fill_features(df.lazy(), &["price", "volume"], "date", "symbol");

// Smooth with rolling mean
let smoothed = smooth_features(df.lazy(), &["returns"], "date", "symbol", 5);

// Top 1000 stocks by market cap per date
let universe = top_n_by_group(df.lazy(), 1000, "market_cap", &["date"], true);
```

## Quick Analysis

Analyze factor attribution for any stock using Yahoo Finance data:

```bash
just analyze UNH        # Default 5-year analysis
just analyze AAPL 3     # 3-year analysis
```

Example output:
```
================================================================================
FACTOR ATTRIBUTION ANALYSIS: UNH
================================================================================
Period: 2022-08-30 to 2025-12-22
Total Return:   +65.52%
--------------------------------------------------------------------------------
Factor               Exposure     Factor Ret   Contribution
-------------------- ------------ ------------ --------------
Market                  1.000         62.82%         62.82%
Healthcare              1.000         18.79%         18.79%
Size                    0.137       -198.91%        -27.32%
Value                  -0.130       -563.61%         73.08%
Momentum               -0.419         10.60%         -4.44%
Idiosyncratic               -              -        -57.41%
--------------------------------------------------------------------------------
SUMMARY:
  Factor-Explained Return:  +122.93%
  Idiosyncratic Return:      -57.41%
================================================================================
```

The idiosyncratic return (-57.41%) represents the portion of UNH's return not explained by the factors. This is large because toraniko-rs is a framework for building factor models, not a pre-built model itself. The demo uses crude proxies from Yahoo Finance data and only 3 style factors across 30 stocks. A well-specified production model with proper fundamental data, more factors, and a larger universe should minimize this residual component.

## Benchmarks

Run benchmarks with:

```bash
just bench
```

Benchmark scenarios include:
- Factor estimation scaling (100 to 5000 assets)
- Style factor count scaling
- Sector count scaling
- Winsorization impact

## Development

### Prerequisites

- Rust 1.88+ (2024 edition)
- [just](https://github.com/casey/just) task runner

### Commands

```bash
# Run all CI checks
just ci

# Run tests
just test

# Run benchmarks
just bench

# Format code
just fix

# Check code
just check

# Build release
just build
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Original Python implementation: [0xfdf/toraniko](https://github.com/0xfdf/toraniko)
