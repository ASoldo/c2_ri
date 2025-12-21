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
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "CallSign")]
    callsign: Option<String>,
    #[serde(rename = "SOG")]
    sog: Option<f64>,
    #[serde(rename = "COG")]
    cog: Option<f64>,
    #[serde(rename = "Heading")]
    heading: Option<f64>,
    #[serde(rename = "VesselType")]
    vessel_type: Option<i32>,
    #[serde(rename = "Status")]
    status: Option<i32>,
    #[serde(rename = "Length")]
    length: Option<f64>,
    #[serde(rename = "Width")]
    width: Option<f64>,
    #[serde(rename = "Draught")]
    draught: Option<f64>,
    #[serde(rename = "Destination")]
    destination: Option<String>,
    #[serde(rename = "BaseDateTime")]
    base_datetime: Option<i64>,
    #[serde(rename = "Latitude")]
    latitude: Option<f64>,
    #[serde(rename = "Longitude")]
    longitude: Option<f64>,
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

fn sanitize_heading(value: Option<f64>) -> Option<f64> {
    value.and_then(|heading| {
        if heading.is_finite() && heading >= 0.0 && heading < 360.0 {
            Some(heading)
        } else {
            None
        }
    })
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
        let lat = feature
            .geometry
            .as_ref()
            .map(|point| point.y)
            .or(feature.attributes.latitude);
        let lon = feature
            .geometry
            .as_ref()
            .map(|point| point.x)
            .or(feature.attributes.longitude);
        let (Some(lat), Some(lon)) = (lat, lon) else {
            continue;
        };
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        let id = if let Some(mmsi) = feature.attributes.mmsi {
            format!("{provider}:{mmsi}")
        } else if let Some(callsign) = feature.attributes.callsign.as_deref() {
            format!("{provider}:{callsign}")
        } else if let Some(object_id) = feature.attributes.object_id {
            format!("{provider}:obj-{object_id}")
        } else {
            format!("{provider}:unknown-{}", records.len() + 1)
        };
        let speed_knots = feature.attributes.sog.filter(|value| value.is_finite());
        let course_deg = feature.attributes.cog.filter(|value| value.is_finite());
        let heading_deg = sanitize_heading(feature.attributes.heading).or(course_deg);
        records.push(ShipState {
            id,
            mmsi: feature.attributes.mmsi,
            name: feature.attributes.name,
            callsign: feature.attributes.callsign,
            lat,
            lon,
            speed_knots,
            course_deg,
            heading_deg,
            vessel_type: feature.attributes.vessel_type,
            status: feature.attributes.status,
            length_m: feature.attributes.length.filter(|value| value.is_finite()),
            width_m: feature.attributes.width.filter(|value| value.is_finite()),
            draught_m: feature.attributes.draught.filter(|value| value.is_finite()),
            destination: feature.attributes.destination,
            last_report_ms: feature.attributes.base_datetime,
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
    let center_hint = match (query.lamin, query.lomin, query.lamax, query.lomax) {
        (Some(lamin), Some(lomin), Some(lamax), Some(lomax))
            if lamin < lamax && lomin < lomax =>
        {
            Some(((lamin + lamax) / 2.0, (lomin + lomax) / 2.0))
        }
        _ => None,
    };

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

    let mut url = reqwest::Url::parse(&state.ship_base_url)
        .map_err(|_| actix_web::error::ErrorBadRequest("invalid ship base url"))?;
    let bbox = (
        query.lamin.map(clamp_lat),
        query.lomin.map(clamp_lon),
        query.lamax.map(clamp_lat),
        query.lomax.map(clamp_lon),
    );
    let mut query_pairs = url.query_pairs_mut();
    query_pairs.append_pair("where", "1=1");
    query_pairs.append_pair("outFields", "*");
    query_pairs.append_pair("f", "json");
    query_pairs.append_pair("resultRecordCount", &limit.to_string());
    query_pairs.append_pair("outSR", "4326");
    query_pairs.append_pair("returnGeometry", "true");
    if let (Some(lamin), Some(lomin), Some(lamax), Some(lomax)) = bbox {
        if lamin < lamax && lomin < lomax {
            let geometry = format!("{lomin},{lamin},{lomax},{lamax}");
            query_pairs.append_pair("geometry", &geometry);
            query_pairs.append_pair("geometryType", "esriGeometryEnvelope");
            query_pairs.append_pair("inSR", "4326");
            query_pairs.append_pair("spatialRel", "esriSpatialRelIntersects");
        }
    }
    drop(query_pairs);

    let response = state
        .tile_client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await;

    let payload = match response {
        Ok(response) if response.status().is_success() => {
            match response.text().await {
                Ok(body) => match serde_json::from_str::<EsriResponse>(&body) {
                    Ok(value) => {
                        let mut snapshot = parse_esri_response(value, &state.ship_provider, limit);
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
                },
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
