use serde::Serialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct ShipState {
    pub id: String,
    pub mmsi: Option<u64>,
    pub name: Option<String>,
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub speed_knots: Option<f64>,
    pub course_deg: Option<f64>,
    pub heading_deg: Option<f64>,
    pub vessel_type: Option<i32>,
    pub status: Option<i32>,
    pub length_m: Option<f64>,
    pub width_m: Option<f64>,
    pub draught_m: Option<f64>,
    pub destination: Option<String>,
    pub last_report_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShipSnapshot {
    pub provider: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub ships: Vec<ShipState>,
}

#[derive(Debug)]
pub struct ShipCache {
    pub last_fetch: Option<Instant>,
    pub payload: Option<ShipSnapshot>,
}

impl ShipCache {
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

fn clamp_lon(mut lon: f64) -> f64 {
    while lon > 180.0 {
        lon -= 360.0;
    }
    while lon < -180.0 {
        lon += 360.0;
    }
    lon
}

pub fn sample_ships(now_ms: u64, count: usize) -> Vec<ShipState> {
    let t = (now_ms as f64 / 1000.0) / 300.0;
    let drift = t.sin() * 0.9;
    let wobble = t.cos() * 1.2;
    let mut ships = vec![
        (366982330_u64, "PACIFIC CREST", 33.73, -118.26, 14.2, 220.0, 70),
        (563091000_u64, "STRAIT EAGLE", 1.26, 103.84, 12.5, 45.0, 80),
        (244660489_u64, "EUROPA TRADER", 51.95, 4.14, 9.8, 270.0, 70),
        (413892000_u64, "SHANGHAI STAR", 31.40, 121.50, 11.3, 110.0, 70),
        (368018000_u64, "NORFOLK SPIRIT", 36.94, -76.33, 8.1, 180.0, 60),
        (431002517_u64, "TOKYO MARU", 35.64, 139.77, 10.6, 30.0, 70),
    ]
    .into_iter()
    .take(count.max(1))
    .enumerate()
    .map(|(idx, (mmsi, name, lat, lon, speed, heading, vessel_type))| {
        let jitter = drift * (1.0 + idx as f64 * 0.12);
        let drift_lon = wobble * (1.0 + idx as f64 * 0.08);
        ShipState {
            id: format!("sim:{mmsi}"),
            mmsi: Some(mmsi),
            name: Some(name.to_string()),
            callsign: None,
            lat: (lat + jitter).max(-85.0).min(85.0),
            lon: clamp_lon(lon + drift_lon),
            speed_knots: Some(speed),
            course_deg: Some(heading),
            heading_deg: Some(heading),
            vessel_type: Some(vessel_type),
            status: Some(0),
            length_m: None,
            width_m: None,
            draught_m: None,
            destination: None,
            last_report_ms: Some(now_ms as i64),
        }
    })
    .collect::<Vec<_>>();
    ships.truncate(count.max(1));
    ships
}
