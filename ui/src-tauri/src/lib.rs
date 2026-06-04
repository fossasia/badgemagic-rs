use anyhow::Result;
use badgemagic::embedded_graphics::{
    geometry::Point,
    mono_font::{iso_8859_1::FONT_5X8, iso_8859_1::FONT_6X9, MonoFont, MonoTextStyle},
    pixelcolor::BinaryColor,
    text::Text,
};
use badgemagic::{
    protocol::{Mode, PayloadBuffer, Speed, Style},
    usb_hid::Device as UsbDevice,
};
use serde::Deserialize;
use std::num::TryFromIntError;
use std::path::PathBuf;
use badgemagic::embedded_graphics::{Drawable, Pixel};
use tauri_plugin_sql::{Migration, MigrationKind};
use u8g2_fonts::{U8g2TextStyle, fonts::{u8g2_font_lucasfont_alternate_tf, u8g2_font_spleen5x8_me, u8g2_font_6x10_tf}};

#[tauri::command]
fn set_text(text: &str, speed: u8, animation: &str, effects: Vec<&str>, font: u8, font_subtype: &str) -> String {

    let speed: Speed = Speed::try_from(speed).unwrap_or(Speed::Fps2_8);
    let mode: Mode = Mode::try_from(animation).unwrap_or(Mode::Left);

    let flash: bool = effects.contains(&"flashing");
    let border: bool = effects.contains(&"border");
    let invert: bool = effects.contains(&"inverted");

    let font_s: Font = font_from_font_and_subtype(font, font_subtype).unwrap();

    let mut payload = PayloadBuffer::new();

    payload = write_text_to_payload(payload, text, speed, mode, flash, border, invert, font_s);

    match write_payload(payload) {
        Ok(_) => "Success!".to_string(),
        Err(err) => {
            format!("Something went wrong: {}", err.backtrace())
        }
    }
}

#[tauri::command]
fn set_drawable(drawable: Vec<bool>, width: usize, speed: u8, animation: &str, effects: Vec<&str>) -> String {
    let mut payload = PayloadBuffer::new();

    let flashing: bool = effects.contains(&"flashing");
    let border: bool = effects.contains(&"border");

    let speed: Speed = Speed::try_from(speed).unwrap_or(Speed::Fps2_8);
    let mode: Mode = Mode::try_from(animation).unwrap_or(Mode::Left);

    let mut style = Style::default().speed(speed).mode(mode);

    if flashing {
        style = style.blink()
    }

    if border {
        style = style.border()
    }

    let mut buffer = payload.add_message(style, width.div_ceil(8));

    for (idx, val) in drawable.iter().enumerate() {
        if *val {
            Pixel(
                Point::new((idx % width) as i32, (idx / width) as i32),
                BinaryColor::On,
            )
                .draw(&mut buffer)
                .unwrap();
        }
    }

    match write_payload(payload) {
        Ok(_) => "Success!".to_string(),
        Err(err) => {
            format!("Something went wrong: {}", err.backtrace())
        }
    }
}


#[tauri::command]
fn set_messages(messages: Vec<Message>) -> String {
    let mut payload = PayloadBuffer::new();

    for message in messages {
        let speed: Speed = Speed::try_from(message.speed).unwrap_or(Speed::Fps2_8);
        let mode: Mode = Mode::try_from(message.animation.as_str()).unwrap_or(Mode::Left);

        let flash: bool = message.effects.contains(&"flashing".to_string());
        let border: bool = message.effects.contains(&"border".to_string());
        let invert: bool = message.effects.contains(&"inverted".to_string());

        let font: Font = font_from_font_and_subtype(message.font, &*message.font_subtype).unwrap();

        payload = write_text_to_payload(
            payload,
            message.text.as_str(),
            speed,
            mode,
            flash,
            border,
            invert,
            font,
        );
    }

    match write_payload(payload) {
        Ok(_) => "Success!".to_string(),
        Err(err) => {
            format!("Something went wrong: {}", err.backtrace())
        }
    }
}

