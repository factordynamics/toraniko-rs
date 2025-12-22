# toraniko

A Rust implementation of the toraniko factor model.

This crate provides a unified interface to the toraniko factor model ecosystem.
Individual components can be enabled via feature flags.

## Features

- `full` (default): Enables all components
- `primitives`: Core type definitions
- `traits`: Trait abstractions
- `math`: Mathematical operations
- `styles`: Style factor implementations
- `model`: Factor return estimation
- `utils`: Data utilities

## Example

```rust,ignore
// With default features (all components):
use toraniko::primitives;
use toraniko::model;

// Or with specific features only:
// [dependencies]
// toraniko = { version = "0.1", default-features = false, features = ["model"] }
```
