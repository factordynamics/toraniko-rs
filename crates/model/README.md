# toraniko-model

Factor return estimation for the toraniko factor model.

## Overview

This crate provides the core factor return estimation logic, implementing
a characteristic factor model similar to Barra and Axioma systems.

## Key Types

- `FactorReturnsEstimator` - Main entry point for factor return estimation
- `EstimatorConfig` - Configuration for the estimator

## Usage

```rust,ignore
use toraniko_model::{FactorReturnsEstimator, EstimatorConfig};
use toraniko_traits::ReturnsEstimator;

let estimator = FactorReturnsEstimator::with_config(EstimatorConfig {
    winsor_factor: Some(0.05),
    residualize_styles: true,
});

let (factor_returns, residuals) = estimator.estimate(
    returns_df,
    mkt_cap_df,
    sector_df,
    style_df,
)?;
```

## Mathematical Model

The factor model decomposes asset returns as:

```text
r_asset = β_market * r_market + Σ(β_sector * r_sector) + Σ(β_style * r_style) + ε
```

Where:
- `r_market` is the market factor return
- `r_sector` are sector factor returns (constrained to sum to zero)
- `r_style` are style factor returns
- `ε` is the idiosyncratic residual