#[tauri::command]
fn list_devices() -> Result<Vec<String>, String> {
    let devices = UsbDevice::list_all();
    match devices {
        Ok(r) => Ok(r),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        // Define your migrations here
        Migration {
            version: 1,
            description: "create_initial_tables",
            sql: "CREATE TABLE messages (id INTEGER PRIMARY KEY AUTOINCREMENT, content_id INTEGER, type TEXT);\
                  CREATE TABLE text_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT, speed INTEGER, animation TEXT, effects TEXT, font INTEGER, subtype TEXT)",
            kind: MigrationKind::Up,
        },
        // Migration {
        //     version: 1,
        //     description: "add_message_groups",
        //     sql: "CREATE TABLE message_groups (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT, speed INTEGER, animation TEXT, effects TEXT, font_size INTEGER)",
        //     kind: MigrationKind::Up,
        // }
    ];

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:messages.db", migrations)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_text,
            set_drawable,
            set_messages,
            list_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

fn write_text_to_payload(
    mut payload: PayloadBuffer,
    input_text: &str,
    speed: Speed,
    mode: Mode,
    flashing: bool,
    border: bool,
    inverted: bool,
    font: Font,
) -> PayloadBuffer {
    let mut style = Style::default().speed(speed).mode(mode);

    if flashing {
        style = style.blink()
    }

    if border {
        style = style.border()
    }

    let position = Point::from(&font);

    let bg_color = BinaryColor::from(inverted);

    let actualFont: ActualFont = ActualFont::from(&font);

    match actualFont {
        ActualFont::Mono(font_type) => {
            let font_s = MonoFont::from(font_type);
            let mut font = MonoTextStyle::new(&font_s, bg_color.invert());
            font.background_color = Some(bg_color);

            let text = Text::new(input_text, position, font);

            payload.add_message_drawable(style, &text);
        }
        ActualFont::U8G2(mut font) => {
            let text = Text::new(input_text, position, font);

            payload.add_message_drawable(style, &text);
        }
    }

    payload
}

fn write_payload(
    // transport: &TransportProtocol,
    payload: PayloadBuffer,
) -> Result<()> {
    UsbDevice::single()?.write(payload)
    // match transport {
    //     TransportProtocol::Usb => UsbDevice::single()?.write(payload),
    //     TransportProtocol::Ble => tokio::runtime::Builder::new_current_thread()
    //         .enable_all()
    //         .build()?
    //         .block_on(async { BleDevice::single().await?.write(payload).await }),
    // }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Deserialize)]
struct Message {
    id: i32,
    text: String,
    speed: u8,
    animation: String,
    effects: Vec<String>,
    font: u8,
    font_subtype: String,
    m_type: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
enum MonoFontType {
    Size5x8,
    #[default]
    Size6x9,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
enum U8G2FontType {
    #[default]
    u8g2_font_lucasfont_alternate_tf,
    u8g2_font_spleen5x8_me,
    u8g2_font_6x10_tf,

}

enum Font {
    Mono(MonoFontType),
    U8G2(U8G2FontType)
}

enum ActualFont<'a> {
    Mono(MonoFont<'a>),
    U8G2(U8g2TextStyle<BinaryColor>)
}
fn font_from_font_and_subtype(font: u8, subtype: &str) -> Result<Font, TryFromIntError> {
    Ok(match font {
        0 => Font::Mono(MonoFontType::try_from(subtype)?),
        1 => Font::U8G2(U8G2FontType::try_from(subtype)?),
        _ => return Err(u8::try_from(-1).unwrap_err()),
    })
}

impl TryFrom<&str> for MonoFontType {
    type Error = TryFromIntError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            "5x8" => Self::Size5x8,
            "6x9" => Self::Size6x9,
            _ => return Err(u8::try_from(-1).unwrap_err()),
        })
    }
}

impl TryFrom<&str> for U8G2FontType {
    type Error = TryFromIntError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            "lucasfont alternate tf" => Self::u8g2_font_lucasfont_alternate_tf,
            "spleen 5x8 me" => Self::u8g2_font_spleen5x8_me,
            "6x10 tf" => Self::u8g2_font_6x10_tf,
            _ => return Err(u8::try_from(-1).unwrap_err()),
        })
    }
}

impl<'a> From<&Font> for ActualFont<'_> {

    fn from(value: &Font) -> Self {
        match value {
            Font::U8G2(font_type) =>
                ActualFont::U8G2(match font_type {
                    U8G2FontType::u8g2_font_lucasfont_alternate_tf => U8g2TextStyle::new(u8g2_font_lucasfont_alternate_tf, BinaryColor::On),
                    U8G2FontType::u8g2_font_spleen5x8_me => U8g2TextStyle::new(u8g2_font_spleen5x8_me, BinaryColor::On),
                    U8G2FontType::u8g2_font_6x10_tf => U8g2TextStyle::new(u8g2_font_6x10_tf, BinaryColor::On),
                }),
            Font::Mono(font_type) =>
                ActualFont::Mono(match font_type {
                    MonoFontType::Size5x8 => FONT_5X8,
                    MonoFontType::Size6x9 => FONT_6X9,
                }),
        }
    }
}

impl<'a> From<&Font> for Point {
    fn from(value: &Font) -> Self {
        match value {
            Font::U8G2(fontType) =>
                match fontType {
                    U8G2FontType::u8g2_font_lucasfont_alternate_tf => Point::new(0, 8),
                    U8G2FontType::u8g2_font_spleen5x8_me => Point::new(0, 8),
                    U8G2FontType::u8g2_font_6x10_tf => Point::new(0, 7),
                },
            Font::Mono(fontType) =>
                match fontType {
                    MonoFontType::Size5x8 => Point::new(0, 8),
                    MonoFontType::Size6x9 => Point::new(0, 7)
                }
        }
    }
}
