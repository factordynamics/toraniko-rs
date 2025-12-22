//! Factor-related type definitions.

use serde::{Deserialize, Serialize};

use crate::Date;

/// Name of a factor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactorName(pub String);

impl FactorName {
    /// Create a new factor name.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the factor name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FactorName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for FactorName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Factor returns for a single date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorReturns {
    /// Date of the returns.
    pub date: Date,
    /// Market factor return.
    pub market: f64,
    /// Sector factor returns (keyed by sector name).
    pub sectors: Vec<(String, f64)>,
    /// Style factor returns (keyed by style name).
    pub styles: Vec<(String, f64)>,
}

impl FactorReturns {
    /// Create new factor returns.
    #[must_use]
    pub const fn new(
        date: Date,
        market: f64,
        sectors: Vec<(String, f64)>,
        styles: Vec<(String, f64)>,
    ) -> Self {
        Self { date, market, sectors, styles }
    }

    /// Get the return for a specific factor by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f64> {
        if name == "market" {
            return Some(self.market);
        }

        self.sectors
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| *r)
            .or_else(|| self.styles.iter().find(|(n, _)| n == name).map(|(_, r)| *r))
    }

    /// Returns all factor names.
    #[must_use]
    pub fn factor_names(&self) -> Vec<&str> {
        let mut names = vec!["market"];
        names.extend(self.sectors.iter().map(|(n, _)| n.as_str()));
        names.extend(self.styles.iter().map(|(n, _)| n.as_str()));
        names
    }

    /// Returns the total number of factors.
    #[must_use]
    pub const fn n_factors(&self) -> usize {
        1 + self.sectors.len() + self.styles.len()
    }
}

/// Factor exposures (betas) for an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorExposures {
    /// Market beta.
    pub market: f64,
    /// Sector exposures (typically one-hot encoded).
    pub sectors: Vec<f64>,
    /// Style exposures.
    pub styles: Vec<f64>,
}

impl FactorExposures {
    /// Create new factor exposures.
    #[must_use]
    pub const fn new(market: f64, sectors: Vec<f64>, styles: Vec<f64>) -> Self {
        Self { market, sectors, styles }
    }

    /// Returns the total number of exposures.
    #[must_use]
    pub const fn n_exposures(&self) -> usize {
        1 + self.sectors.len() + self.styles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_returns_get() {
        let date = Date::from_ymd_opt(2024, 1, 1).unwrap();
        let fr = FactorReturns::new(
            date,
            0.01,
            vec![("Technology".to_string(), 0.02)],
            vec![("momentum".to_string(), 0.005)],
        );

        assert_eq!(fr.get("market"), Some(0.01));
        assert_eq!(fr.get("Technology"), Some(0.02));
        assert_eq!(fr.get("momentum"), Some(0.005));
        assert_eq!(fr.get("nonexistent"), None);
    }

    #[test]
    fn factor_returns_n_factors() {
        let date = Date::from_ymd_opt(2024, 1, 1).unwrap();
        let fr = FactorReturns::new(
            date,
            0.01,
            vec![("A".to_string(), 0.0), ("B".to_string(), 0.0)],
            vec![("X".to_string(), 0.0)],
        );

        assert_eq!(fr.n_factors(), 4); // market + 2 sectors + 1 style
    }
}
