use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::env;

use image::imageops;

pub const TILE_SIZE: u32 = 256;
pub const MAP_TILE_CAPACITY: usize = 256;
pub const WEATHER_TILE_CAPACITY: usize = 128;
pub const SEA_TILE_CAPACITY: usize = 128;
const TILE_QUEUE_DEPTH: usize = 256;
const DEFAULT_WEATHER_BASE_URL: &str = "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best";
const DEFAULT_WEATHER_TILE_MATRIX_SET: &str = "GoogleMapsCompatible_Level6";
const DEFAULT_SEA_BASE_URL: &str = "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best";
const DEFAULT_SEA_TILE_MATRIX_SET: &str = "GoogleMapsCompatible_Level6";
const DEFAULT_WEATHER_FORMAT: &str = "png";
const DEFAULT_SEA_FORMAT: &str = "png";
const DEFAULT_WEATHER_TIME: &str = "default";
const DEFAULT_SEA_TIME: &str = "default";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    Base,
    Weather,
    Sea,
}

impl TileKind {
    fn index(self) -> usize {
        match self {
            TileKind::Base => 0,
            TileKind::Weather => 1,
            TileKind::Sea => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone)]
pub struct TileRequest {
    pub request_id: u64,
    pub kind: TileKind,
    pub key: TileKey,
    pub provider: String,
    pub provider_url: Option<String>,
    pub weather_field: String,
    pub sea_field: String,
    pub layer_index: u32,
}

#[derive(Debug, Clone)]
pub struct TileResult {
    pub request_id: u64,
    pub kind: TileKind,
    pub key: TileKey,
    pub layer_index: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub valid: bool,
}

#[derive(Clone)]
struct TileWorkerConfig {
    tile_base_url: Option<String>,
    weather_base_url: String,
    weather_tile_matrix_set: String,
    weather_time: Option<String>,
    weather_format: String,
    sea_base_url: String,
    sea_tile_matrix_set: String,
    sea_time: Option<String>,
    sea_format: String,
    user_agent: String,
    allow_insecure: bool,
}

pub struct TileFetcher {
    senders: Vec<SyncSender<TileRequest>>,
    next: usize,
    tracker: Arc<TileRequestTracker>,
}

impl TileFetcher {
    pub fn new(worker_count: usize) -> (Self, Receiver<TileResult>) {
        let worker_count = worker_count.max(1);
        let tile_base_url = env::var("C2_NATIVE_TILE_BASE")
            .ok()
            .map(|raw| raw.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty());
        let allow_insecure = env::var("C2_NATIVE_TILE_INSECURE")
            .ok()
            .and_then(|value| parse_env_bool(&value))
            .unwrap_or_else(|| {
                tile_base_url
                    .as_deref()
                    .is_some_and(|url| url.contains(".local"))
            });
        let user_agent = env::var("C2_NATIVE_TILE_USER_AGENT")
            .unwrap_or_else(|_| format!("C2-Walaris/{}", env!("CARGO_PKG_VERSION")));
        let weather_base_url = env::var("C2_NATIVE_WEATHER_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_WEATHER_BASE_URL.to_string());
        let weather_tile_matrix_set = env::var("C2_NATIVE_WEATHER_TILE_MATRIX_SET")
            .unwrap_or_else(|_| DEFAULT_WEATHER_TILE_MATRIX_SET.to_string());
        let weather_time = env::var("C2_NATIVE_WEATHER_DEFAULT_TIME")
            .unwrap_or_else(|_| DEFAULT_WEATHER_TIME.to_string());
        let weather_time = normalize_time(&weather_time);
        let weather_format = env::var("C2_NATIVE_WEATHER_DEFAULT_FORMAT")
            .unwrap_or_else(|_| DEFAULT_WEATHER_FORMAT.to_string());
        let weather_format =
            sanitize_format(&weather_format).unwrap_or_else(|| DEFAULT_WEATHER_FORMAT.to_string());

        let sea_base_url =
            env::var("C2_NATIVE_SEA_BASE_URL").unwrap_or_else(|_| DEFAULT_SEA_BASE_URL.to_string());
        let sea_tile_matrix_set = env::var("C2_NATIVE_SEA_TILE_MATRIX_SET")
            .unwrap_or_else(|_| DEFAULT_SEA_TILE_MATRIX_SET.to_string());
        let sea_time = env::var("C2_NATIVE_SEA_DEFAULT_TIME")
            .unwrap_or_else(|_| DEFAULT_SEA_TIME.to_string());
        let sea_time = normalize_time(&sea_time);
        let sea_format = env::var("C2_NATIVE_SEA_DEFAULT_FORMAT")
            .unwrap_or_else(|_| DEFAULT_SEA_FORMAT.to_string());
        let sea_format =
            sanitize_format(&sea_format).unwrap_or_else(|| DEFAULT_SEA_FORMAT.to_string());

        let worker_config = TileWorkerConfig {
            tile_base_url,
            weather_base_url: weather_base_url.trim_end_matches('/').to_string(),
            weather_tile_matrix_set,
            weather_time,
            weather_format,
            sea_base_url: sea_base_url.trim_end_matches('/').to_string(),
            sea_tile_matrix_set,
            sea_time,
            sea_format,
            user_agent,
            allow_insecure,
        };
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let mut senders = Vec::with_capacity(worker_count);
        let tracker = Arc::new(TileRequestTracker::new());
        for _ in 0..worker_count {
            let (job_tx, job_rx) = std::sync::mpsc::sync_channel(TILE_QUEUE_DEPTH);
            senders.push(job_tx);
            spawn_worker(job_rx, result_tx.clone(), worker_config.clone(), tracker.clone());
        }
        (
            Self {
                senders,
                next: 0,
                tracker,
            },
            result_rx,
        )
    }

    pub fn request(&mut self, request: TileRequest) -> bool {
        if self.senders.is_empty() {
            return false;
        }
        let idx = self.next % self.senders.len();
        self.next = self.next.wrapping_add(1);
        self.senders[idx].try_send(request).is_ok()
    }

    pub fn set_current_request_id(&self, kind: TileKind, request_id: u64) {
        self.tracker.set_current(kind, request_id);
    }
}

struct TileRequestTracker {
    current: [AtomicU64; 3],
}

impl TileRequestTracker {
    fn new() -> Self {
        Self {
            current: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    fn set_current(&self, kind: TileKind, request_id: u64) {
        self.current[kind.index()].store(request_id, Ordering::Relaxed);
    }

    fn is_current(&self, kind: TileKind, request_id: u64) -> bool {
        self.current[kind.index()].load(Ordering::Relaxed) == request_id
    }
}

fn parse_env_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn normalize_time(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lowered = value.to_ascii_lowercase();
    if lowered == "default" || lowered == "auto" || lowered == "latest" || lowered == "now" {
        return None;
    }
    if value.len() == 10
        && value.bytes().enumerate().all(|(idx, b)| {
            if idx == 4 || idx == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return Some(value.to_string());
    }
    Some(value.to_string())
}

fn sanitize_format(value: &str) -> Option<String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" => Some(value.trim().to_ascii_lowercase()),
        _ => None,
    }
}

fn base_tile_url(request: &TileRequest, config: &TileWorkerConfig) -> Option<String> {
    let z = request.key.zoom.to_string();
    let x = request.key.x.to_string();
    let y = request.key.y.to_string();
    if let Some(base_url) = config.tile_base_url.as_deref() {
        let template = format!(
            "{}/ui/tiles/{}/{{z}}/{{x}}/{{y}}",
            base_url, request.provider
        );
        return Some(
            template
                .replace("{z}", &z)
                .replace("{x}", &x)
                .replace("{y}", &y),
        );
    }
    let template = request
        .provider_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(
        template
            .replace("{z}", &z)
            .replace("{x}", &x)
            .replace("{y}", &y),
    )
}

fn gibs_tile_url(
    base_url: &str,
    field: &str,
    tile_matrix_set: &str,
    time: Option<&str>,
    z: u8,
    x: u32,
    y: u32,
    format: &str,
) -> String {
    let field = field.trim();
    if let Some(time) = time {
        format!(
            "{}/{}/default/{}/{}/{}/{}/{}.{}",
            base_url, field, time, tile_matrix_set, z, y, x, format
        )
    } else {
        format!(
            "{}/{}/default/{}/{}/{}/{}.{}",
            base_url, field, tile_matrix_set, z, y, x, format
        )
    }
}

fn weather_tile_url(request: &TileRequest, config: &TileWorkerConfig) -> Option<String> {
    if let Some(base_url) = config.tile_base_url.as_deref() {
        let mut url = format!(
            "{}/ui/tiles/weather/{}/{}/{}",
            base_url, request.key.zoom, request.key.x, request.key.y
        );
        let field = request.weather_field.trim();
        if !field.is_empty() {
            url.push_str("?field=");
            url.push_str(field);
        }
        return Some(url);
    }
    Some(gibs_tile_url(
        &config.weather_base_url,
        &request.weather_field,
        &config.weather_tile_matrix_set,
        config.weather_time.as_deref(),
        request.key.zoom,
        request.key.x,
        request.key.y,
        &config.weather_format,
    ))
}

fn sea_tile_url(request: &TileRequest, config: &TileWorkerConfig) -> Option<String> {
    if let Some(base_url) = config.tile_base_url.as_deref() {
        let mut url = format!(
            "{}/ui/tiles/sea/{}/{}/{}",
            base_url, request.key.zoom, request.key.x, request.key.y
        );
        let field = request.sea_field.trim();
        if !field.is_empty() {
            url.push_str("?field=");
            url.push_str(field);
        }
        return Some(url);
    }
    Some(gibs_tile_url(
        &config.sea_base_url,
        &request.sea_field,
        &config.sea_tile_matrix_set,
        config.sea_time.as_deref(),
        request.key.zoom,
        request.key.x,
        request.key.y,
        &config.sea_format,
    ))
}

fn spawn_worker(
    receiver: Receiver<TileRequest>,
    sender: Sender<TileResult>,
    worker_config: TileWorkerConfig,
    tracker: Arc<TileRequestTracker>,
) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .danger_accept_invalid_certs(worker_config.allow_insecure)
            .user_agent(worker_config.user_agent.clone())
            .build();
        while let Ok(request) = receiver.recv() {
            if !tracker.is_current(request.kind, request.request_id) {
                continue;
            }
            let result = match client.as_ref() {
                Ok(client) => fetch_tile(&request, &worker_config, client),
                Err(_) => empty_result(&request),
            };
            if tracker.is_current(request.kind, request.request_id) {
                let _ = sender.send(result);
            }
        }
    });
}

