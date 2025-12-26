use actix_web::{get, web, Error, HttpResponse};
use serde::Deserialize;

use crate::satellites::{
    now_epoch_millis, sample_satellites, satellites_from_elements, SatelliteSnapshot,
};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SatelliteQuery {
    limit: Option<usize>,
}

#[get("/ui/satellites")]
pub async fn satellites(
    state: web::Data<AppState>,
    query: web::Query<SatelliteQuery>,
) -> Result<HttpResponse, Error> {
    if !state.satellite_enabled {
        return Err(actix_web::error::ErrorNotFound(
            "satellite overlay disabled",
        ));
    }

    let limit = query
        .limit
        .unwrap_or(state.satellite_max)
        .clamp(1, state.satellite_max);
    let sample_limit = limit.min(state.satellite_sample_count);

    let now = std::time::Instant::now();
    let mut cached_payload: Option<SatelliteSnapshot> = None;
    let mut cached_age: Option<std::time::Duration> = None;
    if let Ok(cache) = state.satellite_cache.lock() {
        cached_payload = cache.payload.clone();
        cached_age = cache.last_fetch.map(|last_fetch| now.duration_since(last_fetch));
    }
    let cache_label = match cached_age {
        Some(age) if age > state.satellite_cache_ttl => "cache-stale",
        _ => "cache",
    };
    let cached_response = |label: &str| {
        cached_payload.as_ref().map(|payload| {
            let mut cached = payload.clone();
            cached.source = label.to_string();
            cached
        })
    };
    if let (Some(payload), Some(age)) = (&cached_payload, cached_age) {
        if age < state.satellite_min_interval {
            let mut cached = payload.clone();
            cached.source = "cache".to_string();
            return Ok(HttpResponse::Ok().json(cached));
        }
    }

    let response = state
        .tile_client
        .get(&state.satellite_base_url)
        .header("Accept", "application/json")
        .timeout(state.satellite_timeout)
        .send()
        .await;

    let payload = match response {
        Ok(response) if response.status().is_success() => {
            let elements = response
                .json::<Vec<sgp4::Elements>>()
                .await
                .map_err(actix_web::error::ErrorBadGateway)?;
            let now_dt = sgp4::chrono::Utc::now().naive_utc();
            let satellites = satellites_from_elements(
                &elements,
                &now_dt,
                &state.satellite_provider,
                limit,
            );
            SatelliteSnapshot {
                provider: state.satellite_provider.clone(),
                source: "live".to_string(),
                timestamp_ms: now_epoch_millis(),
                satellites,
            }
        }
        Ok(response) => {
            tracing::warn!(status = %response.status(), "satellite provider error");
            if let Some(cached) = cached_response(cache_label) {
                cached
            } else if state.satellite_sample_enabled {
                SatelliteSnapshot {
                    provider: state.satellite_provider.clone(),
                    source: "sample".to_string(),
                    timestamp_ms: now_epoch_millis(),
                    satellites: sample_satellites(now_epoch_millis(), sample_limit),
                }
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, "satellite provider request failed");
            if let Some(cached) = cached_response(cache_label) {
                cached
            } else if state.satellite_sample_enabled {
                SatelliteSnapshot {
                    provider: state.satellite_provider.clone(),
                    source: "sample".to_string(),
                    timestamp_ms: now_epoch_millis(),
                    satellites: sample_satellites(now_epoch_millis(), sample_limit),
                }
            } else {
                return Ok(HttpResponse::build(actix_web::http::StatusCode::BAD_GATEWAY).finish());
            }
        }
    };

    if let Ok(mut cache) = state.satellite_cache.lock() {
        cache.last_fetch = Some(now);
        if payload.source != "sample" {
            cache.payload = Some(payload.clone());
        }
    }

    Ok(HttpResponse::Ok().json(payload))
}
