#![warn(clippy::all, clippy::pedantic)]

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use badgemagic::{
    protocol::{Mode, PayloadBuffer, Speed, Style},
    usb_hid::Device,
};
use base64::Engine;
use clap::Parser;
use embedded_graphics::{
    geometry::Point,
    image::{Image, ImageRawLE},
    mono_font::{iso_8859_1::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    text::Text,
    Drawable, Pixel,
};
use serde::Deserialize;

#[derive(Parser)]
/// Upload a configuration with up to 8 messages to an LED badge
#[clap(
    version = badgemagic::cli::VERSION,
    author,
    help_template = "\
{before-help}{name} {version}
{author-with-newline}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
    ",
)]
struct Args {
    /// File format of the config file (toml, json)
    #[clap(long)]
    format: Option<String>,

    /// Path to TOML configuration file
    config: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(rename = "message")]
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    blink: bool,

    #[serde(default)]
    border: bool,

    #[serde(default)]
    speed: Speed,

    #[serde(default)]
    mode: Mode,

    #[serde(flatten)]
    content: Content,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, untagged)]
enum Content {
    Text { text: String },
    Bitstring { bitstring: String },
    BitmapBase64 { width: u32, bitmap_base64: String },
    BitmapFile { width: u32, bitmap_file: PathBuf },
    // TODO: implement png
    // PngFile { png_file: PathBuf },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = fs::read_to_string(&args.config)
        .with_context(|| format!("load config: {:?}", args.config))?;

    let config: Config = {
        let extension = args
            .format
            .as_deref()
            .map(AsRef::as_ref)
            .or(args.config.extension())
            .context("missing file extension for config file")?;
        match extension.to_str().unwrap_or_default() {
            "json" => serde_json::from_str(&config).context("parse config")?,
            "toml" => toml::from_str(&config).context("parse config")?,
            _ => anyhow::bail!("unsupported config file extension: {extension:?}"),
        }
    };

    let mut payload = PayloadBuffer::new();

    for message in config.messages {
        let mut style = Style::default();
        if message.blink {
            style = style.blink();
        }
        if message.border {
            style = style.border();
        }
        style = style.speed(message.speed).mode(message.mode);
        match message.content {
            Content::Text { text } => {
                let text = Text::new(
                    &text,
                    Point::new(0, 7),
                    MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
                );
                payload.add_message_drawable(style, &text);
            }
            Content::Bitstring { bitstring } => {
                let lines: Vec<_> = bitstring.trim().lines().collect();

                anyhow::ensure!(
                    lines.len() == 11,
                    "expected 11 lines in bitstring, found {} lines",
                    lines.len()
                );
                let width = lines[0].len();
                if lines.iter().any(|l| l.len() != width) {
                    anyhow::bail!(
                        "lines should have the same length, got: {:?}",
                        lines.iter().map(|l| l.len()).collect::<Vec<_>>()
                    );
                }
                let mut buffer = payload.add_message(style, (width + 7) / 8);

                for (y, line) in lines.iter().enumerate() {
                    for (x, c) in line.chars().enumerate() {
                        match c {
                            '_' => {
                                // off
                            }
                            'X' => {
                                Pixel(
                                    Point::new(x.try_into().unwrap(), y.try_into().unwrap()),
                                    BinaryColor::On,
                                )
                                .draw(&mut buffer)
                                .unwrap();
                            }
                            _ => anyhow::bail!("invalid bit value for bit ({x}, {y}): {c:?}"),
                        }
                    }
                }
            }
            Content::BitmapBase64 {
                width,
                bitmap_base64: bitmap,
            } => {
                let data = if bitmap.ends_with('=') {
                    base64::engine::general_purpose::STANDARD
                } else {
                    base64::engine::general_purpose::STANDARD_NO_PAD
                }
                .decode(bitmap)
                .context("decode bitmap")?;
                let image_raw = ImageRawLE::<BinaryColor>::new(&data, width);
                let image = Image::new(&image_raw, Point::zero());
                payload.add_message_drawable(style, &image);
            }
            Content::BitmapFile { width, bitmap_file } => {
                let data = fs::read(bitmap_file).context("load bitmap")?;
                let image_raw = ImageRawLE::<BinaryColor>::new(&data, width);
                let image = Image::new(&image_raw, Point::zero());
                payload.add_message_drawable(style, &image);
            }
        }
    }

    Device::single()?.write(payload)?;

    Ok(())
}
