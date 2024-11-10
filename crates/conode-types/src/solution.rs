use serde::{Deserialize, Serialize};
use std::fmt;

// Base trait for solution implementations
pub trait CompletionSolution: fmt::Debug + Send + Sync {
    fn is_valid(&self) -> bool;
    fn as_bytes(&self) -> Vec<u8>;
}

// Enum wrapper for different solution types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolutionType {
    Uri(UriSolution),
    Coordinate(CoordinateSolution),
}

// Implementation of base trait for enum
impl CompletionSolution for SolutionType {
    fn is_valid(&self) -> bool {
        match self {
            SolutionType::Uri(s) => s.is_valid(),
            SolutionType::Coordinate(s) => s.is_valid(),
        }
    }

    fn as_bytes(&self) -> Vec<u8> {
        match self {
            SolutionType::Uri(s) => s.as_bytes(),
            SolutionType::Coordinate(s) => s.as_bytes(),
        }
    }
}

// Individual solution types remain similar but implement their own validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UriSolution {
    pub uri: String,
    pub timestamp: u64,
}

impl UriSolution {
    pub fn new(uri: String, timestamp: u64) -> Self {
        Self { uri, timestamp }
    }

    fn is_valid(&self) -> bool {
        self.uri.starts_with("ipfs://")
            || self.uri.starts_with("http://")
            || self.uri.starts_with("https://")
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.uri.as_bytes());
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateSolution {
    latitude: f64,
    longitude: f64,
    altitude: Option<f64>,
    timestamp: u64,
    accuracy: f32,
}

impl CoordinateSolution {
    pub fn new(
        latitude: f64,
        longitude: f64,
        altitude: Option<f64>,
        timestamp: u64,
        accuracy: f32,
    ) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
            timestamp,
            accuracy,
        }
    }

    fn is_valid(&self) -> bool {
        self.latitude >= -90.0
            && self.latitude <= 90.0
            && self.longitude >= -180.0
            && self.longitude <= 180.0
            && self.accuracy >= 0.0
    }

    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.latitude.to_be_bytes());
        bytes.extend_from_slice(&self.longitude.to_be_bytes());
        if let Some(alt) = self.altitude {
            bytes.extend_from_slice(&alt.to_be_bytes());
        }
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes.extend_from_slice(&self.accuracy.to_be_bytes());
        bytes
    }
}
