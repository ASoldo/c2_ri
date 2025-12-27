use actix_web::{get, web, Error, HttpResponse};
use serde::Deserialize;

use crate::ships::{now_epoch_millis, sample_ships, sample_ships_near, ShipSnapshot, ShipState};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ShipQuery {
    lamin: Option<f64>,
    lomin: Option<f64>,
    lamax: Option<f64>,
    lomax: Option<f64>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct EsriResponse {
    features: Vec<EsriFeature>,
}

#[derive(Deserialize)]
struct EsriFeature {
    #[serde(default)]
    attributes: EsriAttributes,
    geometry: Option<EsriPoint>,
}

#[derive(Deserialize, Default)]
struct EsriAttributes {
    #[serde(rename = "OBJECTID")]
    object_id: Option<i64>,
    #[serde(rename = "MMSI")]
    mmsi: Option<u64>,
    #[serde(rename = "AIS_MMSI")]
    ais_mmsi: Option<u64>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "AIS_NAME")]
    ais_name: Option<String>,
    #[serde(rename = "CallSign")]
    callsign: Option<String>,
    #[serde(rename = "AIS_CALLSIGN")]
    ais_callsign: Option<String>,
    #[serde(rename = "SOG")]
    sog: Option<f64>,
    #[serde(rename = "AIS_SPEED")]
    ais_sog: Option<f64>,
    #[serde(rename = "COG")]
    cog: Option<f64>,
    #[serde(rename = "AIS_COURSE")]
    ais_cog: Option<f64>,
    #[serde(rename = "Heading")]
    heading: Option<f64>,
    #[serde(rename = "AIS_HEADING")]
    ais_heading: Option<f64>,
    #[serde(rename = "VesselType")]
    vessel_type: Option<i32>,
    #[serde(rename = "AIS_TYPE")]
    ais_type: Option<i32>,
    #[serde(rename = "Status")]
    status: Option<i32>,
    #[serde(rename = "AIS_NAVSTAT")]
    ais_status: Option<i32>,
    #[serde(rename = "Length")]
    length: Option<f64>,
    #[serde(rename = "Width")]
    width: Option<f64>,
    #[serde(rename = "AIS_A")]
    ais_a: Option<f64>,
    #[serde(rename = "AIS_B")]
    ais_b: Option<f64>,
    #[serde(rename = "AIS_C")]
    ais_c: Option<f64>,
    #[serde(rename = "AIS_D")]
    ais_d: Option<f64>,
    #[serde(rename = "Draught")]
    draught: Option<f64>,
    #[serde(rename = "AIS_DRAUGHT")]
    ais_draught: Option<f64>,
    #[serde(rename = "Destination")]
    destination: Option<String>,
    #[serde(rename = "AIS_DESTINATION")]
    ais_destination: Option<String>,
    #[serde(rename = "BaseDateTime")]
    base_datetime: Option<i64>,
    #[serde(rename = "AIS_TIMESTAMP")]
    ais_timestamp: Option<i64>,
    #[serde(rename = "Latitude")]
    latitude: Option<f64>,
    #[serde(rename = "Longitude")]
    longitude: Option<f64>,
    #[serde(rename = "AIS_LATITUDE")]
    ais_latitude: Option<f64>,
    #[serde(rename = "AIS_LONGITUDE")]
    ais_longitude: Option<f64>,
}

#[derive(Deserialize)]
struct EsriPoint {
    x: f64,
    y: f64,
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

fn value_as_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_i64(),
        Some(serde_json::Value::String(text)) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn value_as_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r_km = 6371.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + (dlon / 2.0).sin().powi(2) * lat1.cos() * lat2.cos();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r_km * c
}

fn center_radius_km(
    bbox: Option<(f64, f64, f64, f64)>,
) -> (f64, f64, f64) {
    let (lat, lon) = if let Some((lamin, lomin, lamax, lomax)) = bbox {
        ((lamin + lamax) / 2.0, (lomin + lomax) / 2.0)
    } else {
        (0.0, 0.0)
    };
    let radius_km: f64 = if let Some((lamin, lomin, lamax, lomax)) = bbox {
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
        let mut max_km: f64 = 0.0;
        for (clat, clon) in samples {
            max_km = max_km.max(haversine_km(lat, lon, clat, clon));
        }
        max_km
    } else {
        180.0
    };
    (lat, lon, radius_km.clamp(10.0, 20000.0))
}

