use core::fmt::Debug;

use crate::types::OutputPinV2;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rgb16(u8, u8);

impl From<u16> for Rgb16 {
    fn from(value: u16) -> Self {
        let [first, second] = value.to_be_bytes();
        Rgb16(first, second)
    }
}

impl Debug for Rgb16 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // write!(f, "{:b}", u16::from_be_bytes([self.0, self.1]))
        write!(f, "{}", if self == &Rgb16::BLACK { ' ' } else { '#' })
    }
}

impl Rgb16 {
    pub const BLACK: Self = Self(0xff, 0xff);
    pub const WHITE: Self = Self(0x00, 0x00);

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r_565 = r as u16 >> 3;
        let g_565 = g as u16 >> 2;
        let b_565 = b as u16 >> 3;

        let inverted = (r_565 << 11) | (g_565 << 5) | b_565;

        Self::from(inverted ^ 0xffff)
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
