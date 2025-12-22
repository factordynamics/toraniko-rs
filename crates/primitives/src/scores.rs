//! Factor score type definitions.

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};

use crate::Date;

/// Cross-sectional factor scores for a single date.
#[derive(Debug, Clone)]
pub struct AssetScores {
    /// Date of the scores.
    pub date: Date,
    /// Asset symbols (n_assets,).
    pub symbols: Vec<String>,
    /// Score values (n_assets,).
    pub scores: Array1<f64>,
}

impl AssetScores {
    /// Create new asset scores.
    #[must_use]
    pub fn new(date: Date, symbols: Vec<String>, scores: Array1<f64>) -> Self {
        debug_assert_eq!(symbols.len(), scores.len());
        Self { date, symbols, scores }
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

    /// Get score for a specific symbol.
    #[must_use]
    pub fn get(&self, symbol: &str) -> Option<f64> {
        self.symbols.iter().position(|s| s == symbol).map(|i| self.scores[i])
    }
}

/// Sector exposure scores (n_assets x n_sectors).
#[derive(Debug, Clone)]
pub struct SectorScores {
    /// Sector names.
    pub sector_names: Vec<String>,
    /// Exposure matrix (n_assets x n_sectors).
    pub exposures: Array2<f64>,
}

impl SectorScores {
    /// Create new sector scores.
    #[must_use]
    pub fn new(sector_names: Vec<String>, exposures: Array2<f64>) -> Self {
        debug_assert_eq!(sector_names.len(), exposures.ncols());
        Self { sector_names, exposures }
    }

    /// Number of sectors.
    #[must_use]
    pub const fn n_sectors(&self) -> usize {
        self.sector_names.len()
    }

    /// Number of assets.
    #[must_use]
    pub fn n_assets(&self) -> usize {
        self.exposures.nrows()
    }

    /// Get the column index for a sector name.
    #[must_use]
    pub fn sector_index(&self, name: &str) -> Option<usize> {
        self.sector_names.iter().position(|n| n == name)
    }
}

/// Style factor scores (n_assets x n_styles).
#[derive(Debug, Clone)]
pub struct StyleScores {
    /// Style names.
    pub style_names: Vec<String>,
    /// Score matrix (n_assets x n_styles).
    pub scores: Array2<f64>,
}

impl StyleScores {
    /// Create new style scores.
    #[must_use]
    pub fn new(style_names: Vec<String>, scores: Array2<f64>) -> Self {
        debug_assert_eq!(style_names.len(), scores.ncols());
        Self { style_names, scores }
    }

    /// Number of styles.
    #[must_use]
    pub const fn n_styles(&self) -> usize {
        self.style_names.len()
    }

    /// Number of assets.
    #[must_use]
    pub fn n_assets(&self) -> usize {
        self.scores.nrows()
    }

    /// Get the column index for a style name.
    #[must_use]
    pub fn style_index(&self, name: &str) -> Option<usize> {
        self.style_names.iter().position(|n| n == name)
    }
}

/// Combined factor scores for a single date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorScores {
    /// Date of the scores.
    pub date: Date,
    /// Asset symbols.
    pub symbols: Vec<String>,
    /// Sector names.
    pub sector_names: Vec<String>,
    /// Style names.
    pub style_names: Vec<String>,
    /// Sector exposures stored as flat vector (n_assets * n_sectors).
    #[serde(skip)]
    sector_data: Vec<f64>,
    /// Style scores stored as flat vector (n_assets * n_styles).
    #[serde(skip)]
    style_data: Vec<f64>,
}

impl FactorScores {
    /// Create new factor scores.
    #[must_use]
    pub fn new(
        date: Date,
        symbols: Vec<String>,
        sectors: SectorScores,
        styles: StyleScores,
    ) -> Self {
        debug_assert_eq!(symbols.len(), sectors.n_assets());
        debug_assert_eq!(symbols.len(), styles.n_assets());

        Self {
            date,
            symbols,
            sector_names: sectors.sector_names,
            style_names: styles.style_names,
            sector_data: sectors.exposures.into_raw_vec_and_offset().0,
            style_data: styles.scores.into_raw_vec_and_offset().0,
        }
    }

    /// Number of assets.
    #[must_use]
    pub const fn n_assets(&self) -> usize {
        self.symbols.len()
    }

    /// Number of sectors.
    #[must_use]
    pub const fn n_sectors(&self) -> usize {
        self.sector_names.len()
    }

    /// Number of styles.
    #[must_use]
    pub const fn n_styles(&self) -> usize {
        self.style_names.len()
    }

    /// Get sector exposures as a slice (n_assets * n_sectors).
    #[must_use]
    pub fn sector_exposures(&self) -> &[f64] {
        &self.sector_data
    }

    /// Get style scores as a slice (n_assets * n_styles).
    #[must_use]
    pub fn style_scores(&self) -> &[f64] {
        &self.style_data
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{Array2, array};

    use super::*;

    #[test]
    fn asset_scores_get() {
        let date = Date::from_ymd_opt(2024, 1, 1).unwrap();
        let scores =
            AssetScores::new(date, vec!["A".to_string(), "B".to_string()], array![0.5, -0.3]);

        assert_eq!(scores.get("A"), Some(0.5));
        assert_eq!(scores.get("B"), Some(-0.3));
        assert_eq!(scores.get("C"), None);
    }

    #[test]
    fn sector_scores_dimensions() {
        let exposures = Array2::from_shape_vec((3, 2), vec![1.0, 0.0, 0.0, 1.0, 1.0, 0.0]).unwrap();
        let sectors = SectorScores::new(vec!["Tech".to_string(), "Finance".to_string()], exposures);

        assert_eq!(sectors.n_assets(), 3);
        assert_eq!(sectors.n_sectors(), 2);
        assert_eq!(sectors.sector_index("Tech"), Some(0));
    }

    #[test]
    fn style_scores_dimensions() {
        let scores = Array2::from_shape_vec((2, 3), vec![0.1, 0.2, 0.3, -0.1, -0.2, -0.3]).unwrap();
        let styles = StyleScores::new(
            vec!["momentum".to_string(), "value".to_string(), "size".to_string()],
            scores,
        );

        assert_eq!(styles.n_assets(), 2);
        assert_eq!(styles.n_styles(), 3);
        assert_eq!(styles.style_index("value"), Some(1));
    }
}
