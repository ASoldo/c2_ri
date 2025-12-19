use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_m: Option<f64>,
    pub accuracy_m: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
}

impl BoundingBox {
    pub fn contains(&self, coord: Coordinate) -> bool {
        coord.latitude <= self.north
            && coord.latitude >= self.south
            && coord.longitude <= self.east
            && coord.longitude >= self.west
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeoFence {
    Circle { center: Coordinate, radius_m: f64 },
    Polygon { vertices: Vec<Coordinate> },
}
