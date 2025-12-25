//! Factor trait definitions.

// Re-export core traits from the factors crate
pub use factors::{
    ConfigurableFactor, DataFrequency, Factor, FactorCategory, FactorConfig, FactorError,
};

/// The kind of factor in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactorKind {
    /// Market factor (beta to overall market).
    Market,
    /// Sector/industry classification factor.
    Sector,
    /// Style/characteristic factor (momentum, value, size, etc.).
    Style,
}

impl std::fmt::Display for FactorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Market => write!(f, "market"),
            Self::Sector => write!(f, "sector"),
            Self::Style => write!(f, "style"),
        }
    }
}

/// Trait for style factors (momentum, value, size, etc.).
///
/// Style factors extend the base Factor and ConfigurableFactor traits from the factors crate.
pub trait StyleFactor: Factor + ConfigurableFactor {
    /// Returns the kind of factor (always Style for style factors).
    fn kind(&self) -> FactorKind {
        FactorKind::Style
    }

    /// Returns whether this factor should be orthogonalized to sector factors.
    fn residualize(&self) -> bool {
        true // default behavior
    }
}

/// Trait for sector/industry factors.
///
/// Sector factors extend the base Factor trait from the factors crate.
pub trait SectorFactor: Factor {
    /// Returns the list of sector names covered by this factor.
    fn sector_names(&self) -> &[String];

    /// Returns whether sector returns should be constrained to sum to zero.
    fn constrained(&self) -> bool {
        true // Barra-style constraint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_kind_display() {
        assert_eq!(FactorKind::Market.to_string(), "market");
        assert_eq!(FactorKind::Sector.to_string(), "sector");
        assert_eq!(FactorKind::Style.to_string(), "style");
    }
}
