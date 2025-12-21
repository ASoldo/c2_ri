use serde::Serialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct FlightState {
    pub id: String,
    pub callsign: Option<String>,
    pub origin_country: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: Option<f64>,
    pub velocity_mps: Option<f64>,
    pub heading_deg: Option<f64>,
    pub on_ground: bool,
    pub last_contact: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlightSnapshot {
    pub provider: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub flights: Vec<FlightState>,
}

#[derive(Debug)]
pub struct FlightCache {
    pub last_fetch: Option<Instant>,
    pub payload: Option<FlightSnapshot>,
}

impl FlightCache {
    pub fn new() -> Self {
        Self {
            last_fetch: None,
            payload: None,
        }
    }
}

pub fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn sample_flights(now_ms: u64, count: usize) -> Vec<FlightState> {
    let t = (now_ms as f64 / 1000.0) / 180.0;
    let drift = t.sin() * 1.4;
    let wobble = t.cos() * 0.9;
    let mut flights = vec![
        ("SIM-ALPHA", "ALPHA1", 48.14, 11.58, 86.0, 235.0),
        ("SIM-BRAVO", "BRAVO2", 51.47, -0.45, 44.0, 218.0),
        ("SIM-CHARLIE", "CHARLIE3", 40.64, -73.78, 118.0, 246.0),
        ("SIM-DELTA", "DELTA4", 35.55, 139.78, 72.0, 264.0),
        ("SIM-ECHO", "ECHO5", 25.25, 55.36, 130.0, 222.0),
    ]
    .into_iter()
    .take(count.max(1))
    .enumerate()
    .map(|(idx, (id, callsign, lat, lon, heading, speed))| {
        let jitter = drift * (1.0 + idx as f64 * 0.12);
        FlightState {
            id: id.to_string(),
            callsign: Some(callsign.to_string()),
            origin_country: Some("SIM".to_string()),
            lat: lat + jitter,
            lon: lon + wobble * (1.0 + idx as f64 * 0.08),
            altitude_m: Some(9800.0 + idx as f64 * 420.0),
            velocity_mps: Some(speed),
            heading_deg: Some((heading + drift * 12.0 + idx as f64 * 6.0) % 360.0),
            on_ground: false,
            last_contact: Some((now_ms / 1000) as i64),
        }
    })
    .collect::<Vec<_>>();
    flights.truncate(count.max(1));
    flights
}

pub fn sample_flights_near(
    now_ms: u64,
    count: usize,
    center_lat: f64,
    center_lon: f64,
) -> Vec<FlightState> {
    let mut flights = sample_flights(now_ms, count);
    let lat = center_lat.max(-85.0).min(85.0);
    let lon = center_lon.max(-180.0).min(180.0);
    let t = (now_ms as f64 / 1000.0) / 90.0;
    for (idx, flight) in flights.iter_mut().enumerate() {
        let spread = 5.0 + idx as f64 * 2.5;
        let drift = (t + idx as f64 * 0.4).sin() * 1.6;
        let wobble = (t + idx as f64 * 0.6).cos() * 2.1;
        flight.lat = (lat + spread * 0.15 + drift).max(-85.0).min(85.0);
        flight.lon = (lon + spread * 0.2 + wobble).max(-180.0).min(180.0);
        flight.heading_deg = flight.heading_deg.map(|h| (h + drift * 8.0) % 360.0);
    }
    flights
}