fn build_aishub_url(
    base_url: &str,
    username: &str,
    bbox: Option<(f64, f64, f64, f64)>,
) -> Result<reqwest::Url, Error> {
    let (lat, lon, radius_km) = center_radius_km(bbox);
    let mut url = reqwest::Url::parse(base_url)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid ship base url"))?;
    url.query_pairs_mut()
        .append_pair("username", username)
        .append_pair("format", "1")
        .append_pair("output", "json")
        .append_pair("compress", "0")
        .append_pair("lat", &format!("{lat:.4}"))
        .append_pair("lon", &format!("{lon:.4}"))
        .append_pair("radius", &format!("{radius_km:.1}"));
    Ok(url)
}

fn sanitize_heading(value: Option<f64>) -> Option<f64> {
    value.and_then(|heading| {
        if heading.is_finite() && heading >= 0.0 && heading < 360.0 {
            Some(heading)
        } else {
            None
        }
    })
}

fn resolve_dimension(
    primary: Option<f64>,
    a: Option<f64>,
    b: Option<f64>,
) -> Option<f64> {
    if let Some(value) = primary.filter(|value| value.is_finite() && *value > 0.0) {
        return Some(value);
    }
    let total = a.unwrap_or(0.0) + b.unwrap_or(0.0);
    if total > 0.0 {
        Some(total)
    } else {
        None
    }
}

fn parse_esri_response(
    response: EsriResponse,
    provider: &str,
    limit: usize,
) -> ShipSnapshot {
    let mut records = Vec::with_capacity(limit.max(1));
    for feature in response.features.into_iter() {
        if records.len() >= limit {
            break;
        }
        let attrs = feature.attributes;
        let lat = feature
            .geometry
            .as_ref()
            .map(|point| point.y)
            .or(attrs.latitude)
            .or(attrs.ais_latitude);
        let lon = feature
            .geometry
            .as_ref()
            .map(|point| point.x)
            .or(attrs.longitude)
            .or(attrs.ais_longitude);
        let (Some(lat), Some(lon)) = (lat, lon) else {
            continue;
        };
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        let mmsi = attrs.mmsi.or(attrs.ais_mmsi);
        let callsign = attrs.callsign.or(attrs.ais_callsign);
        let name = attrs.name.or(attrs.ais_name);
        let destination = attrs.destination.or(attrs.ais_destination);
        let id = if let Some(mmsi) = mmsi {
            format!("{provider}:{mmsi}")
        } else if let Some(callsign) = callsign.as_deref() {
            format!("{provider}:{callsign}")
        } else if let Some(object_id) = attrs.object_id {
            format!("{provider}:obj-{object_id}")
        } else {
            format!("{provider}:unknown-{}", records.len() + 1)
        };
        let speed_knots = attrs
            .sog
            .or(attrs.ais_sog)
            .filter(|value| value.is_finite());
        let course_deg = attrs
            .cog
            .or(attrs.ais_cog)
            .filter(|value| value.is_finite());
        let heading_deg =
            sanitize_heading(attrs.heading.or(attrs.ais_heading)).or(course_deg);
        let vessel_type = attrs.vessel_type.or(attrs.ais_type);
        let status = attrs.status.or(attrs.ais_status);
        let length_m = resolve_dimension(attrs.length, attrs.ais_a, attrs.ais_b)
            .filter(|value| value.is_finite());
        let width_m = resolve_dimension(attrs.width, attrs.ais_c, attrs.ais_d)
            .filter(|value| value.is_finite());
        let draught_m = attrs
            .draught
            .or(attrs.ais_draught)
            .filter(|value| value.is_finite());
        let last_report_ms = attrs.base_datetime.or(attrs.ais_timestamp);
        records.push(ShipState {
            id,
            mmsi,
            name,
            callsign,
            lat,
            lon,
            speed_knots,
            course_deg,
            heading_deg,
            vessel_type,
            status,
            length_m,
            width_m,
            draught_m,
            destination,
            last_report_ms,
        });
    }
    ShipSnapshot {
        provider: provider.to_string(),
        source: "live".to_string(),
        timestamp_ms: now_epoch_millis(),
        ships: records,
    }
}

