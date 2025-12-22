//! Constraint types for factor estimation.

/// Type of constraint to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstraintType {
    /// Sector returns must sum to zero (Barra-style).
    #[default]
    SumToZero,
    /// No constraint on sector returns.
    None,
}

/// Sector constraint configuration.
#[derive(Debug, Clone)]
pub struct SectorConstraint {
    /// Type of constraint.
    pub constraint_type: ConstraintType,
}

impl SectorConstraint {
    /// Create a new sum-to-zero constraint.
    #[must_use]
    pub const fn sum_to_zero() -> Self {
        Self { constraint_type: ConstraintType::SumToZero }
    }

    /// Create no constraint.
    #[must_use]
    pub const fn none() -> Self {
        Self { constraint_type: ConstraintType::None }
    }

    /// Check if constraint is active.
    #[must_use]
    pub const fn is_constrained(&self) -> bool {
        matches!(self.constraint_type, ConstraintType::SumToZero)
    }
}

impl Default for SectorConstraint {
    fn default() -> Self {
        Self::sum_to_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_type_default() {
        let ct = ConstraintType::default();
        assert_eq!(ct, ConstraintType::SumToZero);
    }

    #[test]
    fn sector_constraint_is_constrained() {
        let c = SectorConstraint::sum_to_zero();
        assert!(c.is_constrained());

        let c = SectorConstraint::none();
        assert!(!c.is_constrained());
    }
}
