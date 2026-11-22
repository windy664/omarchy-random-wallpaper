//! Bing's daily picture archive.
//!
//! The archive endpoint ignores the market and hands back the same set of
//! pictures for every request, so the only way to get more of them is to walk
//! the `idx` window: indices 0 to 7 are eight overlapping windows that
//! together hold about fifteen different pictures.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::http;
use crate::scenic::is_scenic;

/// One picture from the archive.
pub struct BingImage {
    pub url: String,
    pub slug: String,
}

/// The windows to walk.
const WINDOWS: usize = 8;

/// How many pictures each window asks for.
const PER_WINDOW: usize = 8;

/// The slug of a picture: the last part of its url, cleaned up.
///
/// The dot is kept, so `OHR.AlpineBraies` survives and the same picture in
/// two languages can be told apart by name and not only by content.
fn slug_of(urlbase: &str) -> String {
    let tail = urlbase.rsplit('/').next().unwrap_or(urlbase);

    tail.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect()
}

/// Fetch the whole window range, newest set first.
pub fn fetch() -> Vec<BingImage> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut images = Vec::new();

    for index in 0..WINDOWS {
        let url = format!(
            "https://www.bing.com/HPImageArchive.aspx?format=js&idx={index}&n={PER_WINDOW}"
        );

        let Ok(body) = http::get_text(&url) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&body) else {
            continue;
        };
        let Some(list) = json["images"].as_array() else {
            continue;
        };

        for image in list {
            let Some(urlbase) = image["urlbase"].as_str() else {
                continue;
            };

            let slug = slug_of(urlbase);
            if slug.is_empty() || !seen.insert(slug.clone()) {
                continue;
            }

            images.push(BingImage {
                url: format!("https://www.bing.com{urlbase}_UHD.jpg"),
                slug,
            });
        }
    }

    images
}

/// Download a picture into the cache and return its path.
///
/// Pictures the rules reject are not downloaded at all: they would only sit
/// in the cache, never be shown.
pub fn download(image: &BingImage, cache: &Path) -> Option<PathBuf> {
    if !is_scenic(&image.slug) {
        return None;
    }

    let path = cache.join(format!("bing-{}.jpg", image.slug));

    if path.metadata().map(|meta| meta.len() > 0).unwrap_or(false) {
        return Some(path);
    }

    match http::download(&image.url, &path) {
        Ok(()) if path.metadata().map(|meta| meta.len() > 0).unwrap_or(false) => Some(path),
        _ => {
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slug_keeps_the_bing_id() {
        assert_eq!(
            slug_of("/th?id=OHR.AlpineBraies_ZH-CN123"),
            "thidOHR.AlpineBraies_ZH-CN123"
        );
        assert_eq!(
            slug_of("/th?id=OHR.Foo_EN-US1?pid=hp"),
            "thidOHR.Foo_EN-US1pidhp"
        );
    }
}
