use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TileProvider {
    pub id: String,
    pub name: String,
    pub url: String,
    pub min_zoom: u8,
    pub max_zoom: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TileConfig {
    providers: Option<HashMap<String, TileProviderConfig>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TileProviderConfig {
    name: Option<String>,
    url: String,
    min_zoom: Option<u8>,
    max_zoom: Option<u8>,
}

pub fn tile_providers_from_value(value: &serde_json::Value) -> Option<HashMap<String, TileProvider>> {
    let config: TileConfig = serde_json::from_value(value.clone()).ok()?;
    let mut providers = HashMap::new();
    for (id, provider) in config.providers.unwrap_or_default() {
        if !provider.url.starts_with("http") {
            continue;
        }
        let name = provider.name.unwrap_or_else(|| id.clone());
        let min_zoom = provider.min_zoom.unwrap_or(0);
        let max_zoom = provider.max_zoom.unwrap_or(19);
        providers.insert(
            id.clone(),
            TileProvider {
                id,
                name,
                url: provider.url,
                min_zoom,
                max_zoom,
            },
        );
    }
    if providers.is_empty() {
        None
    } else {
        Some(providers)
    }
}

pub fn default_tile_providers() -> HashMap<String, TileProvider> {
    let mut providers = HashMap::new();
    providers.insert(
        "osm".to_string(),
        TileProvider {
            id: "osm".to_string(),
            name: "OSM Standard".to_string(),
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
            min_zoom: 0,
            max_zoom: 19,
        },
    );
    providers.insert(
        "hot".to_string(),
        TileProvider {
            id: "hot".to_string(),
            name: "OSM Humanitarian".to_string(),
            url: "https://a.tile.openstreetmap.fr/hot/{z}/{x}/{y}.png".to_string(),
            min_zoom: 0,
            max_zoom: 19,
        },
    );
    providers.insert(
        "opentopo".to_string(),
        TileProvider {
            id: "opentopo".to_string(),
            name: "OpenTopoMap".to_string(),
            url: "https://tile.opentopomap.org/{z}/{x}/{y}.png".to_string(),
            min_zoom: 0,
            max_zoom: 17,
        },
    );
    providers.insert(
        "nasa".to_string(),
        TileProvider {
            id: "nasa".to_string(),
            name: "NASA Blue Marble".to_string(),
            url: "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/BlueMarble_ShadedRelief/default/2013-12-01/GoogleMapsCompatible_Level8/{z}/{y}/{x}.jpg".to_string(),
            min_zoom: 0,
            max_zoom: 8,
        },
    );
    providers
}
