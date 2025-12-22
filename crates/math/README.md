# toraniko-math

Mathematical operations for the toraniko factor model.

## Functions

### Cross-Sectional Operations
- `center_xsection` - Cross-sectional centering and standardization
- `norm_xsection` - Normalize to a range
- `percentiles_xsection` - Percentile-based masking

### Winsorization
- `winsorize` - Clip array to symmetric percentiles
- `winsorize_xsection` - Cross-sectional winsorization

### Weights
- `exp_weights` - Exponentially decaying weights

### Linear Algebra
- `weighted_least_squares` - WLS regression
- `constrained_wls` - Factor model with sector constraint

## Usage

```rust,ignore
use toraniko_math::{center_xsection, winsorize, exp_weights};
```
