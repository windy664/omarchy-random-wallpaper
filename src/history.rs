//! Picking a picture that has not been on the wall yet.
//!
//! A picture is identified by its content, not by its path: the same photo
//! can live in three places (an old numeric cache name, a newer slug name
//! and the local pool) and it must still count as one picture. A Bing image
//! additionally carries an id (`OHR.Name`) that is the same for every
//! encoding of that photo, so the same shot in two languages is one shot.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use rand::seq::IndexedRandom;

/// How many key lines the history keeps: two per picture, ten pictures.
const HISTORY_LINES: usize = 20;

/// The content hash of a file.
pub fn md5_of(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();

    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// The Bing image id in a file name, if there is one.
///
/// Bing calls its pictures `OHR.Something`, and the same picture keeps that
/// id across every resolution and every language.
pub fn oid_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let start = name.find("OHR.")?;
    let id: String = name[start + 4..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();

    if id.is_empty() {
        None
    } else {
        Some(format!("OHR.{id}"))
    }
}

/// Every key a picture is known by.
fn keys_of(path: &Path) -> Vec<String> {
    let mut keys = Vec::new();

    if let Ok(hash) = md5_of(path) {
        keys.push(format!("md5:{hash}"));
    }
    if let Some(oid) = oid_of(path) {
        keys.push(format!("oid:{oid}"));
    }

    keys
}

/// The keys recorded so far.
pub fn read_history(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn write_history(path: &Path, lines: &[String]) -> std::io::Result<()> {
    let mut text = String::new();
    for line in lines.iter().skip(lines.len().saturating_sub(HISTORY_LINES)) {
        text.push_str(line);
        text.push('\n');
    }

    std::fs::write(path, text)
}

/// Turn an old history that stored paths into one that stores keys.
fn migrate(path: &Path) {
    let lines = read_history(path);
    if !lines.iter().any(|line| line.starts_with('/')) {
        return;
    }

    let mut migrated = Vec::new();
    for line in lines {
        if line.starts_with('/') && Path::new(&line).is_file() {
            migrated.extend(keys_of(Path::new(&line)));
        } else {
            migrated.push(line);
        }
    }

    let _ = write_history(path, &migrated);
}

/// Pick one candidate that has not been shown yet.
///
/// Candidates that are the same picture are folded together first. When every
/// candidate has been on the wall, the history is cleared and the whole set
/// becomes fair game again.
pub fn pick(candidates: &[PathBuf], history_path: &Path) -> Option<PathBuf> {
    migrate(history_path);

    let history: HashSet<String> = read_history(history_path).into_iter().collect();

    let mut seen_hashes: HashSet<String> = HashSet::new();
    let mut seen_oids: HashSet<String> = HashSet::new();
    let mut unique: Vec<(String, Option<String>, PathBuf)> = Vec::new();

    for candidate in candidates {
        if !candidate.is_file() {
            continue;
        }

        let Ok(hash) = md5_of(candidate) else {
            continue;
        };
        let oid = oid_of(candidate);

        if seen_hashes.contains(&hash) {
            continue;
        }
        if let Some(oid) = &oid
            && seen_oids.contains(oid)
        {
            continue;
        }

        seen_hashes.insert(hash.clone());
        if let Some(oid) = &oid {
            seen_oids.insert(oid.clone());
        }

        unique.push((hash, oid, candidate.clone()));
    }

    let mut fresh: Vec<&(String, Option<String>, PathBuf)> = unique
        .iter()
        .filter(|(hash, oid, _)| {
            let mut keys = vec![format!("md5:{hash}")];
            if let Some(oid) = oid {
                keys.push(format!("oid:{oid}"));
            }
            !keys.iter().any(|key| history.contains(key))
        })
        .collect();

    if fresh.is_empty() {
        // Everything has been seen, so start over.
        let _ = write_history(history_path, &[]);
        fresh = unique.iter().collect();
    }

    let mut rng = rand::rng();
    let (hash, oid, path) = fresh.choose(&mut rng)?;

    let mut lines = read_history(history_path);
    lines.push(format!("md5:{hash}"));
    if let Some(oid) = oid {
        lines.push(format!("oid:{oid}"));
    }
    let _ = write_history(history_path, &lines);

    Some(path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bing_ids_are_read_out_of_a_file_name() {
        assert_eq!(
            oid_of(Path::new("/tmp/bing-thidOHR.AlpineBraies_ZH-CN123.jpg")),
            Some("OHR.AlpineBraies".to_string())
        );
        assert_eq!(oid_of(Path::new("/tmp/commons-some-title-1234.jpg")), None);
    }
}