fn empty_result(request: &TileRequest) -> TileResult {
    TileResult {
        request_id: request.request_id,
        kind: request.kind,
        key: request.key,
        layer_index: request.layer_index,
        width: TILE_SIZE,
        height: TILE_SIZE,
        data: vec![0; (TILE_SIZE * TILE_SIZE * 4) as usize],
        valid: false,
    }
}

fn fetch_tile(
    request: &TileRequest,
    config: &TileWorkerConfig,
    client: &reqwest::blocking::Client,
) -> TileResult {
    let url = match request.kind {
        TileKind::Base => base_tile_url(request, config),
        TileKind::Weather => weather_tile_url(request, config),
        TileKind::Sea => sea_tile_url(request, config),
    };
    let Some(url) = url else {
        return empty_result(request);
    };

    let mut valid = false;
    let mut data = vec![0; (TILE_SIZE * TILE_SIZE * 4) as usize];
    if let Ok(response) = client.get(&url).send() {
        if let Ok(bytes) = response.bytes() {
            if let Ok(image) = image::load_from_memory(&bytes) {
                let tile = image.to_rgba8();
                let mut tile = if tile.width() != TILE_SIZE || tile.height() != TILE_SIZE {
                    imageops::resize(
                        &tile,
                        TILE_SIZE,
                        TILE_SIZE,
                        imageops::FilterType::CatmullRom,
                    )
                } else {
                    tile
                };
                if matches!(request.kind, TileKind::Base) {
                    for pixel in tile.pixels_mut() {
                        pixel.0[3] = 255;
                    }
                }
                data.copy_from_slice(tile.as_raw());
                valid = true;
            }
        }
    }

    TileResult {
        request_id: request.request_id,
        kind: request.kind,
        key: request.key,
        layer_index: request.layer_index,
        width: TILE_SIZE,
        height: TILE_SIZE,
        data,
        valid,
    }
}
