use actix_web::{get, web, Error, HttpResponse};
use serde::Deserialize;

use crate::flights::{
    now_epoch_millis, sample_flights, sample_flights_near, FlightSnapshot, FlightState,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct FlightQuery {
    lamin: Option<f64>,
    lomin: Option<f64>,
    lamax: Option<f64>,
    lomax: Option<f64>,
    limit: Option<usize>,
}

fn clamp_lat(value: f64) -> f64 {
    value.max(-90.0).min(90.0)
}

fn clamp_lon(value: f64) -> f64 {
    let mut lon = value;
    if lon > 180.0 {
        lon = 180.0;
    }
    if lon < -180.0 {
        lon = -180.0;
    }
    lon
}

fn value_as_f64(value: Option<&serde_json::Value>) -> Option<f64> {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_f64(),
        Some(serde_json::Value::String(text)) => text.parse::<f64>().ok(),
        _ => None,
    }
}

fn value_as_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn haversine_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r_km = 6371.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + (dlon / 2.0).sin().powi(2) * lat1.cos() * lat2.cos();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = r_km * c;
    km / 1.852
}

fn adsb_center_dist_nm(
    bbox: Option<(f64, f64, f64, f64)>,
) -> (f64, f64, f64) {
    let (lat, lon) = if let Some((lamin, lomin, lamax, lomax)) = bbox {
        ((lamin + lamax) / 2.0, (lomin + lomax) / 2.0)
    } else {
        (0.0, 0.0)
    };
    let dist_nm: f64 = if let Some((lamin, lomin, lamax, lomax)) = bbox {
        let lat_mid = (lamin + lamax) / 2.0;
        let lon_mid = (lomin + lomax) / 2.0;
        let samples = [
            (lamin, lomin),
            (lamin, lomax),
            (lamax, lomin),
            (lamax, lomax),
            (lamin, lon_mid),
            (lamax, lon_mid),
            (lat_mid, lomin),
            (lat_mid, lomax),
        ];
        let mut max_nm: f64 = 0.0;
        for (clat, clon) in samples {
            max_nm = max_nm.max(haversine_nm(lat, lon, clat, clon));
        }
        max_nm
    } else {
        250.0
    };
    (lat, lon, dist_nm.clamp(25.0, 12000.0))
}

fn build_adsb_url(
    base_url: &str,
    bbox: Option<(f64, f64, f64, f64)>,
) -> Result<reqwest::Url, Error> {
    let (lat, lon, dist_nm) = adsb_center_dist_nm(bbox);
    let mut url = base_url.to_string();
    url = url.replace("{lat}", &format!("{lat:.4}"));
    url = url.replace("{lon}", &format!("{lon:.4}"));
    url = url.replace("{dist}", &format!("{dist_nm:.0}"));
    let parsed = reqwest::Url::parse(&url)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid flight base url"))?;
    Ok(parsed)
}

fn parse_opensky(value: serde_json::Value, provider: &str, limit: usize) -> FlightSnapshot {
    let now_ms = now_epoch_millis();
    let time = value
        .get("time")
        .and_then(|v| v.as_i64())
        .map(|t| t as u64 * 1000)
        .unwrap_or(now_ms);
    let mut records = Vec::new();
    if let Some(states) = value.get("states").and_then(|v| v.as_array()) {
        for (idx, state) in states.iter().enumerate() {
            let Some(values) = state.as_array() else { continue };
            let icao = values.get(0).and_then(|v| v.as_str()).unwrap_or("").trim();
            let callsign = values
                .get(1)
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            let origin = values
                .get(2)
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            let lon = values.get(5).and_then(|v| v.as_f64());
            let lat = values.get(6).and_then(|v| v.as_f64());
            let (Some(lat), Some(lon)) = (lat, lon) else { continue };
            let baro_alt = values.get(7).and_then(|v| v.as_f64());
            let on_ground = values.get(8).and_then(|v| v.as_bool()).unwrap_or(false);
            let velocity = values.get(9).and_then(|v| v.as_f64());
            let heading = values.get(10).and_then(|v| v.as_f64());
            let geo_alt = values.get(13).and_then(|v| v.as_f64());
            let last_contact = values
                .get(4)
                .and_then(|v| v.as_i64())
                .or_else(|| values.get(3).and_then(|v| v.as_i64()));

            let id = if !icao.is_empty() {
                format!("{provider}:{icao}")
            } else if let Some(callsign) = callsign.as_deref() {
                format!("{provider}:{callsign}")
            } else {
                format!("{provider}:unknown-{idx}")
            };

            records.push(FlightState {
                id,
                callsign,
                origin_country: origin,
                lat,
                lon,
                altitude_m: geo_alt.or(baro_alt),
                velocity_mps: velocity,
                heading_deg: heading,
                on_ground,
                last_contact,
            });

            if records.len() >= limit {
                break;
            }
        }
    }

    FlightSnapshot {
        provider: provider.to_string(),
        source: "live".to_string(),
        timestamp_ms: time,
        flights: records,
    }
}

