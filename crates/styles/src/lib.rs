#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod momentum;
pub use momentum::{MomentumConfig, MomentumFactor};

mod size;
pub use size::{SizeConfig, SizeFactor};

mod value;
pub use value::{ValueConfig, ValueFactor};

mod error;
pub use error::StyleError;
