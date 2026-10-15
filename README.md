# bounce_tales

2008 Nokia game replicated in Rust using the [macroquad](https://macroquad.rs) game engine.

## 1. Install Rust (rustup)

### Linux / macOS

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Windows

Download and run `rustup-init.exe` from <https://rustup.rs>.

### Verify

```sh
rustc --version
cargo --version
```

This project uses Rust edition 2024, so Rust 1.85 or newer is required. To update:

```sh
rustup update stable
```

### Linux system dependencies

macroquad needs a few system libraries on Linux:

```sh
# Debian / Ubuntu
sudo apt install pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev

# Fedora
sudo dnf install libX11-devel libXi-devel mesa-libGL-devel alsa-lib-devel
```

## 2. Get the source

```sh
git clone https://github.com/jyotsanh/bounce_tales.git
cd bounce_tales
```

## 3. Build

```sh
# debug build
cargo build

# optimized release build
cargo build --release
```

## 4. Run

```sh
cargo run --release
```

The binary is also available after building at `target/release/bounce_tales`.

## License

See [LICENSE](LICENSE).
