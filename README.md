# Minesweeper

A classic Minesweeper game that runs in your terminal, written in Rust.
Play with the keyboard, the mouse, or a gamepad.

## Features

- Five difficulty levels, from Beginner to Extreme
- Safe first click: the first cell you reveal always opens a starting area
- Keyboard, mouse and gamepad controls
- Customizable key bindings from the Options menu
- Simple sound effects for reveals, flags, wins and losses
- In-game timer
- A scrolling camera, so large boards fit in small terminals

## Difficulty levels

| Level        | Size    | Mines |
|--------------|---------|-------|
| Beginner     | 9 × 9   | 10    |
| Intermediate | 16 × 16 | 40    |
| Expert       | 30 × 16 | 99    |
| Tryhard      | 30 × 20 | 140   |
| Extreme      | 30 × 24 | 200   |

## Installation

You need a recent stable [Rust toolchain](https://rustup.rs/) (edition 2024, Rust 1.85 or newer).

```sh
git clone https://github.com/christianbraghette/minesweeper.git
cd minesweeper
cargo run --release
```

Or install it into your Cargo bin directory:

```sh
cargo install --git https://github.com/christianbraghette/minesweeper.git
```

### Linux dependencies

Audio (`rodio`) and gamepad support (`gilrs`) need the ALSA and udev development headers:

```sh
# Debian / Ubuntu
sudo apt install libasound2-dev libudev-dev pkg-config

# Fedora
sudo dnf install alsa-lib-devel systemd-devel pkgconf-pkg-config
```

## Controls

### Keyboard (default bindings)

| Key         | Action        |
|-------------|---------------|
| Arrow keys  | Move cursor   |
| `Enter`     | Reveal cell   |
| `Space`     | Flag cell     |
| `m`         | Back to menu  |
| `q`         | Quit          |

All of these can be rebound in **Options**.

### Mouse

| Button      | Action                 |
|-------------|------------------------|
| Left click  | Reveal cell / select   |
| Right click | Flag cell              |

Mouse support depends on your terminal emulator.

### Gamepad

Enable gamepad mode in **Options**.

| Button                     | Action       |
|----------------------------|--------------|
| D-pad                      | Move cursor  |
| South (A on Xbox, ✕ on PS) | Reveal cell  |
| East (B on Xbox, ○ on PS)  | Flag cell    |

After a game ends: **South** plays again, **Start** returns to the menu, **Select** quits.

## Roadmap

See [TODO.md](TODO.md). Planned next:

- Statistics
- Better sound quality

## Contributing

Issues and pull requests are welcome.

## License

Released under the [MIT License](LICENSE).
