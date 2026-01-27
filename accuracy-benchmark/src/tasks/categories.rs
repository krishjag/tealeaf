//! Task category definitions

use serde::{Deserialize, Serialize};

/// Business domain categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Finance,
    Healthcare,
    Retail,
    Logistics,
    Hr,
    Marketing,
    Legal,
    Technology,
    Manufacturing,
    RealEstate,
}

impl Domain {
    pub fn all() -> Vec<Domain> {
        vec![
            Domain::Finance,
            Domain::Healthcare,
            Domain::Retail,
            Domain::Logistics,
            Domain::Hr,
            Domain::Marketing,
            Domain::Legal,
            Domain::Technology,
            Domain::Manufacturing,
            Domain::RealEstate,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Domain::Finance => "finance",
            Domain::Healthcare => "healthcare",
            Domain::Retail => "retail",
            Domain::Logistics => "logistics",
            Domain::Hr => "hr",
            Domain::Marketing => "marketing",
            Domain::Legal => "legal",
            Domain::Technology => "technology",
            Domain::Manufacturing => "manufacturing",
            Domain::RealEstate => "real_estate",
        }
    }
}

impl std::str::FromStr for Domain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "finance" => Ok(Domain::Finance),
            "healthcare" => Ok(Domain::Healthcare),
            "retail" => Ok(Domain::Retail),
            "logistics" => Ok(Domain::Logistics),
            "hr" | "human_resources" => Ok(Domain::Hr),
            "marketing" => Ok(Domain::Marketing),
            "legal" => Ok(Domain::Legal),
            "technology" | "tech" => Ok(Domain::Technology),
            "manufacturing" => Ok(Domain::Manufacturing),
            "real_estate" | "realestate" => Ok(Domain::RealEstate),
            _ => Err(format!("Unknown domain: {}", s)),
        }
    }
}

/// Complexity levels for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Simple = 1,
    Moderate = 2,
    Complex = 3,
    Advanced = 4,
    Expert = 5,
}

impl Complexity {
    pub fn from_level(level: u8) -> Option<Self> {
        match level {
            1 => Some(Complexity::Simple),
            2 => Some(Complexity::Moderate),
            3 => Some(Complexity::Complex),
            4 => Some(Complexity::Advanced),
            5 => Some(Complexity::Expert),
            _ => None,
        }
    }

    pub fn level(&self) -> u8 {
        *self as u8
    }
}

impl std::str::FromStr for Complexity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" | "1" => Ok(Complexity::Simple),
            "moderate" | "2" => Ok(Complexity::Moderate),
            "complex" | "3" => Ok(Complexity::Complex),
            "advanced" | "4" => Ok(Complexity::Advanced),
            "expert" | "5" => Ok(Complexity::Expert),
            _ => Err(format!("Unknown complexity: {}", s)),
        }
    }
}

/// Type of expected output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    Summary,
    Calculation,
    Analysis,
    Prediction,
    Recommendation,
}

impl std::str::FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "summary" | "extraction" => Ok(OutputType::Summary),
            "calculation" | "aggregation" => Ok(OutputType::Calculation),
            "analysis" | "pattern" | "trend" => Ok(OutputType::Analysis),
            "prediction" | "forecast" => Ok(OutputType::Prediction),
            "recommendation" | "decision" => Ok(OutputType::Recommendation),
            _ => Err(format!("Unknown output type: {}", s)),
        }
    }
}
