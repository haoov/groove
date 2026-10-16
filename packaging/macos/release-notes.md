
### Installing on macOS

Open the `.dmg` and drag Groove to Applications. The app is signed ad hoc, not by an Apple
Developer ID, so Gatekeeper blocks the first launch: click Done, then open System Settings →
Privacy & Security and choose Open Anyway. Or clear the quarantine flag from a terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Groove.app
```

Do it again after each update.
