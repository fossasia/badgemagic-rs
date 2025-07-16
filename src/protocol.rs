//! Protocol used to update the badge

use std::num::TryFromIntError;

#[cfg(feature = "embedded-graphics")]
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::BinaryColor,
    prelude::Pixel,
    primitives::Rectangle,
    Drawable,
};
use time::OffsetDateTime;
use zerocopy::{BigEndian, FromBytes, Immutable, IntoBytes, KnownLayout, U16};

/// Message style configuration
/// ```
/// use badgemagic::protocol::{Mode, Style};
/// # (
/// Style::default().blink().border().mode(Mode::Center)
/// # );
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[must_use]
pub struct Style {
    #[cfg_attr(feature = "serde", serde(default))]
    blink: bool,

    #[cfg_attr(feature = "serde", serde(default))]
    border: bool,

    #[cfg_attr(feature = "serde", serde(default))]
    speed: Speed,

    #[cfg_attr(feature = "serde", serde(default))]
    mode: Mode,
}

impl Style {
    /// Enable blink mode
    ///
    /// The message will blink.
    /// ```
    /// use badgemagic::protocol::Style;
    /// # (
    /// Style::default().blink()
    /// # );
    /// ```
    pub fn blink(mut self) -> Self {
        self.blink = true;
        self
    }

    /// Show a dotted border around the display.
    /// ```
    /// use badgemagic::protocol::Style;
    /// # (
    /// Style::default().blink()
    /// # );
    /// ```
    pub fn border(mut self) -> Self {
        self.border = true;
        self
    }

    /// Set the update speed of the animations.
    ///
    /// The animation will jump to the next pixel at the specified frame rate.
    /// ```
    /// use badgemagic::protocol::{Speed, Style};
    /// # (
    /// Style::default().speed(Speed::Fps1_2)
    /// # );
    /// ```
    pub fn speed(mut self, speed: Speed) -> Self {
        self.speed = speed;
        self
    }

    /// Set the display animation.
    /// ```
    /// use badgemagic::protocol::{Mode, Style};
    /// # (
    /// Style::default().mode(Mode::Curtain)
    /// # );
    /// ```
    ///
    /// Show text centered, without an animation:
    /// ```
    /// use badgemagic::protocol::{Mode, Style};
    /// # (
    /// Style::default().mode(Mode::Center)
    /// # );
    /// ```
    pub fn mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }
}

/// Animation update speed
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "u8", into = "u8"))]
pub enum Speed {
    /// 1.2 FPS
    Fps1_2,

    /// 1.3 FPS
    Fps1_3,

    /// 2 FPS
    Fps2,

    /// 2.4 FPS
    Fps2_4,

    /// 2.8 FPS
    #[default]
    Fps2_8,

    /// 4.5 FPS
    Fps4_5,

    /// 7.5 FPS
    Fps7_5,

    /// 15 FPS
    Fps15,
}

impl From<Speed> for u8 {
    fn from(value: Speed) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for Speed {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Fps1_2,
            1 => Self::Fps1_3,
            2 => Self::Fps2,
            3 => Self::Fps2_4,
            4 => Self::Fps2_8,
            5 => Self::Fps4_5,
            6 => Self::Fps7_5,
            7 => Self::Fps15,
            _ => return Err(u8::try_from(-1).unwrap_err()),
        })
    }
}

/// Message display mode
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Mode {
    /// Scroll through the message from left to right
    #[default]
    Left,

    /// Scroll through the message from right to left
    Right,

    /// Enter from the bottom, move up
    Up,

    /// Enter from the top, move down
    Down,

    /// Center the text, no animation
    Center,

    /// Fast mode for animations
    ///
    /// Will leave a 4 pixel gap between screens:
    /// Place a 44x11 pixel screen every 48 pixels
    Fast,

    /// Drop rows of pixels from the top
    Drop,

    /// Open a curtain and reveal the message
    Curtain,

    /// A laser will reveal the message from left to right
    Laser,
}

/// Display Brightness
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Brightness {
    #[default]
    Full = 0x00,
    ThreeQuarters = 0x10,
    Half = 0x20,
    OneQuarter = 0x30,
}

impl From<Brightness> for u8 {
    fn from(value: Brightness) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for Brightness {
    type Error = TryFromIntError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Full,
            0x10 => Self::ThreeQuarters,
            0x20 => Self::Half,
            0x30 => Self::OneQuarter,
            _ => return Err(u8::try_from(-1).unwrap_err()),
        })
    }
}