fn parse_aishub_response(
    response: serde_json::Value,
    provider: &str,
    limit: usize,
) -> ShipSnapshot {
    let mut records = Vec::with_capacity(limit.max(1));
    let entries = match response {
        serde_json::Value::Array(list) => list,
        serde_json::Value::Object(mut map) => map
            .remove("ships")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    for (idx, entry) in entries.into_iter().enumerate() {
        if records.len() >= limit {
            break;
        }
        let Some(obj) = entry.as_object() else { continue };
        let lat = value_as_f64(
            obj.get("lat")
                .or_else(|| obj.get("LAT"))
                .or_else(|| obj.get("latitude"))
                .or_else(|| obj.get("LATITUDE")),
        );
        let lon = value_as_f64(
            obj.get("lon")
                .or_else(|| obj.get("LON"))
                .or_else(|| obj.get("longitude"))
                .or_else(|| obj.get("LONGITUDE")),
        );
        let (Some(lat), Some(lon)) = (lat, lon) else { continue };
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        let mmsi = value_as_f64(obj.get("MMSI").or_else(|| obj.get("mmsi")))
            .map(|value| value as u64);
        let name = value_as_string(obj.get("NAME").or_else(|| obj.get("name")));
        let callsign = value_as_string(
            obj.get("CALLSIGN")
                .or_else(|| obj.get("callsign"))
                .or_else(|| obj.get("CallSign")),
        );
        let destination = value_as_string(
            obj.get("DESTINATION")
                .or_else(|| obj.get("destination"))
                .or_else(|| obj.get("Destination")),
        );
        let speed_knots = value_as_f64(obj.get("SOG").or_else(|| obj.get("sog")))
            .filter(|value| value.is_finite());
        let course_deg = value_as_f64(obj.get("COG").or_else(|| obj.get("cog")))
            .filter(|value| value.is_finite());
        let heading_deg = sanitize_heading(
            value_as_f64(obj.get("HEADING").or_else(|| obj.get("heading"))),
        )
        .or(course_deg);
        let vessel_type = value_as_f64(
            obj.get("TYPE")
                .or_else(|| obj.get("type"))
                .or_else(|| obj.get("VesselType")),
        )
        .map(|value| value as i32);
        let draught_m = value_as_f64(
            obj.get("DRAUGHT")
                .or_else(|| obj.get("draught"))
                .or_else(|| obj.get("Draught")),
        );
        let timestamp = value_as_i64(
            obj.get("timestamp")
                .or_else(|| obj.get("TIMESTAMP"))
                .or_else(|| obj.get("time"))
                .or_else(|| obj.get("TIME")),
        );
        let id = if let Some(mmsi) = mmsi {
            format!("{provider}:{mmsi}")
        } else if let Some(callsign) = callsign.as_deref() {
            format!("{provider}:{callsign}")
        } else {
            format!("{provider}:unknown-{}", idx + 1)
        };

        records.push(ShipState {
            id,
            mmsi,
            name,
            callsign,
            lat,
            lon,
            speed_knots,
            course_deg,
            heading_deg,
            vessel_type,
            status: None,
            length_m: None,
            width_m: None,
            draught_m,
            destination,
            last_report_ms: timestamp,
        });
    }

    ShipSnapshot {
        provider: provider.to_string(),
        source: "live".to_string(),
        timestamp_ms: now_epoch_millis(),
        ships: records,
    }
}

#[get("/ui/ships")]
pub async fn ships(
    state: web::Data<AppState>,
    query: web::Query<ShipQuery>,
) -> Result<HttpResponse, Error> {
    if !state.ship_enabled {
        return Err(actix_web::error::ErrorNotFound("ship overlay disabled"));
    }

    let limit = query
        .limit
        .unwrap_or(state.ship_max_ships)
        .clamp(1, state.ship_max_ships);
    let sample_limit = limit.min(state.ship_sample_count);
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

    let now = std::time::Instant::now();
    let mut cached_payload: Option<ShipSnapshot> = None;
    let mut cached_age: Option<std::time::Duration> = None;
    if let Ok(cache) = state.ship_cache.lock() {
        cached_payload = cache.payload.clone();
        cached_age = cache.last_fetch.map(|last_fetch| now.duration_since(last_fetch));
    }
    if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
        if age < state.ship_min_interval {
            let mut cached = payload.clone();
            cached.source = "cache".to_string();
            return Ok(HttpResponse::Ok().json(cached));
        }
    }

    let sample_payload = || {
        let now_ms = now_epoch_millis();
        let ships = if let Some((lat, lon)) = center_hint {
            sample_ships_near(now_ms, sample_limit, lat, lon)
        } else {
            sample_ships(now_ms, sample_limit)
        };
        ShipSnapshot {
            provider: state.ship_provider.clone(),
            source: "sample".to_string(),
            timestamp_ms: now_ms,
            ships,
        }
    };
    let fallback_payload = || {
        if state.ship_sample_enabled {
            return Some(sample_payload());
        }
        if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
            if age < state.ship_cache_ttl {
                let mut cached = payload.clone();
                cached.source = "cache".to_string();
                return Some(cached);
            }
        }
        None
    };

    let provider_key = state.ship_provider.trim().to_ascii_lowercase();
    let use_aishub = provider_key.contains("aishub")
        && state
            .ship_username
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

    let response = if use_aishub {
        let username = state.ship_username.as_deref().unwrap_or_default();
        let url = build_aishub_url(&state.ship_base_url, username, bbox_resolved)?;
        state
            .tile_client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
    } else {
        let mut url = reqwest::Url::parse(&state.ship_base_url)
            .map_err(|_| actix_web::error::ErrorBadRequest("invalid ship base url"))?;
        let mut query_pairs = url.query_pairs_mut();
        query_pairs.append_pair("where", "1=1");
        query_pairs.append_pair("outFields", "*");
        query_pairs.append_pair("f", "json");
        query_pairs.append_pair("resultRecordCount", &limit.to_string());
        query_pairs.append_pair("outSR", "4326");
        query_pairs.append_pair("returnGeometry", "true");
        if let Some((lamin, lomin, lamax, lomax)) = bbox_resolved {
            let geometry = format!("{lomin},{lamin},{lomax},{lamax}");
            query_pairs.append_pair("geometry", &geometry);
            query_pairs.append_pair("geometryType", "esriGeometryEnvelope");
            query_pairs.append_pair("inSR", "4326");
            query_pairs.append_pair("spatialRel", "esriSpatialRelIntersects");
        }
        drop(query_pairs);
        state
            .tile_client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
    };

    let payload = match response {
        Ok(response) if response.status().is_success() => {
            match response.text().await {
                Ok(body) => {
                    if use_aishub {
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(value) => {
                                let mut snapshot =
                                    parse_aishub_response(value, &state.ship_provider, limit);
                                if snapshot.ships.is_empty() && state.ship_sample_enabled {
                                    snapshot = sample_payload();
                                }
                                snapshot
                            }
                            Err(err) => {
                                tracing::warn!(error = %err, "ship provider parse failed");
                                if let Some(payload) = fallback_payload() {
                                    payload
                                } else {
                                    return Ok(HttpResponse::build(
                                        actix_web::http::StatusCode::BAD_GATEWAY,
                                    )
                                    .finish());
                                }
                            }
                        }
                    } else {
                        match serde_json::from_str::<EsriResponse>(&body) {
                            Ok(value) => {
                                let mut snapshot =
                                    parse_esri_response(value, &state.ship_provider, limit);
                                if snapshot.ships.is_empty() && state.ship_sample_enabled {
                                    snapshot = sample_payload();
                                }
                                snapshot
                            }
                            Err(err) => {
                                tracing::warn!(error = %err, "ship provider parse failed");
                                if let Some(payload) = fallback_payload() {
                                    payload
                                } else {
                                    return Ok(HttpResponse::build(
                                        actix_web::http::StatusCode::BAD_GATEWAY,
                                    )
                                    .finish());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, "ship provider response read failed");
                    if let Some(payload) = fallback_payload() {
                        payload
                    } else {
                        return Ok(
                            HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish(),
                        );
                    }
                }
            }
        }
        Ok(response) => {
            tracing::warn!(status = %response.status(), "ship provider error");
            if let Some(payload) = fallback_payload() {
                payload
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "ship provider request failed");
            if let Some(payload) = fallback_payload() {
                payload
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
    };

    if let Ok(mut cache) = state.ship_cache.lock() {
        cache.last_fetch = Some(now);
        cache.payload = Some(payload.clone());
    }

    Ok(HttpResponse::Ok().json(payload))
}
