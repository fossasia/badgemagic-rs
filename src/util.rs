//! Graphics utilities

use embedded_graphics::Drawable;

use self::layout::ZStack;

/// Drawable layout extension
pub trait DrawableLayoutExt: Drawable + Sized {
    /// Draw a
    fn z_stack<T>(self, other: T) -> ZStack<Self, T> {
        ZStack(self, other)
    }
}

impl<T> DrawableLayoutExt for T where T: Drawable {}

pub mod layout {
    //! Types used by `DrawableLayoutExt `

    use embedded_graphics::{
        geometry::{Dimensions, Point},
        primitives::Rectangle,
        Drawable,
    };

    pub struct ZStack<A, B>(pub(super) A, pub(super) B);

    impl<A, B> Dimensions for ZStack<A, B>
    where
        A: Dimensions,
        B: Dimensions,
    {
        fn bounding_box(&self) -> Rectangle {
            let a = self.0.bounding_box();
            let b = self.1.bounding_box();
            let left = i32::min(a.top_left.x, b.top_left.x);
            let top = i32::min(a.top_left.y, b.top_left.y);
            let right = i32::max(a.bottom_right().unwrap().x, b.bottom_right().unwrap().x);
            let bottom = i32::max(a.bottom_right().unwrap().y, b.bottom_right().unwrap().y);
            Rectangle::with_corners(Point::new(left, top), Point::new(right, bottom))
        }
    }

    impl<A, B> Drawable for ZStack<A, B>
    where
        A: Drawable,
        B: Drawable<Color = A::Color>,
    {
        type Color = A::Color;

        type Output = (A::Output, B::Output);

        fn draw<D>(&self, target: &mut D) -> std::prelude::v1::Result<Self::Output, D::Error>
        where
            D: embedded_graphics::prelude::DrawTarget<Color = Self::Color>,
        {
            let a = self.0.draw(target)?;
            let b = self.1.draw(target)?;
            Ok((a, b))
        }
    }
}
