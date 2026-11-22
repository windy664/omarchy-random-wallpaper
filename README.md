<div align="center">

# omarchy-random-wallpaper

![Rust](https://img.shields.io/badge/Rust-2024-d99a6c?style=flat-square&logo=rust&logoColor=white)
![Platform](https://img.shields.io/badge/Omarchy%20%2B%20Hyprland-1a79a5?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-3da639?style=flat-square)

</div>

A wallpaper rotator for [Omarchy](https://omarchy.org) that only puts
**landscapes and city skylines** on the screen, and does not repeat itself.

---

## What it does

Candidates come from three places:

- **Bing's daily picture archive.** The endpoint ignores the market and
  returns the same set every time, so the tool walks the `idx` window
  (indices 0 to 7) to get about fifteen different pictures.
- **Wikimedia Commons' featured pictures** of nature, landscapes, skylines and
  cityscapes.
- **A local pool** of pictures you picked by hand, which are trusted.

Every candidate goes through the rules, and only what passes is downloaded
and shown.

## The rules

A picture has to look like a landscape or a city, judged by the words in its
name. Bing's pictures are judged by their slug (`OHR.AlpineBraies_EN-US…`),
Commons' by their title. Both lists are deliberately conservative: a picture
that is not clearly one of the two is not applied. Showing nothing is better
than showing a picture of a coffee cup.

During the day only pictures that are bright enough are used, so text stays
readable on them.

## Never the same picture twice

A picture is identified by its **content**, not by its path. The same photo
can live in three places — an old numeric cache name, a newer slug name and
the local pool — and it still counts as one picture.

Bing gives each picture an id (`OHR.Name`) that is the same for every
resolution and every language, so the same shot in two encodings is one shot
as well.

When every candidate has been on the wall, the history is cleared and the
whole set comes back around.

## Install

```sh
cargo install --git https://github.com/windy664/omarchy-random-wallpaper
```

The CI builds the binary and attaches it to every run, so a machine without a
Rust toolchain can install it from the artifact instead:

```sh
gh run download --repo windy664/omarchy-random-wallpaper \
  --name omarchy-random-wallpaper-x86_64-unknown-linux-gnu \
  -D .
install -Dm755 random-wallpaper ~/.local/bin/random-wallpaper
```

## Use

```sh
random-wallpaper
```

It sets the wallpaper with `omarchy theme bg set` and sends a notification
with the name of the picture it chose.

Run it from a timer:

```ini
# ~/.config/systemd/user/random-wallpaper.timer
[Unit]
Description=Rotate the wallpaper

[Timer]
OnBootSec=5min
OnUnitActiveSec=2h
AccuracySec=1min

[Install]
WantedBy=timers.target
```

## Where things live

| Path | What |
|---|---|
| `~/.cache/random-wallpapers/` | the downloaded pictures |
| `~/.cache/random-wallpapers/history` | the pictures that were shown |
| `~/.cache/random-wallpapers/brightness.cache` | the measured brightness of each picture |
| `~/.cache/random-wallpapers/commons.list` | today's Commons list |
| `~/.cache/random-wallpapers/commons.all` | every Commons picture ever seen |
| `~/.config/omarchy/backgrounds/catppuccin/` | the local pool, trusted as it is |

## Notes

- It shells out to `magick` to check that a download is a picture and to
  measure its brightness, and to `omarchy theme bg set` to apply it.
- The cache keeps the newest 60 Bing pictures and 40 Commons pictures.

## License

MIT — see [LICENSE](LICENSE).
