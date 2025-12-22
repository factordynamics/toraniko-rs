//! Weight type definitions.

use ndarray::Array1;
use serde::{Deserialize, Serialize};

/// Market capitalization weights.
#[derive(Debug, Clone)]
pub struct MarketCapWeights {
    /// Raw market caps.
    raw: Array1<f64>,
    /// Normalized weights (sum to 1).
    normalized: Array1<f64>,
    /// Square root weights for WLS.
    sqrt_weights: Array1<f64>,
}

impl MarketCapWeights {
    /// Create market cap weights from raw values.
    #[must_use]
    pub fn from_raw(raw: Array1<f64>) -> Self {
        let total: f64 = raw.sum();
        let normalized = if total > 0.0 { &raw / total } else { raw.clone() };
        let sqrt_weights = raw.mapv(|x| x.max(0.0).sqrt());

        Self { raw, normalized, sqrt_weights }
    }

    /// Get the raw market caps.
    #[must_use]
    pub const fn raw(&self) -> &Array1<f64> {
        &self.raw
    }

    /// Get normalized weights (sum to 1).
    #[must_use]
    pub const fn normalized(&self) -> &Array1<f64> {
        &self.normalized
    }

    /// Get square root weights for WLS regression.
    #[must_use]
    pub const fn sqrt_weights(&self) -> &Array1<f64> {
        &self.sqrt_weights
    }

    /// Number of assets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

/// Exponentially decaying weights for time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExponentialWeights {
    /// Window size.
    window: usize,
    /// Half-life in periods.
    half_life: usize,
    /// Computed weights (most recent first).
    weights: Vec<f64>,
}

impl ExponentialWeights {
    /// Create exponential weights.
    ///
    /// # Arguments
    /// * `window` - Number of trailing periods
    /// * `half_life` - Half-life in periods
    #[must_use]
    pub fn new(window: usize, half_life: usize) -> Self {
        let weights = Self::compute_weights(window, half_life);
        Self { window, half_life, weights }
    }

    fn compute_weights(window: usize, half_life: usize) -> Vec<f64> {
        if window == 0 || half_life == 0 {
            return vec![0.0; window];
        }

        let decay = 0.5_f64.powf(1.0 / half_life as f64);
        let mut weights: Vec<f64> = (0..window).map(|i| decay.powi(i as i32)).collect();

        // Normalize to sum to 1
        let total: f64 = weights.iter().sum();
        if total > 0.0 {
            for w in &mut weights {
                *w /= total;
            }
        }

        weights
    }

    /// Get the weights as a slice.
    #[must_use]
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Get the weights as an ndarray.
    #[must_use]
    pub fn to_array(&self) -> Array1<f64> {
        Array1::from_vec(self.weights.clone())
    }

    /// Get window size.
    #[must_use]
    pub const fn window(&self) -> usize {
        self.window
    }

    /// Get half-life.
    #[must_use]
    pub const fn half_life(&self) -> usize {
        self.half_life
    }

    /// Get weight at a specific lag.
    #[must_use]
    pub fn at(&self, lag: usize) -> Option<f64> {
        self.weights.get(lag).copied()
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn market_cap_weights_normalized() {
        let raw = array![100.0, 200.0, 300.0, 400.0];
        let weights = MarketCapWeights::from_raw(raw);

        let sum: f64 = weights.normalized().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn market_cap_weights_sqrt() {
        let raw = array![4.0, 9.0, 16.0];
        let weights = MarketCapWeights::from_raw(raw);

        assert!((weights.sqrt_weights()[0] - 2.0).abs() < 1e-10);
        assert!((weights.sqrt_weights()[1] - 3.0).abs() < 1e-10);
        assert!((weights.sqrt_weights()[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn exp_weights_sum_to_one() {
        let weights = ExponentialWeights::new(20, 5);
        let sum: f64 = weights.weights().iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn exp_weights_decreasing() {
        let weights = ExponentialWeights::new(10, 3);
        for i in 1..weights.weights().len() {
            assert!(weights.weights()[i] < weights.weights()[i - 1]);
        }
    }

    #[test]
    fn exp_weights_half_life_property() {
        let weights = ExponentialWeights::new(20, 5);
        // Weight at half_life should be ~0.5 of weight at 0
        let ratio = weights.at(5).unwrap() / weights.at(0).unwrap();
        assert!((ratio - 0.5).abs() < 0.01);
    }
}
