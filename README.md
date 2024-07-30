[![CI](../../actions/workflows/ci.yaml/badge.svg)](../../actions/workflows/ci.yaml)

# Badge Magic in Rust

Library and CLI to configure LED badges.

## Installation

As of now there are no proper releases (with version numbers) of this tool.

The latest commit on the main branch just gets build and released automatically.

Download the prebuild program for one of the following operating systems:

- [Linux (GNU / 64 bit)](../../releases/latest/download/badgemagic.x86_64-unknown-linux-gnu)
- [Windows (64 bit)](../../releases/latest/download/badgemagic.x86_64-pc-windows-msvc.exe)
- [MacOS (Intel)](../../releases/latest/download/badgemagic.x86_64-apple-darwin)
- [MacOS (M1, etc.)](../../releases/latest/download/badgemagic.aarch64-apple-darwin)

```sh
# After the download rename the file to `badgemagic`
mv badgemagic.<target> badgemagic

# Make the program executable (linux / macOS only)
chmod +x badgemagic

# Test that it works
./badgemagic --help
```

> Note: The windows and macOS build is not actively tested. Please try it out and report back whether it worked or any problems that might occour.

If your system is not listed above (Linux / Windows on ARM, musl, etc.) or you want to do it anyway, you can install this tool from source:

```sh
cargo install --git https://github.com/fossasia/badgemagic-rs --features cli
badgemagic --help
```

Or clone the repo and run the CLI:
```sh
git clone https://github.com/fossasia/badgemagic-rs
cd badgemagic-rs
cargo run --features cli -- --help
```

## Usage

Execute the `badgemagic` tool and pass the file name of your configuration file alongside the mode of transport (USB or Bluetooth Low Energy).
Depending on how you installed the tool:

```sh
# Downloaded from release page
./badgemagic config.toml

# Installed with cargo install
badgemagic config.toml

# Run from git repository
cargo run --features cli -- config.toml
```

The above command will read your configuration from a file named `config.toml` in the current directory.
The transport mode can be either `--transport usb` or `--transport ble` for transferring the message via Bluetooth Low Energy.
Usage of BLE on macOS requires special permissions, which is explained in more detail [here](https://github.com/deviceplug/btleplug#macos).

## Configuration

You can have a look at the example configurations in the [`demo` directory](demo).

The TOML configuration consists of up to 8 message sections starting with `[[message]]`.

Each message can have the following options:
```toml
[[message]]
# Enable blink mode
blink = true

# Show a dotted border arround the display
border = true

# Set the update speed of the animations (0 to 7)
speed = 6

# Set the display animation (left, right, up, down, center, fast, drop, curtain, laser)
mode = "left"

# The text to show on the display
text = "Lorem ipsum dolor sit amet."
```

You can omit options you don't need:
```toml
[[message]]
mode = "center"
text = "Hello"
```

If you want you can "draw" images as ASCII art (`_` = Off, `X` = On):
```toml
[[message]]
mode = "center"
bitstring = """
___XXXXX___
__X_____X__
_X_______X_
X__XX_XX__X
X__XX_XX__X
X_________X
X_XX___XX_X
X__XXXXX__X
_X__XXX__X_
__X_____X__
___XXXXX___
"""
```

You just replace the `text` option with `bitstring`. All other options (e.g. `border`, `blink`) still work and can be combined with a custom image.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
