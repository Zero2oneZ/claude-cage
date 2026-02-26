//! WindingLevel - Refinement levels for tori
//!
//! Step 1.13 from BUILD_STEPS.md

use serde::{Deserialize, Serialize};

/// Winding levels represent the refinement state of a torus
///
/// Higher levels = more trustworthy, more validated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum WindingLevel {
    /// Just captured, unstructured
    RawIdea = 1,

    /// Organized, has structure
    Structured = 2,

    /// Edge cases handled
    Refined = 3,

    /// Tests pass, validated
    Tested = 4,

    /// Full documentation
    Documented = 5,

    /// Deployed to production
    Production = 6,
}

impl WindingLevel {
    /// Convert from u8
    pub fn from_u8(level: u8) -> Option<Self> {
        match level {
            1 => Some(Self::RawIdea),
            2 => Some(Self::Structured),
            3 => Some(Self::Refined),
            4 => Some(Self::Tested),
            5 => Some(Self::Documented),
            6 => Some(Self::Production),
            _ => None,
        }
    }

    /// Convert to u8
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Get trustworthiness factor (0.0 - 1.0)
    ///
    /// Higher winding = higher trustworthiness
    pub fn trustworthiness(&self) -> f64 {
        match self {
            Self::RawIdea => 0.1,
            Self::Structured => 0.3,
            Self::Refined => 0.5,
            Self::Tested => 0.7,
            Self::Documented => 0.85,
            Self::Production => 1.0,
        }
    }

    /// Get the next level (if not already at max)
    pub fn next(&self) -> Option<Self> {
        Self::from_u8(self.as_u8() + 1)
    }

    /// Get the previous level (if not already at min)
    pub fn prev(&self) -> Option<Self> {
        if self.as_u8() > 1 {
            Self::from_u8(self.as_u8() - 1)
        } else {
            None
        }
    }

    /// Human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::RawIdea => "Raw idea, just captured",
            Self::Structured => "Organized with structure",
            Self::Refined => "Edge cases handled",
            Self::Tested => "Validated with tests",
            Self::Documented => "Fully documented",
            Self::Production => "Production ready",
        }
    }

    /// Requirements to advance to this level
    pub fn requirements(&self) -> &'static str {
        match self {
            Self::RawIdea => "Initial capture",
            Self::Structured => "Organize content, define boundaries",
            Self::Refined => "Handle edge cases, resolve ambiguities",
            Self::Tested => "Pass validation tests, verify correctness",
            Self::Documented => "Write complete documentation",
            Self::Production => "Deploy and monitor in production",
        }
    }

    /// Check if this level allows certain operations
    pub fn allows_production_use(&self) -> bool {
        *self >= Self::Tested
    }

    /// Check if this level requires audit before modification
    pub fn requires_audit(&self) -> bool {
        *self >= Self::Production
    }
}

impl Default for WindingLevel {
    fn default() -> Self {
        Self::RawIdea
    }
}

impl std::fmt::Display for WindingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L{}: {}", self.as_u8(), self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u8() {
        assert_eq!(WindingLevel::from_u8(1), Some(WindingLevel::RawIdea));
        assert_eq!(WindingLevel::from_u8(6), Some(WindingLevel::Production));
        assert_eq!(WindingLevel::from_u8(0), None);
        assert_eq!(WindingLevel::from_u8(7), None);
    }

    #[test]
    fn test_trustworthiness_ordering() {
        let levels = [
            WindingLevel::RawIdea,
            WindingLevel::Structured,
            WindingLevel::Refined,
            WindingLevel::Tested,
            WindingLevel::Documented,
            WindingLevel::Production,
        ];

        for i in 1..levels.len() {
            assert!(
                levels[i].trustworthiness() > levels[i - 1].trustworthiness(),
                "Level {:?} should have higher trust than {:?}",
                levels[i],
                levels[i - 1]
            );
        }
    }

    #[test]
    fn test_next_prev() {
        let level = WindingLevel::Refined;
        assert_eq!(level.next(), Some(WindingLevel::Tested));
        assert_eq!(level.prev(), Some(WindingLevel::Structured));

        assert_eq!(WindingLevel::Production.next(), None);
        assert_eq!(WindingLevel::RawIdea.prev(), None);
    }

    #[test]
    fn test_production_requirements() {
        assert!(!WindingLevel::RawIdea.allows_production_use());
        assert!(!WindingLevel::Structured.allows_production_use());
        assert!(!WindingLevel::Refined.allows_production_use());
        assert!(WindingLevel::Tested.allows_production_use());
        assert!(WindingLevel::Production.allows_production_use());
    }
}