fn parse_adsb_lol(value: serde_json::Value, provider: &str, limit: usize) -> FlightSnapshot {
    let now_ms = value
        .get("now")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(now_epoch_millis);
    let mut records = Vec::new();
    if let Some(states) = value.get("ac").and_then(|v| v.as_array()) {
        for (idx, state) in states.iter().enumerate() {
            let lat = value_as_f64(state.get("lat"));
            let lon = value_as_f64(state.get("lon"));
            let (Some(lat), Some(lon)) = (lat, lon) else { continue };
            let alt_geom = value_as_f64(state.get("alt_geom"));
            let alt_baro_raw = state.get("alt_baro");
            let alt_baro = value_as_f64(alt_baro_raw);
            let altitude_m = alt_geom
                .or(alt_baro)
                .map(|feet| feet * 0.3048);
            let velocity_mps = value_as_f64(state.get("gs"))
                .map(|knots| knots * 0.514444);
            let heading = value_as_f64(state.get("track"));
            let on_ground = matches!(alt_baro_raw, Some(serde_json::Value::String(text)) if text == "ground")
                || alt_baro.map(|value| value <= 0.5).unwrap_or(false);
            let callsign = value_as_string(state.get("flight"));
            let origin = value_as_string(state.get("r"))
                .or_else(|| value_as_string(state.get("t")));
            let hex = value_as_string(state.get("hex"));
            let id = if let Some(hex) = hex.as_deref() {
                format!("{provider}:{hex}")
            } else if let Some(callsign) = callsign.as_deref() {
                format!("{provider}:{callsign}")
            } else {
                format!("{provider}:unknown-{idx}")
            };
            let last_contact = value_as_f64(state.get("seen_pos"))
                .map(|seen| now_ms.saturating_sub((seen * 1000.0) as u64))
                .map(|ms| (ms / 1000) as i64);

            records.push(FlightState {
                id,
                callsign,
                origin_country: origin,
                lat,
                lon,
                altitude_m,
                velocity_mps,
                heading_deg: heading,
                on_ground,
                last_contact,
            });

            if records.len() >= limit {
                break;
            }
        }
    }

    FlightSnapshot {
        provider: provider.to_string(),
        source: "live".to_string(),
        timestamp_ms: now_ms,
        flights: records,
    }
}

