use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use image::imageops;

pub const TILE_SIZE: u32 = 256;
pub const MAP_TILE_CAPACITY: usize = 256;
pub const WEATHER_TILE_CAPACITY: usize = 128;
pub const SEA_TILE_CAPACITY: usize = 128;
const TILE_QUEUE_DEPTH: usize = 256;

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

pub struct TileFetcher {
    senders: Vec<SyncSender<TileRequest>>,
    next: usize,
    tracker: Arc<TileRequestTracker>,
}

impl TileFetcher {
    pub fn new(worker_count: usize) -> (Self, Receiver<TileResult>) {
        let worker_count = worker_count.max(1);
        let base_url = std::env::var("C2_NATIVE_TILE_BASE")
            .unwrap_or_else(|_| "https://c2.local".to_string())
            .trim_end_matches('/')
            .to_string();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        let mut senders = Vec::with_capacity(worker_count);
        let tracker = Arc::new(TileRequestTracker::new());
        for _ in 0..worker_count {
            let (job_tx, job_rx) = std::sync::mpsc::sync_channel(TILE_QUEUE_DEPTH);
            senders.push(job_tx);
            spawn_worker(job_rx, result_tx.clone(), base_url.clone(), tracker.clone());
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

fn spawn_worker(
    receiver: Receiver<TileRequest>,
    sender: Sender<TileResult>,
    base_url: String,
    tracker: Arc<TileRequestTracker>,
) {
    thread::spawn(move || {
        let allow_insecure = std::env::var("C2_NATIVE_TILE_INSECURE")
            .ok()
            .as_deref()
            .map(|v| v == "1")
            .unwrap_or_else(|| base_url.contains(".local"));
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .danger_accept_invalid_certs(allow_insecure)
            .build();
        while let Ok(request) = receiver.recv() {
            if !tracker.is_current(request.kind, request.request_id) {
                continue;
            }
            let result = match client.as_ref() {
                Ok(client) => fetch_tile(&request, &base_url, client),
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
    base_url: &str,
    client: &reqwest::blocking::Client,
) -> TileResult {
    let x = request.key.x;
    let y = request.key.y;
    let template = match request.kind {
        TileKind::Base => format!(
            "{}/ui/tiles/{}/{{z}}/{{x}}/{{y}}",
            base_url, request.provider
        ),
        TileKind::Weather => format!("{}/ui/tiles/weather/{{z}}/{{x}}/{{y}}", base_url),
        TileKind::Sea => format!("{}/ui/tiles/sea/{{z}}/{{x}}/{{y}}", base_url),
    };

    let mut url = template
        .replace("{z}", &request.key.zoom.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string());
    match request.kind {
        TileKind::Weather => {
            url.push_str("?field=");
            url.push_str(&request.weather_field);
        }
        TileKind::Sea => {
            url.push_str("?field=");
            url.push_str(&request.sea_field);
        }
        TileKind::Base => {}
    }

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
