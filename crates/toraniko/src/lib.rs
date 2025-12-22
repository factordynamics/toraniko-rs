//! # toraniko
//!
//! A Rust implementation of the toraniko factor model.
//!
//! This crate provides a unified interface to the toraniko factor model ecosystem.
//! Individual components can be enabled via feature flags.
//!
//! ## Features
//!
//! - `full` (default): Enables all components
//! - `primitives`: Core type definitions
//! - `traits`: Trait abstractions
//! - `math`: Mathematical operations
//! - `styles`: Style factor implementations
//! - `model`: Factor return estimation
//! - `utils`: Data utilities
//!
//! ## Example
//!
//! ```rust,ignore
//! // With default features (all components):
//! use toraniko::primitives;
//! use toraniko::model;
//!
//! // Or with specific features only:
//! // [dependencies]
//! // toraniko = { version = "0.1", default-features = false, features = ["model"] }
//! ```

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/factordynamics/toraniko-rs/main/assets/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/factordynamics/toraniko-rs/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[cfg(feature = "primitives")]
#[doc(inline)]
pub use toraniko_primitives as primitives;
#[cfg(feature = "traits")]
#[doc(inline)]
pub use toraniko_traits as traits;
#[cfg(feature = "math")]
#[doc(inline)]
pub use toraniko_math as math;
#[cfg(feature = "styles")]
#[doc(inline)]
pub use toraniko_styles as styles;
#[cfg(feature = "model")]
#[doc(inline)]
pub use toraniko_model as model;
#[cfg(feature = "utils")]
#[doc(inline)]
pub use toraniko_utils as utils;
