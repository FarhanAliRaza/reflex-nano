//! Content-addressed immutable assets and Rust gzip compression.
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use flate2::{write::GzEncoder, Compression};
use std::{io::Write, sync::OnceLock};

pub(crate) struct Asset {
    pub path: String,
    body: &'static str,
    gzip: Vec<u8>,
    etag: String,
    mime: &'static str,
}
fn gzip(bytes: &[u8], level: Compression) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), level);
    encoder.write_all(bytes).expect("memory compression");
    encoder.finish().expect("memory compression")
}
fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
impl Asset {
    fn new(name: &str, body: &'static str, mime: &'static str) -> Self {
        let hash = format!("{:016x}", fingerprint(body.as_bytes()));
        Self {
            path: format!("/__nano/asset/{hash}/{name}"),
            body,
            gzip: gzip(body.as_bytes(), Compression::best()),
            etag: format!("W/\"{hash}\""),
            mime,
        }
    }
    pub fn response(&self, headers: &HeaderMap, immutable: bool) -> Response {
        let compressed = accepts_gzip(headers);
        let mut response = if headers
            .get("if-none-match")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|value| {
                value.split(',').any(|s| {
                    s.trim().trim_start_matches("W/") == self.etag.trim_start_matches("W/")
                        || s.trim() == "*"
                })
            }) {
            StatusCode::NOT_MODIFIED.into_response()
        } else if compressed {
            ([("content-type", self.mime)], self.gzip.clone()).into_response()
        } else {
            ([("content-type", self.mime)], self.body).into_response()
        };
        let target = response.headers_mut();
        target.insert("etag", self.etag.parse().unwrap());
        target.insert("vary", "Accept-Encoding".parse().unwrap());
        target.insert(
            "cache-control",
            if immutable {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            }
            .parse()
            .unwrap(),
        );
        target.insert("x-content-type-options", "nosniff".parse().unwrap());
        if compressed && response.status() != StatusCode::NOT_MODIFIED {
            response
                .headers_mut()
                .insert("content-encoding", "gzip".parse().unwrap());
        }
        response
    }
}
include!(concat!(env!("OUT_DIR"), "/react_assets.rs"));
pub(crate) fn react_assets() -> &'static Vec<Asset> {
    static ASSETS: OnceLock<Vec<Asset>> = OnceLock::new();
    ASSETS.get_or_init(|| {
        let mut combined = Vec::new();
        for (name, body) in REACT_FILES {
            combined.extend_from_slice(name.as_bytes());
            combined.extend_from_slice(body.as_bytes());
        }
        let hash = fingerprint(&combined);
        REACT_FILES
            .iter()
            .map(|(name, body)| {
                let mut asset = Asset::new(
                    name,
                    body,
                    if name.ends_with(".css") {
                        "text/css; charset=utf-8"
                    } else {
                        "text/javascript; charset=utf-8"
                    },
                );
                asset.path = format!("/__nano/react/{hash:016x}/{name}");
                asset
            })
            .collect()
    })
}
pub(crate) fn react_entry() -> &'static Asset {
    react_assets()
        .iter()
        .find(|a| a.path.ends_with("/react.js"))
        .unwrap()
}
pub(crate) fn client() -> &'static Asset {
    static ASSET: OnceLock<Asset> = OnceLock::new();
    ASSET.get_or_init(|| {
        Asset::new(
            "client.js",
            include_str!(concat!(env!("OUT_DIR"), "/client.min.js")),
            "text/javascript; charset=utf-8",
        )
    })
}
pub(crate) fn css() -> &'static Asset {
    static ASSET: OnceLock<Asset> = OnceLock::new();
    ASSET.get_or_init(|| {
        Asset::new(
            "style.css",
            include_str!("style.css"),
            "text/css; charset=utf-8",
        )
    })
}
pub(crate) fn accepts_gzip(headers: &HeaderMap) -> bool {
    let Some(value) = headers.get("accept-encoding").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let mut wildcard = false;
    for entry in value.split(',') {
        let mut parts = entry.trim().split(';');
        let name = parts.next().unwrap().trim();
        let quality = parts
            .find_map(|v| {
                v.trim()
                    .strip_prefix("q=")
                    .map(|q| q.parse::<f64>().unwrap_or(0.0))
            })
            .unwrap_or(1.0);
        if name.eq_ignore_ascii_case("gzip") {
            return quality > 0.0;
        }
        if name == "*" {
            wildcard = quality > 0.0;
        }
    }
    wildcard
}
pub(crate) fn document(body: String, headers: &HeaderMap) -> Response {
    let mut response = if body.len() >= 1024 && accepts_gzip(headers) {
        let bytes = gzip(body.as_bytes(), Compression::fast());
        let mut response = Response::new(Body::from(bytes));
        response
            .headers_mut()
            .insert("content-encoding", "gzip".parse().unwrap());
        response
    } else {
        Response::new(Body::from(body))
    };
    response
        .headers_mut()
        .insert("content-type", "text/html; charset=utf-8".parse().unwrap());
    response
        .headers_mut()
        .insert("vary", "Accept-Encoding".parse().unwrap());
    response
}
