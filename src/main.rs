//! A wallpaper rotator that only puts landscapes and city skylines on the
//! screen, and does not repeat itself.
//!
//! Candidates come from three places:
//!
//!   - Bing's daily picture archive, filtered by the words in its slug
//!   - Wikimedia Commons' featured pictures, filtered by the title
//!   - a local pool of pictures that were picked by hand and are trusted
//!
//! A picture is identified by its content, so the same photo in three places
//! counts once. Daytime only shows pictures that are bright enough to read
//! text on.

mod bing;
mod commons;
mod history;
mod http;
mod magick;
mod scenic;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::{Local, Timelike};

/// The hour the day starts, as far as the wallpaper is concerned.
const DAY_START: u32 = 7;

/// The hour the day ends.
const DAY_END: u32 = 19;

/// How bright a picture has to be to be shown during the day.
const DAYTIME_BRIGHTNESS: f64 = 0.45;

/// How many Bing pictures to keep in the cache.
const BING_CACHE_LIMIT: usize = 60;

/// How many Commons pictures to keep in the cache.
const COMMONS_CACHE_LIMIT: usize = 40;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// The mean brightness of a picture, cached between runs.
fn brightness_of(path: &Path, cache: &Path) -> f64 {
    let key = path.to_string_lossy().into_owned();

    if let Ok(text) = std::fs::read_to_string(cache) {
        for line in text.lines().rev() {
            if let Some((cached_path, value)) = line.rsplit_once('|')
                && cached_path == key
                && let Ok(value) = value.parse::<f64>()
            {
                return value;
            }
        }
    }

    let brightness = magick::mean_brightness(path).unwrap_or(0.5);

    if let Ok(mut text) = std::fs::read_to_string(cache) {
        text.push_str(&format!("{key}|{brightness}\n"));
        let _ = std::fs::write(cache, text);
    }

    brightness
}

/// Keep only the bright pictures while the sun is up.
fn filter_by_brightness(candidates: Vec<PathBuf>, cache: &Path) -> Vec<PathBuf> {
    let hour = Local::now().hour();
    if !(DAY_START..DAY_END).contains(&hour) {
        return candidates;
    }

    let mut all = Vec::new();
    let mut bright = Vec::new();

    for candidate in candidates {
        let brightness = brightness_of(&candidate, cache);
        all.push(candidate.clone());

        if brightness > DAYTIME_BRIGHTNESS {
            bright.push(candidate);
        }
    }

    // A dull day is better than no wallpaper.
    if bright.is_empty() { all } else { bright }
}

/// Keep the newest `limit` files that start with `prefix`.
fn prune(cache: &Path, prefix: &str, limit: usize) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };

    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            if !name.starts_with(prefix) {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();

    files.sort_by_key(|(modified, _)| *modified);
    files.reverse();

    for (_, path) in files.into_iter().skip(limit) {
        let _ = std::fs::remove_file(path);
    }
}

/// Remove the old numeric Bing names that hold a picture the slug names
/// already hold.
fn prune_legacy_duplicates(cache: &Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };

    let mut hashes: HashSet<String> = HashSet::new();
    let mut legacy: Vec<PathBuf> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        if name.starts_with("bing-th") && name.ends_with(".jpg") {
            if let Ok(hash) = history::md5_of(&path) {
                hashes.insert(hash);
            }
        } else if name.starts_with("bing-")
            && name.ends_with(".jpg")
            && name["bing-".len()..].starts_with(|c: char| c.is_ascii_digit())
        {
            legacy.push(path);
        }
    }

    for path in legacy {
        if let Ok(hash) = history::md5_of(&path)
            && hashes.contains(&hash)
        {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Create a file if it is not there yet, without touching what is in it.
fn touch(path: &Path) {
    use std::fs::OpenOptions;

    let _ = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path);
}

/// The pictures in the local pool, which were picked by hand and are trusted.
fn local_pool(pool: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(pool) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.to_string_lossy().to_lowercase();
            name.ends_with(".jpg") || name.ends_with(".png")
        })
        .collect()
}

fn main() {
    let cache = home().join(".cache/random-wallpapers");
    let pool = home().join(".config/omarchy/backgrounds/catppuccin");
    let history_path = cache.join("history");
    let brightness_cache = cache.join("brightness.cache");

    if std::fs::create_dir_all(&cache).is_err() {
        eprintln!("random-wallpaper: cannot create {}", cache.display());
        std::process::exit(1);
    }

    // Both files are state, not scratch: create them if they are missing but
    // never empty them, or nothing would ever be remembered.
    touch(&history_path);
    touch(&brightness_cache);

    let mut candidates: Vec<PathBuf> = Vec::new();

    for image in bing::fetch() {
        if let Some(path) = bing::download(&image, &cache) {
            candidates.push(path);
        }
    }

    candidates.extend(local_pool(&pool));

    commons::refresh_list(&cache);
    let commons_images = commons::allowed_images(&cache);
    candidates.extend(commons::download(&cache, &commons_images));

    // The same picture can arrive from more than one source.
    let mut seen: HashSet<PathBuf> = HashSet::new();
    candidates.retain(|path| seen.insert(path.clone()));

    candidates = filter_by_brightness(candidates, &brightness_cache);

    let Some(image) = history::pick(&candidates, &history_path) else {
        return;
    };

    prune(&cache, "bing-", BING_CACHE_LIMIT);
    prune(&cache, "commons-", COMMONS_CACHE_LIMIT);
    prune_legacy_duplicates(&cache);

    let applied = Command::new("timeout")
        .args(["30", "omarchy", "theme", "bg", "set"])
        .arg(&image)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !applied {
        eprintln!(
            "random-wallpaper: could not set {}",
            image.to_string_lossy()
        );
        return;
    }

    let name = image
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let _ = Command::new("notify-send")
        .args(["-u", "low", "壁纸轮换", &name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
