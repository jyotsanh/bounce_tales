## Downloads

| System | File |
| --- | --- |
| Linux (Debian, Ubuntu 22.04+ and friends) | `bounce_tales-<version>-linux-amd64.deb`: `sudo apt install ./bounce_tales-*.deb`, then start **Bounce Tales** from the app menu |
| Windows 10/11 (64-bit) | `…-windows-x64-setup.exe` (installer, no admin needed) or `…-windows-x64.zip` (portable: unzip and run `bounce_tales.exe`) |
| macOS 11+ (Apple Silicon and Intel) | `…-macos-universal.dmg`: open it and drag **Bounce Tales** to Applications |

The Windows and macOS builds are not code-signed yet:
- **Windows**: if SmartScreen says "Windows protected your PC", choose **More info → Run anyway**.
- **macOS**: the first time, right-click the app and choose **Open** (or run `xattr -dr com.apple.quarantine "/Applications/Bounce Tales.app"`).

`SHA256SUMS.txt` lists the checksum of every file (`sha256sum -c SHA256SUMS.txt`).