const MSG_PADDING_ALIGN: usize = 64;

const MAGIC: [u8; 5] = *b"wang\0";

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct Header {
    magic: [u8; 5],
    brightness: u8,
    blink: u8,
    border: u8,
    speed_and_mode: [u8; 8],
    message_length: [U16<BigEndian>; 8],
    _padding_1: [u8; 6],
    timestamp: Timestamp,
    _padding_2: [u8; 20],
}

#[derive(FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
struct Timestamp {
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl Timestamp {
    fn new(ts: OffsetDateTime) -> Self {
        Self {
            #[allow(clippy::cast_possible_truncation)] // clippy does not understand `rem_euclid(100) <= 100`
            year: ts.year().rem_euclid(100) as u8,
            month: ts.month() as u8,
            day: ts.day(),
            hour: ts.hour(),
            minute: ts.minute(),
            second: ts.second(),
        }
    }

    fn now() -> Self {
        Self::new(OffsetDateTime::now_utc())
    }
}

/// Buffer to create a payload
///
/// A payload consists of up to 8 messages
/// ```
/// # #[cfg(feature = "embedded-graphics")]
/// # fn main() {
/// # use badgemagic::protocol::{PayloadBuffer, Style};
/// use badgemagic::embedded_graphics::{
///     geometry::{Point, Size},
///     pixelcolor::BinaryColor,
///     primitives::{PrimitiveStyle, Rectangle, Styled},
/// };
///
/// let mut buffer = PayloadBuffer::default();
/// buffer.add_message_drawable(
///     Style::default(),
///     &Styled::new(
///         Rectangle::new(Point::new(2, 2), Size::new(4, 7)),
///         PrimitiveStyle::with_fill(BinaryColor::On),
///     ),
/// );
/// # }
/// # #[cfg(not(feature = "embedded-graphics"))]
/// # fn main() {}
/// ```
pub struct PayloadBuffer {
    num_messages: u8,
    data: Vec<u8>,
}

impl Default for PayloadBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadBuffer {
    /// Create a new empty buffer
    #[must_use]
    pub fn new() -> Self {
        Self {
            num_messages: 0,
            data: Header {
                magic: MAGIC,
                brightness: 0,
                blink: 0,
                border: 0,
                speed_and_mode: [0; 8],
                message_length: [0.into(); 8],
                _padding_1: [0; 6],
                timestamp: Timestamp::now(),
                _padding_2: [0; 20],
            }
            .as_bytes()
            .into(),
        }
    }

    fn header_mut(&mut self) -> &mut Header {
        Header::mut_from_prefix(&mut self.data).unwrap().0
    }

    pub fn set_brightness(&mut self, brightness: Brightness) {
        self.header_mut().brightness = brightness.into();
    }

    /// Return the current number of messages
    pub fn num_messages(&mut self) -> usize {
        self.num_messages as usize
    }

    /// Add a messages containing the specified `content`
    ///
    /// ## Panics
    /// This method panics if it is unable to draw the content.
    #[cfg(feature = "embedded-graphics")]
    pub fn add_message_drawable<O>(
        &mut self,
        style: Style,
        content: &(impl Drawable<Color = BinaryColor, Output = O> + Dimensions),
    ) -> O {
        #[allow(clippy::cast_possible_wrap)]
        fn saturating_usize_to_isize(n: usize) -> isize {
            usize::min(n, isize::MAX as usize) as isize
        }

        fn add(a: i32, b: u32) -> usize {
            let result = a as isize + saturating_usize_to_isize(b as usize);
            result.try_into().unwrap_or_default()
        }

        let bounds = content.bounding_box();
        let width = add(bounds.top_left.x, bounds.size.width);
        let mut message = self.add_message(style, width.div_ceil(8));
        content.draw(&mut message).unwrap()
    }

