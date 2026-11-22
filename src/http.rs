//! The small HTTP layer everything else uses.

use std::time::Duration;

/// The user agent the requests go out with.
///
/// Wikimedia asks for one, and it is better to say what this is than to look
/// like a browser.
pub const USER_AGENT: &str = "OmarchyWallpaper/1.0 (local desktop wallpaper rotator)";

fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(40)))
        .user_agent(USER_AGENT)
        .build();

    ureq::Agent::new_with_config(config)
}

/// Fetch a URL as text.
pub fn get_text(url: &str) -> Result<String, String> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|error| format!("{url}: {error}"))?;

    response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("{url}: {error}"))
}

/// Fetch a URL as bytes.
pub fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut response = agent()
        .get(url)
        .call()
        .map_err(|error| format!("{url}: {error}"))?;

    response
        .body_mut()
        .read_to_vec()
        .map_err(|error| format!("{url}: {error}"))
}

/// Fetch a URL into a file.
pub fn download(url: &str, path: &std::path::Path) -> Result<(), String> {
    let body = get_bytes(url)?;
    std::fs::write(path, body).map_err(|error| format!("{}: {error}", path.display()))
}
