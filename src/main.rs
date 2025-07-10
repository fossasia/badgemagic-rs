#![warn(clippy::all, clippy::pedantic)]

use anyhow::{Context, Result};
use badgemagic::{
    ble::Device as BleDevice,
    protocol::{Brightness, Mode, PayloadBuffer, Speed, Style},
    usb_hid::Device as UsbDevice,
};
use base64::Engine;
use clap::{Parser, ValueEnum};
#[cfg(not(any(feature = "u8g2-fonts")))]
use embedded_graphics::mono_font::{iso_8859_1::FONT_6X9, MonoTextStyle};
use embedded_graphics::{
    geometry::Point,
    image::{Image, ImageRawLE},
    pixelcolor::BinaryColor,
    text::Text,
    Drawable, Pixel,
};
use image::{
    codecs::gif::GifDecoder, imageops::FilterType, AnimationDecoder, ImageReader, Pixel as iPixel,
};
use serde::Deserialize;
use std::{fs, fs::File, io::BufReader, path::PathBuf};
#[cfg(feature = "u8g2-fonts")]
use u8g2_fonts::{fonts::u8g2_font_lucasfont_alternate_tf, U8g2TextStyle};

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

    /// Transport protocol to use
    #[clap(long)]
    transport: TransportProtocol,

    /// Brightness of the panel
    #[clap(long)]
    brightness: Option<Brightness>,

    /// List all devices visible to a transport and exit
    #[clap(long)]
    list_devices: bool,

    /// Path to TOML configuration file
    #[clap(required_unless_present = "list_devices")]
    config: Option<PathBuf>,
}

#[derive(Clone, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum TransportProtocol {
    Usb,
    Ble,
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
    ImageFile { img_file: PathBuf },
    GifFile { gif_file: PathBuf },
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    if args.list_devices {
        return list_devices(&args.transport);
    }

    let payload = generate_payload(&mut args)?;

    write_payload(&args.transport, payload)
}

fn list_devices(transport: &TransportProtocol) -> Result<()> {
    let devices = match transport {
        TransportProtocol::Usb => UsbDevice::list_all(),
        TransportProtocol::Ble => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async { BleDevice::list_all().await }),
    }?;

    eprintln!(
        "found {} {} devices",
        devices.len(),
        transport.to_possible_value().unwrap().get_name(),
    );
    for device in devices {
        println!("- {device}");
    }

    Ok(())
}

fn generate_payload(args: &mut Args) -> Result<PayloadBuffer> {
    let config_path = args.config.take().unwrap_or_default();
    let config = fs::read_to_string(&config_path)
        .with_context(|| format!("load config: {config_path:?}"))?;
    let config: Config = {
        let extension = args
            .format
            .as_deref()
            .map(AsRef::as_ref)
            .or(config_path.extension())
            .context("missing file extension for config file")?;
        match extension.to_str().unwrap_or_default() {
            "json" => serde_json::from_str(&config).context("parse config")?,
            "toml" => toml::from_str(&config).context("parse config")?,
            _ => anyhow::bail!("unsupported config file extension: {extension:?}"),
        }
    };

    let mut payload = PayloadBuffer::new(args.brightness.unwrap_or_default());

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
                #[cfg(not(any(feature = "u8g2-fonts")))]
                let text = Text::new(
                    &text,
                    Point::new(0, 7),
                    MonoTextStyle::new(&FONT_6X9, BinaryColor::On),
                );

                #[cfg(feature = "u8g2-fonts")]
                let text = Text::new(
                    &text,
                    Point::new(0, 8),
                    U8g2TextStyle::new(u8g2_font_lucasfont_alternate_tf, BinaryColor::On),
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
                                Pixel(Point::new(x.try_into()?, y.try_into()?), BinaryColor::On)
                                    .draw(&mut buffer)?;
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
            Content::ImageFile { img_file } => {
                let img_reader = ImageReader::open(img_file)?;
                let img = img_reader
                    .decode()?
                    .resize(u32::MAX, 11, FilterType::Nearest)
                    .into_luma8();
                let (width, height) = img.dimensions();
                let mut buffer = payload.add_message(style, (width as usize + 7) / 8);
                for y in 0..height {
                    for x in 0..width {
                        if img.get_pixel(x, y).0 > [31] {
                            Pixel(Point::new(x.try_into()?, y.try_into()?), BinaryColor::On)
                                .draw(&mut buffer)?;
                        }
                    }
                }
            }
            Content::GifFile { gif_file } => {
                let file_in = BufReader::new(File::open(gif_file)?);
                let frames = GifDecoder::new(file_in)?
                    .into_frames()
                    .collect_frames()
                    .expect("error decoding gif");

                let frame_count = frames.len();
                let (width, height) = frames.first().unwrap().buffer().dimensions();
                if height != 11 || width != 44 {
                    anyhow::bail!("Expected 44x11 pixel gif file");
                }

                let mut buffer = payload.add_message(style, (48 * frame_count + 7) / 8);

                for (i, frame) in frames.iter().enumerate() {
                    let buf = frame.buffer();
                    for y in 0..11 {
                        for x in 0..44 {
                            if buf.get_pixel(x, y).to_luma().0 > [31] {
                                Pixel(
                                    Point::new((x as usize + i * 48).try_into()?, y.try_into()?),
                                    BinaryColor::On,
                                )
                                .draw(&mut buffer)?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(payload)
}

fn write_payload(
    transport: &TransportProtocol,
    payload: PayloadBuffer,
) -> Result<(), anyhow::Error> {
    match transport {
        TransportProtocol::Usb => UsbDevice::single()?.write(payload),
        TransportProtocol::Ble => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async { BleDevice::single().await?.write(payload).await }),
    }
}
