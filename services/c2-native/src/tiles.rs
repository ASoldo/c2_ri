use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use image::{imageops, GenericImage, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileKind {
    Base,
    Weather,
    Sea,
}

impl TileKind {
    pub fn all() -> std::collections::HashSet<TileKind> {
        let mut set = std::collections::HashSet::new();
        set.insert(TileKind::Base);
        set.insert(TileKind::Weather);
        set.insert(TileKind::Sea);
        set
    }
}

#[derive(Debug, Clone)]
pub struct TileResult {
    pub request_id: u64,
    pub kind: TileKind,
    pub zoom: u8,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub valid: bool,
}

#[derive(Debug, Clone)]
pub struct TileRequest {
    pub request_id: u64,
    pub zoom: u8,
    pub provider: String,
    pub weather_field: String,
    pub sea_field: String,
}

pub struct TileFetcher {
    base_url: String,
    layer_size: u32,
    sender: Sender<TileResult>,
}

impl TileFetcher {
    pub fn new(layer_size: u32) -> (Self, Receiver<TileResult>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        let base_url = std::env::var("C2_NATIVE_TILE_BASE")
            .unwrap_or_else(|_| "https://c2.local".to_string())
            .trim_end_matches('/')
            .to_string();
        (
            Self {
                base_url,
                layer_size,
                sender,
            },
            receiver,
        )
    }

    pub fn request_all(&self, request: TileRequest) {
        self.request(TileKind::Base, request.clone());
        self.request(TileKind::Weather, request.clone());
        self.request(TileKind::Sea, request);
    }

    pub fn request(&self, kind: TileKind, request: TileRequest) {
        let base_url = self.base_url.clone();
        let sender = self.sender.clone();
        let layer_size = self.layer_size;
        thread::spawn(move || {
            let result = match kind {
                TileKind::Base => build_layer(
                    kind,
                    &request,
                    layer_size,
                    &format!("{base_url}/ui/tiles/{}/{{z}}/{{x}}/{{y}}", request.provider),
                    None,
                ),
                TileKind::Weather => build_layer(
                    kind,
                    &request,
                    layer_size,
                    &format!("{base_url}/ui/tiles/weather/{{z}}/{{x}}/{{y}}"),
                    Some(&request.weather_field),
                ),
                TileKind::Sea => build_layer(
                    kind,
                    &request,
                    layer_size,
                    &format!("{base_url}/ui/tiles/sea/{{z}}/{{x}}/{{y}}"),
                    Some(&request.sea_field),
                ),
            };
            let _ = sender.send(result);
        });
    }
}

fn build_layer(
    kind: TileKind,
    request: &TileRequest,
    layer_size: u32,
    template: &str,
    field: Option<&str>,
) -> TileResult {
    let tile_size = 256u32;
    let actual_zoom = request.zoom.min(3);
    let tiles = 1u32 << actual_zoom;
    let mosaic_size = tiles * tile_size;
    let mut mosaic = RgbaImage::new(mosaic_size, mosaic_size);
    let allow_insecure = std::env::var("C2_NATIVE_TILE_INSECURE")
        .ok()
        .as_deref()
        .map(|v| v == "1")
        .unwrap_or_else(|| template.contains(".local"));
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .danger_accept_invalid_certs(allow_insecure)
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return TileResult {
                request_id: request.request_id,
                kind,
                zoom: request.zoom,
                width: layer_size,
                height: layer_size,
                data: vec![0; (layer_size * layer_size * 4) as usize],
                valid: false,
            };
        }
    };

    let mut tiles_loaded = 0u32;
    for y in 0..tiles {
        for x in 0..tiles {
            let mut url = template
                .replace("{z}", &actual_zoom.to_string())
                .replace("{x}", &x.to_string())
                .replace("{y}", &y.to_string());
            if let Some(field) = field {
                let separator = if url.contains('?') { '&' } else { '?' };
                url.push(separator);
                url.push_str("field=");
                url.push_str(field);
            }
            if let Ok(response) = client.get(&url).send() {
                if let Ok(bytes) = response.bytes() {
                    if let Ok(image) = image::load_from_memory(&bytes) {
                        let tile = image.to_rgba8();
                        let tile = if tile.width() != tile_size || tile.height() != tile_size {
                            imageops::resize(&tile, tile_size, tile_size, imageops::FilterType::Triangle)
                        } else {
                            tile
                        };
                        if mosaic.copy_from(&tile, x * tile_size, y * tile_size).is_ok() {
                            tiles_loaded += 1;
                        }
                    }
                }
            }
        }
    }

    let final_image = if mosaic_size != layer_size {
        imageops::resize(&mosaic, layer_size, layer_size, imageops::FilterType::Triangle)
    } else {
        mosaic
    };

    TileResult {
        request_id: request.request_id,
        kind,
        zoom: actual_zoom,
        width: layer_size,
        height: layer_size,
        data: final_image.into_raw(),
        valid: tiles_loaded > 0,
    }
}
