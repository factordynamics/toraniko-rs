#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod fill;
pub use fill::fill_features;

mod smooth;
pub use smooth::smooth_features;

mod rank;
pub use rank::top_n_by_group;

mod error;
pub use error::UtilsError;
