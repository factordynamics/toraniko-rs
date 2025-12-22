//! Asset type definitions.

use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};

/// Unique identifier for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into, Serialize, Deserialize)]
pub struct AssetId(pub u64);

impl AssetId {
    /// Create a new asset ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Stock ticker symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub struct Symbol(pub String);

impl Symbol {
    /// Create a new symbol.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the symbol as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Represents a single asset with its metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    /// Unique identifier.
    pub id: AssetId,
    /// Ticker symbol.
    pub symbol: Symbol,
    /// Optional sector classification.
    pub sector: Option<String>,
}

impl Asset {
    /// Create a new asset.
    #[must_use]
    pub const fn new(id: AssetId, symbol: Symbol, sector: Option<String>) -> Self {
        Self { id, symbol, sector }
    }

    /// Create an asset with just an ID and symbol.
    #[must_use]
    pub const fn simple(id: AssetId, symbol: Symbol) -> Self {
        Self { id, symbol, sector: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_from_str() {
        let sym: Symbol = "AAPL".into();
        assert_eq!(sym.as_str(), "AAPL");
    }

    #[test]
    fn asset_creation() {
        let asset =
            Asset::new(AssetId::new(1), Symbol::new("GOOG"), Some("Technology".to_string()));
        assert_eq!(asset.symbol.as_str(), "GOOG");
        assert_eq!(asset.sector, Some("Technology".to_string()));
    }
}
