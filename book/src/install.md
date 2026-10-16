# Install

Download the package for your platform from the
[latest release](https://github.com/haoov/groove/releases). Each release also holds a
`SHA256SUMS` file to check the download.

## Linux

Groove ships as a `.deb` for x86-64. Install it with `apt`:

```sh
sudo apt install ./groove_*.deb
```

It needs glibc 2.39 or later and `git`. It recommends `gh` and `glab`.

## macOS

Groove ships as one `.dmg` for Apple Silicon and Intel. It runs on macOS 26 or later and
needs `git`.

1. Open the `.dmg`.
2. Drag Groove to Applications.

### Let Gatekeeper open it

The app is signed ad hoc, not with an Apple Developer ID. Gatekeeper blocks the first
launch. Do one of these:

- Click Done. Open System Settings → Privacy & Security, then choose Open Anyway.
- Clear the quarantine flag from a terminal:

  ```sh
  xattr -dr com.apple.quarantine /Applications/Groove.app
  ```

Do it again after each update.

## Next

Do the [first run](first-run.md).
