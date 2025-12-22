# toraniko-utils

Data utilities for the toraniko factor model.

## Functions

- `fill_features` - Forward-fill null values within partitions
- `smooth_features` - Apply rolling mean smoothing
- `top_n_by_group` - Select top N rows per group

## Usage

```rust,ignore
use toraniko_utils::{fill_features, smooth_features, top_n_by_group};

// Fill nulls in features, sorted by date, partitioned by symbol
let filled = fill_features(df.lazy(), &["price", "volume"], "date", "symbol");

// Smooth features with 5-day rolling mean
let smoothed = smooth_features(df.lazy(), &["price"], "date", "symbol", 5);

// Get top 100 stocks by market cap per date
let top = top_n_by_group(df.lazy(), 100, "market_cap", &["date"], true);
```
