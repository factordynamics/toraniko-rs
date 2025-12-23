#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod attribution;
pub use attribution::{AttributionResult, FactorContribution, compute_attribution};

mod factor_returns;
pub use factor_returns::{EstimatorConfig, FactorReturnsEstimator};

mod wls;
pub use wls::{WlsConfig, WlsFactorEstimator};

mod constraints;
pub use constraints::{ConstraintType, SectorConstraint};

mod error;
pub use error::ModelError;

/// Re-export commonly used types.
pub mod prelude {
    pub use toraniko_traits::{FactorEstimator, ReturnsEstimator};

    pub use super::{EstimatorConfig, FactorReturnsEstimator, ModelError};
}
