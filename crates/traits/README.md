# toraniko-traits

Trait abstractions for the toraniko factor model.

## Traits

- **Factor Traits**: `Factor`, `StyleFactor`, `SectorFactor`
- **Transform Traits**: `CrossSectionTransform`, `TimeSeriesTransform`
- **Estimator Traits**: `FactorEstimator`, `ReturnsEstimator`

## Design

This crate defines the core abstractions that allow pluggable implementations
of factors, transformations, and estimators. All style factors implement the
`StyleFactor` trait, enabling a unified API for factor computation.

## Usage

```rust,ignore
use toraniko_traits::{Factor, StyleFactor, FactorKind};
```
