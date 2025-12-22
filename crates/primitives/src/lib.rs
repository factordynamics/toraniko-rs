#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/factordynamics/toraniko-rs/issues/")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]

mod asset;
pub use asset::{Asset, AssetId, Symbol};

mod factor;
pub use factor::{FactorExposures, FactorName, FactorReturns};

mod returns;
pub use returns::{AssetReturns, ResidualReturns};

mod scores;
pub use scores::{AssetScores, FactorScores, SectorScores, StyleScores};

mod weights;
pub use weights::{ExponentialWeights, MarketCapWeights};

/// Re-export common date type.
pub type Date = chrono::NaiveDate;
