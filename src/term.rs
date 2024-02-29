use esp32s3_hal::Delay;
use esp_println::println;

use crate::lcd::Lcd;
use crate::{lazy_spinlock::LazySpinlock, lcd::Rgb16};

macro_rules! include_rgb565 {
    ($path:expr) => {{
        let bytes = include_bytes!($path);
        Rgb16::from_bytes(bytes)
    }};
}

pub static FONT: &[u8] = include_bytes!("./image/font.bmi");

/// Represents a single character on the terminal, a character has a
/// background and a foreground color, as well as a value.
#[derive(Clone, Copy)]
pub struct Char {
    /// The ascii character at this location, first bit is reserved
    pub value: u8,
    pub foreground: Rgb16,
    pub background: Rgb16,
}

impl Char {
    pub fn is_flushed(&self) -> bool {
        (self.value & 0b1000_0000) == 0
    }

    pub fn mark_clogged(&mut self) {
        self.value |= 0b1000_0000
    }

    pub fn mark_flushed(&mut self) {
        self.value &= 0b0111_1111;
    }

    pub fn value(&self) -> u8 {
        self.value & 0b0111_1111
    }

    /// Get a bit array of pixels reperesenting this letter
    fn get_letter_bits(&self) -> &[u8] {
        let letter_size: usize = 16;
        let start = (self.value() - b' ') as usize * letter_size;
        &FONT[start..start + letter_size]
    }

    fn get_letter_pixels(&self, letter: &mut [Rgb16; 8 * 16]) {
        let letter_bits = self.get_letter_bits();
        for (rowi, &row) in letter_bits.iter().enumerate() {
            for offset in 0..8 {
                if ((0b1000_0000 >> offset) & row) != 0 {
                    letter[rowi * 8 + offset] = self.foreground;
                }
            }
        }
    }

    pub fn display(&self, lcd: &mut impl Lcd) {
        let mut letter = [self.background; 8 * 16];
        self.get_letter_pixels(&mut letter);
        lcd.write_rgb(&letter);
    }
}

impl Default for Char {
    fn default() -> Self {
        Self {
            value: 0b1000_0000 | b' ',
            foreground: Rgb16::WHITE,
            background: Rgb16::BLACK,
        }
    }
}

pub struct Term<const WIDTH: usize, const HEIGHT: usize> {
    cells: [[Char; WIDTH]; HEIGHT],
}

impl<const WIDTH: usize, const HEIGHT: usize> Term<WIDTH, HEIGHT> {
    pub fn new() -> Self {
        Self {
            cells: [[Char::default(); WIDTH]; HEIGHT],
        }
    }

    pub fn display_immediately(lcd: &mut impl Lcd, (x, y): (usize, usize), mut ch: Char) {
        ch.mark_flushed();
        lcd.prepare_window(
            ((x * 8) as u16, (x * 8 + 7) as u16),
            ((y * 16) as u16, (y * 16 + 15) as u16),
        );
        ch.display(lcd);
    }

    pub fn set_char(&mut self, coords: (usize, usize), mut ch: Char) {
        ch.mark_clogged();
        self.cells[coords.1][coords.0] = ch;
    }

    pub fn set_row_chars(&mut self, row: usize, s: &[u8]) {
        for (&s, c) in s.iter().zip(self.cells[row].iter_mut()) {
            c.value = s;
            c.mark_clogged();
        }
    }

    pub fn display(&mut self, lcd: &mut impl Lcd) {
        for (i, row) in self.cells.iter_mut().enumerate() {
            for (j, ch) in row.iter_mut().enumerate() {
                if ch.is_flushed() {
                    continue;
                }
                Self::display_immediately(lcd, (j, i), *ch);
                ch.mark_flushed();
            }
        }
    }
}
