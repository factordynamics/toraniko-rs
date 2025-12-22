//! Weighted least squares factor estimation.

use ndarray::{Array1, Array2};
use toraniko_math::{ConstrainedWlsResult, constrained_wls, winsorize};
use toraniko_traits::{EstimatorError, FactorEstimator};

/// Configuration for WLS estimator.
#[derive(Debug, Clone)]
pub struct WlsConfig {
    /// Winsorization percentile for returns (None to disable).
    pub winsor_factor: Option<f64>,
    /// Whether to residualize styles to sectors.
    pub residualize_styles: bool,
}

impl Default for WlsConfig {
    fn default() -> Self {
        Self { winsor_factor: Some(0.05), residualize_styles: true }
    }
}

/// Low-level WLS factor estimator.
///
/// Performs weighted least squares estimation for a single time period.
#[derive(Debug, Clone)]
pub struct WlsFactorEstimator {
    config: WlsConfig,
}

impl WlsFactorEstimator {
    /// Create a new WLS estimator with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(WlsConfig::default())
    }

    /// Get the winsorization factor.
    #[must_use]
    pub const fn winsor_factor(&self) -> Option<f64> {
        self.config.winsor_factor
    }

    /// Get whether styles are residualized.
    #[must_use]
    pub const fn residualize_styles(&self) -> bool {
        self.config.residualize_styles
    }
}

impl Default for WlsFactorEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl FactorEstimator for WlsFactorEstimator {
    type Config = WlsConfig;

    fn with_config(config: Self::Config) -> Self {
        Self { config }
    }

    fn estimate_single(
        &self,
        returns: &Array1<f64>,
        weights: &Array1<f64>,
        sector_scores: &Array2<f64>,
        style_scores: &Array2<f64>,
    ) -> Result<(Array1<f64>, Array1<f64>), EstimatorError> {
        let n = returns.len();
        let n_sectors = sector_scores.ncols();
        let n_styles = style_scores.ncols();

        // Validate dimensions
        if weights.len() != n {
            return Err(EstimatorError::DimensionMismatch {
                expected: n,
                actual: weights.len(),
                context: "weights".to_string(),
            });
        }

        if sector_scores.nrows() != n {
            return Err(EstimatorError::DimensionMismatch {
                expected: n,
                actual: sector_scores.nrows(),
                context: "sector_scores".to_string(),
            });
        }

        if style_scores.nrows() != n {
            return Err(EstimatorError::DimensionMismatch {
                expected: n,
                actual: style_scores.nrows(),
                context: "style_scores".to_string(),
            });
        }

        // Optionally winsorize returns
        let returns_clean = if let Some(pct) = self.config.winsor_factor {
            winsorize(returns, pct).map_err(|e| EstimatorError::LinearAlgebra(e.to_string()))?
        } else {
            returns.clone()
        };

        // Compute sqrt market cap weights
        let sqrt_weights: Array1<f64> = weights.mapv(|x| x.max(0.0).sqrt());

        // Perform constrained WLS
        let result: ConstrainedWlsResult =
            constrained_wls(&returns_clean, &sqrt_weights, sector_scores, style_scores)
                .map_err(|e| EstimatorError::LinearAlgebra(e.to_string()))?;

        // Combine into single factor returns array
        // Order: [market, sectors..., styles...]
        let mut factor_returns = Array1::zeros(1 + n_sectors + n_styles);
        factor_returns[0] = result.market_return;
        for (i, &r) in result.sector_returns.iter().enumerate() {
            factor_returns[1 + i] = r;
        }
        for (i, &r) in result.style_returns.iter().enumerate() {
            factor_returns[1 + n_sectors + i] = r;
        }

        Ok((factor_returns, result.residuals))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use ndarray::array;

    use super::*;

    #[test]
    fn wls_config_defaults() {
        let config = WlsConfig::default();
        assert_eq!(config.winsor_factor, Some(0.05));
        assert!(config.residualize_styles);
    }

    #[test]
    fn wls_estimator_basic() {
        let estimator = WlsFactorEstimator::new();

        let returns = array![0.01, 0.02, 0.015, 0.025, 0.03, 0.01];
        let weights = array![100.0, 200.0, 150.0, 250.0, 300.0, 100.0];

        // Two sectors
        let sectors = ndarray::Array2::from_shape_vec(
            (6, 2),
            vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        )
        .unwrap();

        // One style
        let styles =
            ndarray::Array2::from_shape_vec((6, 1), vec![0.5, 0.3, 0.2, -0.2, -0.3, -0.5]).unwrap();

        let (factor_returns, residuals) =
            estimator.estimate_single(&returns, &weights, &sectors, &styles).unwrap();

        // Should have 1 market + 2 sectors + 1 style = 4 factors
        assert_eq!(factor_returns.len(), 4);
        assert_eq!(residuals.len(), 6);

        // Sector returns should sum to zero
        let sector_sum = factor_returns[1] + factor_returns[2];
        assert_relative_eq!(sector_sum, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn wls_dimension_mismatch() {
        let estimator = WlsFactorEstimator::new();

        let returns = array![0.01, 0.02, 0.015];
        let weights = array![100.0, 200.0]; // Wrong size

        let sectors =
            ndarray::Array2::from_shape_vec((3, 2), vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0]).unwrap();
        let styles = ndarray::Array2::from_shape_vec((3, 1), vec![0.1, 0.2, 0.3]).unwrap();

        let result = estimator.estimate_single(&returns, &weights, &sectors, &styles);
        assert!(result.is_err());
    }
}
