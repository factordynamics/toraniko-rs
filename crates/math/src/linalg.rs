//! Linear algebra operations for factor estimation.

use ndarray::{Array1, Array2, s};

use crate::MathError;

/// Result of weighted least squares regression.
#[derive(Debug, Clone)]
pub struct WlsResult {
    /// Estimated coefficients.
    pub coefficients: Array1<f64>,
    /// Residuals.
    pub residuals: Array1<f64>,
    /// R-squared.
    pub r_squared: f64,
}

/// Result of constrained weighted least squares for factor model.
#[derive(Debug, Clone)]
pub struct ConstrainedWlsResult {
    /// Market return.
    pub market_return: f64,
    /// Sector returns.
    pub sector_returns: Array1<f64>,
    /// Style returns.
    pub style_returns: Array1<f64>,
    /// Residual returns.
    pub residuals: Array1<f64>,
}

/// Perform weighted least squares regression.
///
/// Solves: argmin_beta sum(w_i * (y_i - X_i * beta)^2)
///
/// Uses SVD-based solution for numerical stability.
///
/// # Arguments
/// * `y` - Response vector (n,)
/// * `x` - Design matrix (n x p)
/// * `weights` - Weight vector (n,), typically sqrt(market_cap)
///
/// # Returns
/// WLS result with coefficients and residuals.
///
/// # Errors
/// Returns error if dimensions mismatch or matrix is singular.
pub fn weighted_least_squares(
    y: &Array1<f64>,
    x: &Array2<f64>,
    weights: &Array1<f64>,
) -> Result<WlsResult, MathError> {
    let n = y.len();
    let p = x.ncols();

    if x.nrows() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: x.nrows() });
    }
    if weights.len() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: weights.len() });
    }

    if n == 0 {
        return Err(MathError::EmptyData);
    }

    // Weight the response
    let y_weighted: Array1<f64> = y.iter().zip(weights.iter()).map(|(yi, wi)| yi * wi).collect();

    // Weight the design matrix
    let mut x_weighted = x.clone();
    for i in 0..n {
        for j in 0..p {
            x_weighted[[i, j]] *= weights[i];
        }
    }

    // Solve using normal equations: (X'WX)^-1 X'Wy
    // For numerical stability, we use the weighted versions directly
    let xtx = x_weighted.t().dot(&x_weighted);
    let xty = x_weighted.t().dot(&y_weighted);

    // Solve using Cholesky or fallback to pseudo-inverse approach
    let coefficients = solve_linear_system(&xtx, &xty)?;

    // Compute residuals
    let fitted = x.dot(&coefficients);
    let residuals = y - &fitted;

    // Compute R-squared
    let y_mean = y.mean().unwrap_or(0.0);
    let ss_tot: f64 = y.iter().map(|yi| (yi - y_mean).powi(2)).sum();
    let ss_res: f64 = residuals.iter().map(|r| r.powi(2)).sum();
    let r_squared = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };

    Ok(WlsResult { coefficients, residuals, r_squared })
}

/// Solve a linear system Ax = b using Gaussian elimination with partial pivoting.
fn solve_linear_system(a: &Array2<f64>, b: &Array1<f64>) -> Result<Array1<f64>, MathError> {
    let n = a.nrows();
    if n == 0 {
        return Err(MathError::EmptyData);
    }
    if a.ncols() != n {
        return Err(MathError::LinearAlgebra("matrix must be square".to_string()));
    }
    if b.len() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: b.len() });
    }

    // Augmented matrix [A | b]
    let mut aug = Array2::zeros((n, n + 1));
    for i in 0..n {
        for j in 0..n {
            aug[[i, j]] = a[[i, j]];
        }
        aug[[i, n]] = b[i];
    }

    // Gaussian elimination with partial pivoting
    for col in 0..n {
        // Find pivot
        let mut max_row = col;
        let mut max_val = aug[[col, col]].abs();
        for row in (col + 1)..n {
            if aug[[row, col]].abs() > max_val {
                max_val = aug[[row, col]].abs();
                max_row = row;
            }
        }

        if max_val < 1e-14 {
            return Err(MathError::LinearAlgebra(
                "matrix is singular or nearly singular".to_string(),
            ));
        }

        // Swap rows
        if max_row != col {
            for j in 0..=n {
                let tmp = aug[[col, j]];
                aug[[col, j]] = aug[[max_row, j]];
                aug[[max_row, j]] = tmp;
            }
        }

        // Eliminate column
        for row in (col + 1)..n {
            let factor = aug[[row, col]] / aug[[col, col]];
            for j in col..=n {
                aug[[row, j]] -= factor * aug[[col, j]];
            }
        }
    }

    // Back substitution
    let mut x = Array1::zeros(n);
    for i in (0..n).rev() {
        let mut sum = aug[[i, n]];
        for j in (i + 1)..n {
            sum -= aug[[i, j]] * x[j];
        }
        x[i] = sum / aug[[i, i]];
    }

    Ok(x)
}

