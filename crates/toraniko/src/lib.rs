#![doc = include_str!("../README.md")]
#![doc(
    issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/",
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
