use core::fmt::Debug;

use crate::types::OutputPinV2;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
/// This should be opaque -- we actually disallow 0xffff as a value, which
/// we reserve as "transparent". `0b11111_111111_11110` is black. The
/// original color does not exist.
pub struct Rgb16(u8, u8);

impl From<u16> for Rgb16 {
    fn from(value: u16) -> Self {
        let [first, second] = value.to_be_bytes();
        Rgb16(first, second)
    }
}

impl Debug for Rgb16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:b}", u16::from_be_bytes([self.0, self.1]))
    }
}

impl Rgb16 {
    pub const IGNORE: Self = Self(0xff, 0xff - 1);
    pub const BLACK: Self = Self(0xff, 0xff);
    pub const WHITE: Self = Self(0x00, 0x00);

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r: u16 = (0b11111 * r as u16 / 255) << 11;
        let g: u16 = (0b111111 * g as u16 / 255) << 5;
        let b: u16 = 0b11111 * b as u16 / 255;

        let this = Self::from(r | g | b);
        if this == Self::IGNORE {
            Self::BLACK
        } else {
            this
        }
    }

    pub fn as_bytes(buf: &[Rgb16]) -> &[u8] {
        // SAFETY: safe because Rgb16 is size 2 (repr C) and we resize to
        // twice the length. This is just a mirror of `buf`, so it can live
        // for exactly the same lifetime.
        unsafe { core::slice::from_raw_parts(buf as *const [Rgb16] as _, buf.len() * 2) }
    }

    pub const fn from_bytes(buf: &[u8]) -> &[Rgb16] {
        unsafe { core::slice::from_raw_parts(buf as *const [u8] as _, buf.len() / 2) }
    }
}

pub trait Lcd {
    /// Set the window, such that subsequent writes will write to this
    /// region -- this means we also want to set `RAMWR`
    fn prepare_window(&mut self, x: (u16, u16), y: (u16, u16));

    /// Write a single pixel to the LCD. `rgb` should not be mutated such
    /// that it will have a visible effect on `rgb`. What we want
    fn write_rgb(&mut self, rgb: &[Rgb16]);
}