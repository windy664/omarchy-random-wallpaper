//! Featured pictures from Wikimedia Commons.
//!
//! The categories hold a lot more than landscapes, so the titles are filtered
//! as well. The list of candidates is kept on disk and accumulates, because
//! the members of a category change and a single day is not enough to fill
//! the pool.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::http;
use crate::magick;
use crate::scenic::is_rejected;

/// The categories to draw from.
const CATEGORIES: &[&str] = &[
    "Featured pictures of nature",
    "Featured pictures of landscapes",
    "Featured pictures of skylines",
    "Featured pictures of cityscapes",
];

/// How many pictures to download in one run.
const DOWNLOAD_LIMIT: usize = 8;

/// The smallest picture worth putting on a screen.
const MIN_WIDTH: u64 = 2000;
const MIN_HEIGHT: u64 = 1000;

/// The aspect ratios a desktop wallpaper can take.
const MIN_RATIO: f64 = 1.2;
const MAX_RATIO: f64 = 2.8;

/// One picture from Commons.
#[derive(Clone)]
pub struct CommonsImage {
    pub url: String,
    pub title: String,
}

/// Percent-encode a query value.
fn encode(value: &str) -> String {
    let mut out = String::new();

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }

    out
}

/// Ask the API for the members of one category.
fn query_category(category: &str) -> Vec<CommonsImage> {
    let query = [
        ("action", "query"),
        ("generator", "categorymembers"),
        ("gcmtitle", &format!("Category:{category}")),
        ("gcmtype", "file"),
        ("gcmlimit", "150"),
        ("prop", "imageinfo"),
        ("iiprop", "url|size|mime"),
        ("iiurlwidth", "2560"),
        ("format", "json"),
    ];

    let url = format!(
        "https://commons.wikimedia.org/w/api.php?{}",
        query
            .iter()
            .map(|(key, value)| format!("{}={}", key, encode(value)))
            .collect::<Vec<_>>()
            .join("&")
    );

    let Ok(body) = http::get_text(&url) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<Value>(&body) else {
        return Vec::new();
    };

    let Some(pages) = json["query"]["pages"].as_object() else {
        return Vec::new();
    };

    let mut images = Vec::new();

    for page in pages.values() {
        let title = page["title"].as_str().unwrap_or_default();
        let title = title.strip_prefix("File:").unwrap_or(title);

        let Some(info) = page["imageinfo"].as_array().and_then(|list| list.first()) else {
            continue;
        };

        let mime = info["mime"].as_str().unwrap_or_default();
        if mime != "image/jpeg" && mime != "image/png" {
            continue;
        }

        let Some(url) = info["thumburl"].as_str().filter(|url| !url.is_empty()) else {
            continue;
        };

        let width = info["width"].as_u64().unwrap_or(0);
        let height = info["height"].as_u64().unwrap_or(0);
        if width < MIN_WIDTH || height < MIN_HEIGHT {
            continue;
        }

        let ratio = width as f64 / height as f64;
        if !(MIN_RATIO..=MAX_RATIO).contains(&ratio) {
            continue;
        }

        images.push(CommonsImage {
            url: url.to_string(),
            title: title.to_string(),
        });
    }

    images
}

/// Refresh the daily list and merge it into the accumulated one.
pub fn refresh_list(cache: &Path) {
    let list_path = cache.join("commons.list");
    let all_path = cache.join("commons.all");

    let fresh = today() == modified_on(&list_path);
    if fresh {
        return;
    }

    let mut lines: Vec<String> = Vec::new();

    for category in CATEGORIES {
        for image in query_category(category) {
            lines.push(format!("{}\t{}", image.url, image.title));
        }
    }

    if lines.is_empty() {
        let _ = std::fs::write(&list_path, "");
        return;
    }

    let _ = std::fs::write(&list_path, lines.join("\n"));

    // The accumulated list keeps every picture ever seen, so the pool does
    // not shrink when a category loses a member.
    let mut all: HashSet<String> = std::fs::read_to_string(&all_path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    all.extend(lines);
    let mut all: Vec<String> = all.into_iter().collect();
    all.sort();

    let _ = std::fs::write(&all_path, all.join("\n"));
}

/// Every picture the accumulated list holds that the rules allow.
pub fn allowed_images(cache: &Path) -> Vec<CommonsImage> {
    let Ok(text) = std::fs::read_to_string(cache.join("commons.all")) else {
        return Vec::new();
    };

    text.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let (url, title) = line.split_once('\t')?;
            if is_rejected(title) {
                return None;
            }
            Some(CommonsImage {
                url: url.to_string(),
                title: title.to_string(),
            })
        })
        .collect()
}

/// A file name for a picture: a readable part of the title and a hash of the
/// url, so the same picture keeps the same name.
fn file_name(image: &CommonsImage) -> String {
    let slug = slug_of(if image.title.is_empty() {
        &image.url
    } else {
        &image.title
    });

    let hash = md5_of(&image.url);
    let extension = if image.url.ends_with(".png") {
        "png"
    } else {
        "jpg"
    };

    format!("commons-{slug}-{hash}.{extension}")
}

/// A short, readable, file-system-safe version of a title.
fn slug_of(text: &str) -> String {
    let mut out = String::new();
    let mut separator = false;

    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character);
            separator = false;
        } else if !separator {
            out.push('_');
            separator = true;
        }
    }

    out.chars().take(28).collect()
}

/// The first eight characters of the md5 of a string.
fn md5_of(text: &str) -> String {
    use md5::{Digest, Md5};

    let mut hasher = Md5::new();
    hasher.update(text.as_bytes());
    let digest = format!("{:x}", hasher.finalize());

    digest[..8].to_string()
}

/// Download the pictures that are not on the wall yet.
pub fn download(cache: &Path, images: &[CommonsImage]) -> Vec<PathBuf> {
    let mut downloaded = Vec::new();
    let mut budget = DOWNLOAD_LIMIT;
    let mut expected: HashSet<PathBuf> = HashSet::new();

    for image in images {
        let path = cache.join(file_name(image));
        expected.insert(path.clone());

        if path.metadata().map(|meta| meta.len() > 0).unwrap_or(false) {
            downloaded.push(path);
            continue;
        }

        if budget == 0 {
            continue;
        }

        let partial = path.with_extension("part");
        if http::download(&image.url, &partial).is_err() {
            let _ = std::fs::remove_file(&partial);
            continue;
        }

        if magick::is_readable(&partial) {
            let _ = std::fs::rename(&partial, &path);
            budget = budget.saturating_sub(1);
            downloaded.push(path);
        } else {
            let _ = std::fs::remove_file(&partial);
        }
    }

    // Pictures that are no longer allowed are dropped, so a rule change
    // actually takes effect on the cache.
    if let Ok(entries) = std::fs::read_dir(cache) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();

            if !name.starts_with("commons-") || name.ends_with(".part") {
                continue;
            }
            if !expected.contains(&path) {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    downloaded
}

/// Today's date, as `YYYYMMDD`.
fn today() -> String {
    chrono::Local::now().format("%Y%m%d").to_string()
}

/// The date a file was last written, as `YYYYMMDD`.
fn modified_on(path: &Path) -> String {
    let Ok(meta) = std::fs::metadata(path) else {
        return String::new();
    };
    let Ok(modified) = meta.modified() else {
        return String::new();
    };
    let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH) else {
        return String::new();
    };
    let Some(stamp) = chrono::DateTime::from_timestamp(since.as_secs() as i64, 0) else {
        return String::new();
    };

    stamp
        .with_timezone(&chrono::Local)
        .format("%Y%m%d")
        .to_string()
}
