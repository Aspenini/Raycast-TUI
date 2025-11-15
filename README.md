# Raycast TUI

A cross-platform terminal-based raycaster written in Rust. Experience a classic 3D-style rendering in your terminal!

## Features

- **Cross-platform**: Works on Windows, Linux, and macOS
- **Terminal-based**: No OS-specific code, pure terminal rendering
- **Real-time raycasting**: Smooth 3D-style rendering using raycasting algorithm
- **Interactive controls**: Move and rotate your view in real-time

## Controls

- **W / ↑**: Move forward
- **S / ↓**: Move backward
- **A**: Strafe left
- **D**: Strafe right
- **←**: Rotate left
- **→**: Rotate right
- **Q / Esc**: Quit

## Building

Make sure you have Rust installed. Then:

```bash
cargo build --release
```

## Running

```bash
cargo run --release
```

Or run the release binary directly:

```bash
./target/release/raycast-tui
```

## How It Works

The raycaster uses a DDA (Digital Differential Analyzer) algorithm to cast rays from the player's viewpoint. Each ray determines the distance to the nearest wall, which is then used to calculate the height of the wall column on screen. Different colors represent different distances, creating a depth effect.

The map is represented as a 2D grid where `1` represents walls and `0` represents empty space. The player can move and rotate within this space, and the raycaster renders the 3D perspective in real-time.

## Requirements

- Rust 1.70+ (edition 2021)
- A terminal that supports ANSI colors

## License

This project is open source and available for use.

