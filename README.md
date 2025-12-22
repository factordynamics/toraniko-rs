# toraniko-rs

A high-performance Rust implementation of the Toraniko characteristic factor model for quantitative equity trading.

[![CI](https://github.com/factordynamics/toraniko-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/factordynamics/toraniko-rs/actions/workflows/ci.yml)
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

## Crate Structure

| Crate | Description |
|-------|-------------|
| `toraniko-primitives` | Core type definitions (Asset, Symbol, FactorReturns, etc.) |
| `toraniko-traits` | Trait abstractions (Factor, StyleFactor, ReturnsEstimator) |
| `toraniko-math` | Mathematical operations (WLS, winsorization, weights) |
| `toraniko-styles` | Style factor implementations (Momentum, Size, Value) |
| `toraniko-model` | Factor return estimation |
| `toraniko-utils` | Data utilities (fill, smooth, rank) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
toraniko-model = "0.1"
toraniko-styles = "0.1"
toraniko-utils = "0.1"
```

## Usage

### Computing Style Factor Scores

```rust
use toraniko_styles::{MomentumFactor, SizeFactor, ValueFactor};
use toraniko_traits::Factor;
use polars::prelude::*;

// Create factors
let momentum = MomentumFactor::new();
let size = SizeFactor::new();
let value = ValueFactor::new();

// Compute scores from your data
let mom_scores = momentum.compute_scores(returns_df.lazy())?;
let sze_scores = size.compute_scores(mkt_cap_df.lazy())?;
let val_scores = value.compute_scores(value_df.lazy())?;
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
use toraniko_styles::{MomentumConfig, MomentumFactor};
use toraniko_traits::StyleFactor;

let config = MomentumConfig {
    trailing_days: 252,   // 1 year lookback
    half_life: 63,        // 3 month decay
    lag: 20,              // Skip most recent month
    winsor_factor: 0.01,
};

let momentum = MomentumFactor::with_config(config);
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
