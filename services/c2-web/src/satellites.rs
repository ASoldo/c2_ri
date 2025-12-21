use serde::Serialize;
use sgp4::{iau_epoch_to_sidereal_time, julian_years_since_j2000, Constants, Elements};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct SatelliteState {
    pub id: String,
    pub name: Option<String>,
    pub norad_id: u64,
    pub lat: f64,
    pub lon: f64,
    pub altitude_km: f64,
    pub velocity_kms: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub period_min: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SatelliteSnapshot {
    pub provider: String,
    pub source: String,
    pub timestamp_ms: u64,
    pub satellites: Vec<SatelliteState>,
}

#[derive(Debug)]
pub struct SatelliteCache {
    pub last_fetch: Option<Instant>,
    pub payload: Option<SatelliteSnapshot>,
}

impl SatelliteCache {
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

fn teme_to_ecef(position: [f64; 3], gmst_rad: f64) -> [f64; 3] {
    let (x, y, z) = (position[0], position[1], position[2]);
    let cos_t = gmst_rad.cos();
    let sin_t = gmst_rad.sin();
    [
        cos_t * x + sin_t * y,
        -sin_t * x + cos_t * y,
        z,
    ]
}

fn ecef_to_geodetic(position: [f64; 3]) -> Option<(f64, f64, f64)> {
    let (x, y, z) = (position[0], position[1], position[2]);
    let a = sgp4::WGS84.ae;
    let f = 1.0 / 298.257_223_563;
    let b = a * (1.0 - f);
    let e2 = f * (2.0 - f);
    let ep2 = (a * a - b * b) / (b * b);
    let p = (x * x + y * y).sqrt();
    if p < 1e-8 {
        return None;
    }
    let theta = (z * a).atan2(p * b);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();
    let lat = (z + ep2 * b * sin_theta.powi(3))
        .atan2(p - e2 * a * cos_theta.powi(3));
    let lon = y.atan2(x);
    let sin_lat = lat.sin();
    let n = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;
    Some((lat.to_degrees(), lon.to_degrees(), alt))
}

pub fn satellites_from_elements(
    elements: &[Elements],
    now: &sgp4::chrono::NaiveDateTime,
    provider: &str,
    limit: usize,
) -> Vec<SatelliteState> {
    let epoch = julian_years_since_j2000(now);
    let gmst = iau_epoch_to_sidereal_time(epoch);
    let mut satellites = Vec::with_capacity(limit.max(1));
    for element in elements.iter() {
        if satellites.len() >= limit {
            break;
        }
        let minutes = match element.datetime_to_minutes_since_epoch(now) {
            Ok(minutes) => minutes,
            Err(_) => continue,
        };
        let constants = match Constants::from_elements(element) {
            Ok(constants) => constants,
            Err(_) => continue,
        };
        let prediction = match constants.propagate(minutes) {
            Ok(prediction) => prediction,
            Err(_) => continue,
        };
        let ecef = teme_to_ecef(prediction.position, gmst);
        let (lat, lon, altitude_km) = match ecef_to_geodetic(ecef) {
            Some(values) => values,
            None => continue,
        };
        let velocity_kms = {
            let v = prediction.velocity;
            let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if speed.is_finite() { Some(speed) } else { None }
        };
        let period_min = if element.mean_motion > 0.0 {
            Some(1440.0 / element.mean_motion)
        } else {
            None
        };
        let id = format!("{provider}:{}", element.norad_id);
        satellites.push(SatelliteState {
            id,
            name: element.object_name.clone(),
            norad_id: element.norad_id,
            lat,
            lon,
            altitude_km,
            velocity_kms,
            inclination_deg: Some(element.inclination),
            period_min,
        });
    }
    satellites
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

pub fn sample_satellites(now_ms: u64, count: usize) -> Vec<SatelliteState> {
    let t = (now_ms as f64 / 1000.0) / 240.0;
    let drift = t.sin() * 3.2;
    let wobble = t.cos() * 4.1;
    let mut sats = vec![
        (25544_u64, "ISS", 12.4, -42.2, 420.0, 7.66, 51.6, 92.9),
        (20580_u64, "HST", 28.3, -75.1, 540.0, 7.56, 28.5, 96.5),
        (24876_u64, "GPS BIIR-2", 40.2, 71.4, 20200.0, 3.87, 55.0, 718.0),
        (33591_u64, "IRIDIUM 33", -12.8, 131.5, 780.0, 7.46, 86.4, 100.0),
        (43226_u64, "STARLINK-2101", 8.4, 25.2, 550.0, 7.60, 53.0, 95.0),
        (36744_u64, "SES-1", 0.2, -79.0, 35786.0, 3.07, 0.1, 1436.0),
    ]
    .into_iter()
    .take(count.max(1))
    .enumerate()
    .map(
        |(idx, (norad_id, name, lat, lon, altitude_km, velocity_kms, inc, period))| {
            let jitter = drift * (1.0 + idx as f64 * 0.12);
            let drift_lon = wobble * (1.0 + idx as f64 * 0.08);
            let alt_wobble = (t + idx as f64 * 0.2).sin() * 8.0;
            SatelliteState {
                id: format!("sim:{norad_id}"),
                name: Some(name.to_string()),
                norad_id,
                lat: (lat + jitter).max(-85.0).min(85.0),
                lon: clamp_lon(lon + drift_lon),
                altitude_km: (altitude_km + alt_wobble).max(100.0),
                velocity_kms: Some(velocity_kms),
                inclination_deg: Some(inc),
                period_min: Some(period),
            }
        },
    )
    .collect::<Vec<_>>();
    sats.truncate(count.max(1));
    sats
}
