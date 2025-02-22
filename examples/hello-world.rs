#![warn(clippy::all, clippy::pedantic)]

use anyhow::Result;
use badgemagic::{
    embedded_graphics::{
        geometry::Point, mono_font::MonoTextStyle, pixelcolor::BinaryColor, text::Text,
    },
    protocol::{Brightness, Mode, PayloadBuffer, Style},
    usb_hid::Device,
    util::DrawableLayoutExt,
};

fn main() -> Result<()> {
    let mut payload = PayloadBuffer::new(Brightness::default());

    payload.add_message_drawable(
        Style::default().mode(Mode::Center),
        &Text::new(
            "Hello",
            Point::new(0, 8),
            MonoTextStyle::new(
                &embedded_graphics::mono_font::iso_8859_1::FONT_6X9,
                BinaryColor::On,
            ),
        ),
    );

    payload.add_message_drawable(
        Style::default().mode(Mode::Center),
        &Text::new(
            "Hello",
            Point::new(0, 5),
            MonoTextStyle::new(
                &embedded_graphics::mono_font::iso_8859_1::FONT_4X6,
                BinaryColor::On,
            ),
        )
        .z_stack(Text::new(
            "World",
            Point::new(23, 8),
            MonoTextStyle::new(
                &embedded_graphics::mono_font::iso_8859_1::FONT_4X6,
                BinaryColor::On,
            ),
        )),
    );

    Device::single()?.write(payload)?;

    Ok(())
}