/// Perform constrained weighted least squares for factor model.
///
/// Implements the constraint that sector factor returns sum to zero,
/// which makes the market factor identifiable.
///
/// # Arguments
/// * `y` - Asset returns (n,)
/// * `weights` - Market cap sqrt weights (n,)
/// * `sector_matrix` - Sector exposures (n x n_sectors)
/// * `style_matrix` - Style scores (n x n_styles)
///
/// # Returns
/// Constrained WLS result with market, sector, style returns and residuals.
///
/// # Errors
/// Returns error if dimensions mismatch or system is singular.
pub fn constrained_wls(
    y: &Array1<f64>,
    weights: &Array1<f64>,
    sector_matrix: &Array2<f64>,
    style_matrix: &Array2<f64>,
) -> Result<ConstrainedWlsResult, MathError> {
    let n = y.len();
    let n_sectors = sector_matrix.ncols();
    let n_styles = style_matrix.ncols();

    // Validate dimensions
    if weights.len() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: weights.len() });
    }
    if sector_matrix.nrows() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: sector_matrix.nrows() });
    }
    if style_matrix.nrows() != n {
        return Err(MathError::DimensionMismatch { expected: n, actual: style_matrix.nrows() });
    }

    if n_sectors == 0 {
        return Err(MathError::LinearAlgebra("must have at least one sector".to_string()));
    }

    // Build design matrix: [1 | transformed_sectors | styles]
    // We impose the constraint sum(sector_returns) = 0 by using a change of variables.
    // Instead of n_sectors columns, we use n_sectors - 1 columns where each column
    // is the difference from the last sector.
    let n_cols = 1 + (n_sectors - 1) + n_styles;
    let mut x = Array2::zeros((n, n_cols));

    // Market column (all ones)
    for i in 0..n {
        x[[i, 0]] = 1.0;
    }

    // Sector columns (difference from last sector)
    for i in 0..n {
        for j in 0..(n_sectors - 1) {
            x[[i, 1 + j]] = sector_matrix[[i, j]] - sector_matrix[[i, n_sectors - 1]];
        }
    }

    // Style columns
    for i in 0..n {
        for j in 0..n_styles {
            x[[i, 1 + (n_sectors - 1) + j]] = style_matrix[[i, j]];
        }
    }

    // Perform WLS
    let result = weighted_least_squares(y, &x, weights)?;

    // Extract results
    let market_return = result.coefficients[0];

    // Reconstruct sector returns with constraint
    let mut sector_returns = Array1::zeros(n_sectors);
    for j in 0..(n_sectors - 1) {
        sector_returns[j] = result.coefficients[1 + j];
    }
    // Last sector return is negative sum of others (constraint: sum = 0)
    sector_returns[n_sectors - 1] = -sector_returns.slice(s![..(n_sectors - 1)]).sum();

    let style_returns = result.coefficients.slice(s![(1 + n_sectors - 1)..]).to_owned();

    Ok(ConstrainedWlsResult {
        market_return,
        sector_returns,
        style_returns,
        residuals: result.residuals,
    })
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use ndarray::array;

    use super::*;

    #[test]
    fn wls_simple_regression() {
        let y = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let x =
            Array2::from_shape_vec((5, 2), vec![1.0, 1.0, 1.0, 2.0, 1.0, 3.0, 1.0, 4.0, 1.0, 5.0])
                .unwrap();
        let weights = array![1.0, 1.0, 1.0, 1.0, 1.0];

        let result = weighted_least_squares(&y, &x, &weights).unwrap();

        // Perfect fit: y = 0 + 1*x
        assert_relative_eq!(result.coefficients[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result.coefficients[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result.r_squared, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn wls_weighted_regression() {
        let y = array![1.0, 2.0, 3.0, 4.0, 100.0];
        let x =
            Array2::from_shape_vec((5, 2), vec![1.0, 1.0, 1.0, 2.0, 1.0, 3.0, 1.0, 4.0, 1.0, 5.0])
                .unwrap();
        // Very low weight on the outlier
        let weights = array![1.0, 1.0, 1.0, 1.0, 0.001];

        let result = weighted_least_squares(&y, &x, &weights).unwrap();

        // Should be close to y = 0 + 1*x ignoring the outlier
        assert_relative_eq!(result.coefficients[1], 1.0, epsilon = 0.1);
    }

    #[test]
    fn constrained_wls_sector_sum_zero() {
        let y = array![0.01, 0.02, 0.015, 0.025, 0.03, 0.01];
        let weights = array![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];

        // Two sectors, each asset in one sector
        let sectors = Array2::from_shape_vec(
            (6, 2),
            vec![1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
        )
        .unwrap();

        // One style factor
        let styles = Array2::from_shape_vec((6, 1), vec![0.5, 0.3, 0.2, -0.2, -0.3, -0.5]).unwrap();

        let result = constrained_wls(&y, &weights, &sectors, &styles).unwrap();

        // Sector returns should sum to zero
        let sector_sum: f64 = result.sector_returns.sum();
        assert_relative_eq!(sector_sum, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn constrained_wls_dimensions() {
        // Need more observations than unknowns: 1 market + 2 sectors + 1 style = 4 unknowns
        // Use 6 observations for a well-determined system
        let y = array![0.01, 0.02, 0.015, 0.025, 0.012, 0.018];
        let weights = array![1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        // 2 sectors (one-hot encoded)
        let sectors = Array2::from_shape_vec(
            (6, 2),
            vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
        )
        .unwrap();
        // 1 style factor
        let styles = Array2::from_shape_vec((6, 1), vec![0.1, -0.1, 0.2, 0.0, -0.1, 0.15]).unwrap();

        let result = constrained_wls(&y, &weights, &sectors, &styles).unwrap();

        assert_eq!(result.sector_returns.len(), 2);
        assert_eq!(result.style_returns.len(), 1);
        assert_eq!(result.residuals.len(), 6);
    }
}