    /// Add a message with `count * 8`  columns
    ///
    /// The returned `MessageBuffer` can be used as an `embedded_graphics::DrawTarget`
    /// with the `embedded_graphics` feature.
    ///
    /// ## Panics
    /// Panics if the supported number of messages is reached.
    pub fn add_message(&mut self, style: Style, count: usize) -> MessageBuffer {
        let index = self.num_messages as usize;
        assert!(
            index < 8,
            "maximum number of supported messages reached: {index} messages",
        );
        self.num_messages += 1;

        let header = self.header_mut();

        if style.blink {
            header.blink |= 1 << index;
        }
        if style.border {
            header.border |= 1 << index;
        }
        header.speed_and_mode[index] = ((style.speed as u8) << 4) | style.mode as u8;
        header.message_length[index] = count.try_into().unwrap();

        let start = self.data.len();
        self.data.resize(start + count * 11, 0);
        MessageBuffer(FromBytes::mut_from_bytes(&mut self.data[start..]).unwrap())
    }

    /// Get the current payload as bytes (without padding)
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Convert the payload buffer into bytes (with padding)
    #[allow(clippy::missing_panics_doc)] // should never panic
    #[must_use]
    pub fn into_padded_bytes(self) -> impl AsRef<[u8]> {
        let mut data = self.data;

        let prev_len = data.len();

        // pad msg to align to 64 bytes
        data.resize(
            (data.len() + (MSG_PADDING_ALIGN - 1)) & !(MSG_PADDING_ALIGN - 1),
            0,
        );

        // validate alignment
        assert_eq!(data.len() % 64, 0);
        assert!(prev_len <= data.len());

        data
    }
}

/// A display buffer for a single message.
///
/// Can be used as an `embedded_graphics::DrawTarget`.
pub struct MessageBuffer<'a>(&'a mut [[u8; 11]]);

impl MessageBuffer<'_> {
    /// Set the state of the pixel at point (`x`, `y`)
    ///
    /// Returns `None` if the pixel was out of bounds.
    pub fn set(&mut self, (x, y): (usize, usize), state: State) -> Option<()> {
        let byte = self.0.get_mut(x / 8)?.get_mut(y)?;
        let bit = 0x80 >> (x % 8);
        match state {
            State::Off => {
                *byte &= !bit;
            }
            State::On => {
                *byte |= bit;
            }
        }
        Some(())
    }

    #[cfg(feature = "embedded-graphics")]
    fn set_embedded_graphics(&mut self, point: Point, color: BinaryColor) -> Option<()> {
        let x = point.x.try_into().ok()?;
        let y = point.y.try_into().ok()?;
        self.set((x, y), color.into())
    }
}

/// State of a pixel
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum State {
    #[default]
    Off,
    On,
}

impl From<bool> for State {
    fn from(value: bool) -> Self {
        if value {
            Self::On
        } else {
            Self::Off
        }
    }
}

#[cfg(feature = "embedded-graphics")]
impl From<BinaryColor> for State {
    fn from(value: BinaryColor) -> Self {
        match value {
            BinaryColor::Off => Self::Off,
            BinaryColor::On => Self::On,
        }
    }
}

#[cfg(feature = "embedded-graphics")]
impl Dimensions for MessageBuffer<'_> {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        Rectangle::new(
            Point::zero(),
            Size::new((self.0.len() * 8).try_into().unwrap(), 11),
        )
    }
}

#[cfg(feature = "embedded-graphics")]
impl DrawTarget for MessageBuffer<'_> {
    type Color = BinaryColor;

    type Error = std::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            #[allow(clippy::manual_assert)]
            if self.set_embedded_graphics(point, color).is_none() {
                panic!(
                    "tried to draw pixel outside the display area (x: {}, y: {})",
                    point.x, point.y
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::ops::Range;

    use super::{Brightness, Speed};

    #[test]
    fn speed_to_u8_and_back() {
        const VALID_SPEED_VALUES: Range<u8> = 1..8;
        for i in u8::MIN..u8::MAX {
            if let Ok(speed) = Speed::try_from(i) {
                assert_eq!(u8::from(speed), i);
            } else {
                assert!(!VALID_SPEED_VALUES.contains(&i));
            }
        }
    }

    #[test]
    fn brightness_to_u8_and_back() {
        const VALID_BRIGHTNESS_VALUES: [(Brightness, u8); 4] = [
            (Brightness::Full, 0x00),
            (Brightness::ThreeQuarters, 0x10),
            (Brightness::Half, 0x20),
            (Brightness::OneQuarter, 0x30),
        ];

        for (value, raw) in VALID_BRIGHTNESS_VALUES {
            assert_eq!(u8::from(value), raw);
            assert_eq!(Brightness::try_from(raw).unwrap(), value);
        }
    }

}