#[get("/ui/flights")]
pub async fn flights(
    state: web::Data<AppState>,
    query: web::Query<FlightQuery>,
) -> Result<HttpResponse, Error> {
    if !state.flight_enabled {
        return Err(actix_web::error::ErrorNotFound("flight overlay disabled"));
    }

    let limit = query
        .limit
        .unwrap_or(state.flight_max_flights)
        .clamp(1, state.flight_max_flights);
    let sample_limit = limit.min(state.flight_sample_count);

    let now = std::time::Instant::now();
    let mut cached_payload: Option<FlightSnapshot> = None;
    let mut cached_age: Option<std::time::Duration> = None;
    if let Ok(cache) = state.flight_cache.lock() {
        cached_payload = cache.payload.clone();
        cached_age = cache.last_fetch.map(|last_fetch| now.duration_since(last_fetch));
    }
    if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
        if age < state.flight_min_interval && payload.flights.len() >= limit {
            let mut cached = payload.clone();
            cached.source = "cache".to_string();
            return Ok(HttpResponse::Ok().json(cached));
        }
    }

    let provider_key = state.flight_provider.trim().to_ascii_lowercase();
    let is_adsb = provider_key.contains("adsb");
    let bbox = (
        query.lamin.map(clamp_lat),
        query.lomin.map(clamp_lon),
        query.lamax.map(clamp_lat),
        query.lomax.map(clamp_lon),
    );
    let bbox_resolved = match bbox {
        (Some(lamin), Some(lomin), Some(lamax), Some(lomax))
            if lamin < lamax && lomin < lomax =>
        {
            Some((lamin, lomin, lamax, lomax))
        }
        _ => None,
    };
    let center_hint = bbox_resolved.map(|(lamin, lomin, lamax, lomax)| {
        ((lamin + lamax) / 2.0, (lomin + lomax) / 2.0)
    });
    let url = if is_adsb {
        build_adsb_url(&state.flight_base_url, bbox_resolved)?
    } else {
        let mut url = reqwest::Url::parse(&state.flight_base_url)
            .map_err(|_| actix_web::error::ErrorBadRequest("invalid flight base url"))?;
        if let Some((lamin, lomin, lamax, lomax)) = bbox_resolved {
            url.query_pairs_mut()
                .append_pair("lamin", &lamin.to_string())
                .append_pair("lomin", &lomin.to_string())
                .append_pair("lamax", &lamax.to_string())
                .append_pair("lomax", &lomax.to_string());
        }
        url
    };

    let mut request = state
        .tile_client
        .get(url)
        .header("Accept", "application/json");
    if let (Some(user), Some(pass)) = (
        state.flight_username.as_deref(),
        state.flight_password.as_deref(),
    ) {
        request = request.basic_auth(user, Some(pass));
    }

    let response = request.send().await;
    let payload = match response {
        Ok(response) if response.status().is_success() => {
            let value = response
                .json::<serde_json::Value>()
                .await
                .map_err(actix_web::error::ErrorBadGateway)?;
            if is_adsb {
                parse_adsb_lol(value, &state.flight_provider, limit)
            } else {
                parse_opensky(value, &state.flight_provider, limit)
            }
        }
        Ok(response) => {
            tracing::warn!(status = %response.status(), "flight provider error");
            if state.flight_sample_enabled {
                let flights = if let Some((lat, lon)) = center_hint {
                    sample_flights_near(now_epoch_millis(), sample_limit, lat, lon)
                } else {
                    sample_flights(now_epoch_millis(), sample_limit)
                };
                FlightSnapshot {
                    provider: state.flight_provider.clone(),
                    source: "sample".to_string(),
                    timestamp_ms: now_epoch_millis(),
                    flights,
                }
            } else if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
                if age < state.flight_cache_ttl {
                    let mut cached = payload.clone();
                    cached.source = "cache".to_string();
                    return Ok(HttpResponse::Ok().json(cached));
                }
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "flight provider request failed");
            if state.flight_sample_enabled {
                let flights = if let Some((lat, lon)) = center_hint {
                    sample_flights_near(now_epoch_millis(), sample_limit, lat, lon)
                } else {
                    sample_flights(now_epoch_millis(), sample_limit)
                };
                FlightSnapshot {
                    provider: state.flight_provider.clone(),
                    source: "sample".to_string(),
                    timestamp_ms: now_epoch_millis(),
                    flights,
                }
            } else if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
                if age < state.flight_cache_ttl {
                    let mut cached = payload.clone();
                    cached.source = "cache".to_string();
                    return Ok(HttpResponse::Ok().json(cached));
                }
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
    };

    if let Ok(mut cache) = state.flight_cache.lock() {
        cache.last_fetch = Some(now);
        cache.payload = Some(payload.clone());
    }

    Ok(HttpResponse::Ok().json(payload))
}
