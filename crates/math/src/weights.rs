//! Weight generation functions.

use ndarray::Array1;

/// Generate exponentially decaying weights.
///
/// # Arguments
/// * `window` - Number of trailing periods
/// * `half_life` - Half-life in periods
///
/// # Returns
/// Array of weights, most recent first, normalized to sum to 1.
#[must_use]
pub fn exp_weights(window: usize, half_life: usize) -> Array1<f64> {
    if window == 0 || half_life == 0 {
        return Array1::zeros(window);
    }

    let decay = 0.5_f64.powf(1.0 / half_life as f64);
    let mut weights = Array1::zeros(window);

    for i in 0..window {
        weights[i] = decay.powi(i as i32);
    }

    // Normalize to sum to 1
    let total: f64 = weights.sum();
    if total > 0.0 {
        weights /= total;
    }

    weights
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rstest::rstest;

    use super::*;

    #[test]
    fn exp_weights_sum_to_one() {
        let weights = exp_weights(20, 5);
        assert_relative_eq!(weights.sum(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn exp_weights_decreasing() {
        let weights = exp_weights(10, 3);
        for i in 1..weights.len() {
            assert!(weights[i] < weights[i - 1]);
        }
    }

    #[rstest]
    #[case(10, 5)]
    #[case(252, 126)]
    #[case(504, 126)]
    fn exp_weights_half_life_property(#[case] window: usize, #[case] half_life: usize) {
        let weights = exp_weights(window, half_life);
        // Weight at half_life should be ~0.5 of weight at 0
        if half_life < window {
            let ratio = weights[half_life] / weights[0];
            assert_relative_eq!(ratio, 0.5, epsilon = 0.01);
        }
    }

    #[test]
    fn exp_weights_zero_window() {
        let weights = exp_weights(0, 5);
        assert!(weights.is_empty());
    }

    #[test]
    fn exp_weights_zero_half_life() {
        let weights = exp_weights(10, 0);
        assert!(weights.iter().all(|&w| w == 0.0));
    }

    #[test]
    fn exp_weights_single_element() {
        let weights = exp_weights(1, 5);
        assert_eq!(weights.len(), 1);
        assert_relative_eq!(weights[0], 1.0, epsilon = 1e-10);
    }
}
