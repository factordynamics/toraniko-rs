#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod cross_section;
pub use cross_section::{
    CenterXSection, NormXSection, center_xsection, norm_xsection, percentiles_xsection,
};

mod winsorize;
pub use winsorize::{Winsorizer, winsorize, winsorize_xsection};

mod weights;
pub use weights::exp_weights;

mod linalg;
pub use linalg::{ConstrainedWlsResult, WlsResult, constrained_wls, weighted_least_squares};

mod error;
pub use error::MathError;
