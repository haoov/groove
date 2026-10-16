# Groove

A desktop workspace for a platform engineer who works through agents.

Groove is a Rust application drawn on the GPU. Its docs are in [`docs/`](docs/README.md).

## Install

Download from the [latest release](https://github.com/haoov/groove/releases).

### Linux

Install the `.deb`:

```sh
sudo apt install ./groove_*.deb
```

It needs glibc 2.39 or later and `git`, and recommends `gh` and `glab`.

### macOS

Open the `.dmg` and drag Groove to Applications. It runs on macOS 26 or later, on Apple
Silicon and Intel, and needs `git`.

The app is signed ad hoc, not by an Apple Developer ID, so Gatekeeper blocks the first
launch. Click Done, then open System Settings → Privacy & Security and choose Open Anyway.
Or clear the quarantine flag:

```sh
xattr -dr com.apple.quarantine /Applications/Groove.app
```

Do it again after each update.
