//! ImageMagick, used for the two things it is good at here: checking that a
//! download is really a picture, and measuring how bright it is.

use std::path::Path;
use std::process::Command;

/// Whether the file is an image ImageMagick can read.
pub fn is_readable(path: &Path) -> bool {
    Command::new("magick")
        .arg("identify")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// The mean brightness of a picture, between 0.0 and 1.0.
///
/// The picture is shrunk first: the mean is the same at any size and reading
/// a full resolution file is far slower.
pub fn mean_brightness(path: &Path) -> Option<f64> {
    let output = Command::new("magick")
        .arg(path)
        .args(["-resize", "64x64"])
        .args(["-colorspace", "Gray"])
        .args(["-format", "%[fx:mean]"])
        .arg("info:")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.trim().parse::<f64>().ok()
}
