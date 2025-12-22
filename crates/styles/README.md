# toraniko-styles

Style factor implementations for the toraniko factor model.

## Factors

- **Momentum**: Exponentially-weighted cumulative returns with lag
- **Size**: Log market cap (SMB-style, negated so small = positive)
- **Value**: Composite of book/price, sales/price, cash flow/price

## Usage

```rust,ignore
use toraniko_styles::{MomentumFactor, SizeFactor, ValueFactor};
use toraniko_traits::Factor;

let mom = MomentumFactor::new();
let scores = mom.compute_scores(data)?;
```

## Configuration

Each factor has a `Config` type with sensible defaults:

```rust,ignore
use toraniko_styles::{MomentumConfig, MomentumFactor};
use toraniko_traits::StyleFactor;

let config = MomentumConfig {
    trailing_days: 252,  // 1 year instead of default 504
    half_life: 63,       // 3 months instead of default 126
    lag: 20,
    winsor_factor: 0.01,
};
let mom = MomentumFactor::with_config(config);
```
