# toraniko-primitives

Core type definitions for the toraniko factor model.

## Types

- **Asset Types**: `AssetId`, `Symbol`, `Asset`
- **Factor Types**: `FactorName`, `FactorReturns`, `FactorExposures`
- **Return Types**: `AssetReturns`, `ResidualReturns`
- **Score Types**: `AssetScores`, `SectorScores`, `StyleScores`, `FactorScores`
- **Weight Types**: `MarketCapWeights`, `ExponentialWeights`

## Usage

```rust,ignore
use toraniko_primitives::{Symbol, FactorReturns, MarketCapWeights};
```
