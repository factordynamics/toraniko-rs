//! Return type definitions.

use ndarray::Array1;
use serde::{Deserialize, Serialize};

use crate::Date;

/// Asset returns for a single date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReturns {
    /// Date of the returns.
    pub date: Date,
    /// Asset symbols.
    pub symbols: Vec<String>,
    /// Return values.
    #[serde(skip)]
    pub returns: Array1<f64>,
}

impl AssetReturns {
    /// Create new asset returns.
    #[must_use]
    pub fn new(date: Date, symbols: Vec<String>, returns: Array1<f64>) -> Self {
        debug_assert_eq!(symbols.len(), returns.len());
        Self { date, symbols, returns }
    }

    /// Number of assets.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Get return for a specific symbol.
    #[must_use]
    pub fn get(&self, symbol: &str) -> Option<f64> {
        self.symbols.iter().position(|s| s == symbol).map(|i| self.returns[i])
    }
}

/// Residual returns (idiosyncratic returns) for a single date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidualReturns {
    /// Date of the returns.
    pub date: Date,
    /// Asset symbols.
    pub symbols: Vec<String>,
    /// Residual return values.
    #[serde(skip)]
    pub residuals: Array1<f64>,
}

impl ResidualReturns {
    /// Create new residual returns.
    #[must_use]
    pub fn new(date: Date, symbols: Vec<String>, residuals: Array1<f64>) -> Self {
        debug_assert_eq!(symbols.len(), residuals.len());
        Self { date, symbols, residuals }
    }

    /// Number of assets.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Get residual for a specific symbol.
    #[must_use]
    pub fn get(&self, symbol: &str) -> Option<f64> {
        self.symbols.iter().position(|s| s == symbol).map(|i| self.residuals[i])
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn asset_returns_get() {
        let date = Date::from_ymd_opt(2024, 1, 1).unwrap();
        let returns = AssetReturns::new(
            date,
            vec!["AAPL".to_string(), "GOOG".to_string()],
            array![0.01, 0.02],
        );

        assert_eq!(returns.get("AAPL"), Some(0.01));
        assert_eq!(returns.get("GOOG"), Some(0.02));
        assert_eq!(returns.get("MSFT"), None);
    }

    #[test]
    fn residual_returns_len() {
        let date = Date::from_ymd_opt(2024, 1, 1).unwrap();
        let residuals = ResidualReturns::new(
            date,
            vec!["A".to_string(), "B".to_string()],
            array![0.001, -0.002],
        );

        assert_eq!(residuals.len(), 2);
        assert!(!residuals.is_empty());
    }
}
