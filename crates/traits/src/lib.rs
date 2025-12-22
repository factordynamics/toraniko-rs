#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod factor;
pub use factor::{Factor, FactorError, FactorKind, SectorFactor, StyleFactor};

mod transform;
pub use transform::{CrossSectionTransform, TimeSeriesTransform, TransformError};

mod estimator;
pub use estimator::{EstimatorError, FactorEstimator, ReturnsEstimator};
