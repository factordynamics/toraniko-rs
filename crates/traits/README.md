# toraniko-traits

Trait abstractions for the toraniko factor model.

## Traits

### Re-exported from `factors` crate

The following core types are re-exported from the `factors` crate for convenience:
- `Factor` - Core factor trait for computing factor scores
- `FactorCategory` - Enum categorizing factors (Style, Sector, Market, etc.)
- `ConfigurableFactor` - Trait for factors with configurable parameters
- `FactorError` - Error types for factor operations
- `DataFrequency` - Enum for data frequency (Daily, Weekly, Monthly)

### Toraniko-specific Traits

- **Factor Traits**: `FactorKind`, `StyleFactor`, `SectorFactor`
- **Transform Traits**: `CrossSectionTransform`, `TimeSeriesTransform`
- **Estimator Traits**: `FactorEstimator`, `ReturnsEstimator`

## Design

This crate defines the core abstractions that allow pluggable implementations
of factors, transformations, and estimators. Core factor types like `Factor`,
`FactorCategory`, and `ConfigurableFactor` are re-exported from the `factors`
crate, which provides the canonical factor registry and implementations.

The toraniko-specific traits (`FactorKind`, `StyleFactor`, `SectorFactor`) extend
the base `Factor` trait with additional functionality specific to the toraniko
factor model architecture.

## Usage

```rust,ignore
// Re-exported from factors crate
use toraniko_traits::{Factor, FactorCategory, ConfigurableFactor};

// Toraniko-specific types
use toraniko_traits::{FactorKind, StyleFactor, SectorFactor};
```
