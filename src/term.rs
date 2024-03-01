use core::ops::RangeBounds;

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

pub const CHAR_HEIGHT: usize = 16;
pub const CHAR_WIDTH: usize = 8;

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
        let letter_size: usize = CHAR_HEIGHT;
        let start = (self.value() - b' ') as usize * letter_size;
        &FONT[start..start + letter_size]
    }

    fn get_letter_pixels(
        &self,
        letter: &mut [Rgb16],
        cols: impl IntoIterator<Item = usize> + Clone,
    ) {
        let letter_bits = self.get_letter_bits();
        let mut i = 0;
        // this only works because a letter is a byte wide
        for &row in letter_bits.iter() {
            for offset in cols.clone() {
                if ((0b1000_0000 >> offset) & row) != 0 {
                    letter[i] = self.foreground;
                }
                i += 1;
            }
        }
    }

    pub fn display(&self, lcd: &mut impl Lcd) {
        let mut letter = [self.background; 8 * 16];
        self.get_letter_pixels(&mut letter, 0..8);
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

    pub fn display_immediately(lcd: &mut impl Lcd, (x, y): (usize, usize), mut ch: Char) {
        ch.mark_flushed();
        lcd.prepare_window(
            ((x * 8) as u16, (x * 8 + 7) as u16),
            ((y * 16) as u16, (y * 16 + 15) as u16),
        );
        ch.display(lcd);
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

pub struct ScrollableRow {
    row_index: usize,
    /// inclusive left bound
    start: usize,
    /// char count
    width: usize,
    /// the number of pixels we have shifted left
    shift: isize,
    foreground: Rgb16,
    background: Rgb16,
}

impl ScrollableRow {
    pub fn new(
        row_index: usize,
        start: usize,
        width: usize,
        background: Rgb16,
        foreground: Rgb16,
    ) -> Self {
        Self {
            row_index,
            start,
            width,
            shift: 0,
            background,
            foreground,
        }
    }

    pub fn shift(&mut self, by: isize) {
        self.shift += by;
    }

    /// Update the values in this region of a terminal with the value that we
    /// have scrolled to
    pub fn stamp<const WIDTH: usize, const HEIGHT: usize>(&self, term: &mut Term<WIDTH, HEIGHT>) {
        todo!()
    }

    fn new_char(&self, val: u8) -> Char {
        Char {
            value: val,
            foreground: self.foreground,
            background: self.background,
        }
    }

    pub fn display(&self, text: &[u8], lcd: &mut impl Lcd) {
        // this algorithm is hell...
        // TODO: make it human-readable
        if self.width == 0 {
            return;
        }

        let text_len = text.len() as isize;
        let y_start = (self.row_index * CHAR_HEIGHT) as u16;
        let y_end = y_start + CHAR_HEIGHT as u16 - 1;
        let x_start = (self.start * CHAR_WIDTH) as u16;

        // idea is to start on the right char and then step from there...
        let char_shift = self.shift / CHAR_WIDTH as isize;
        let char_shift = char_shift % text_len;
        let startch = (0 - char_shift) % text_len;
        let startch = if startch < 0 {
            startch + text_len
        } else {
            startch
        } as usize;

        // we might truncate the first char (and the last) this is the first
        let offset = self.shift % CHAR_WIDTH as isize;
        let startbit = if self.shift <= 0 {
            -offset
        } else {
            CHAR_WIDTH as isize - offset
        } as usize;
        let endbit = CHAR_WIDTH;
        let truncated_width = (endbit - startbit) as u16;

        // print the first char truncated
        let mut letter = [self.background; CHAR_WIDTH * CHAR_HEIGHT];
        self.new_char(text[startch])
            .get_letter_pixels(&mut letter, startbit..endbit);
        lcd.prepare_window((x_start, x_start + truncated_width - 1), (y_start, y_end));
        lcd.write_rgb(&letter[..truncated_width as usize * CHAR_HEIGHT]);

        // print the rest of the chars not truncated
        for i in 1..self.width {
            let ch = text[(startch + i) % text.len()];
            let x_start = x_start + truncated_width + (CHAR_WIDTH as u16) * (i as u16 - 1);
            lcd.prepare_window((x_start, x_start + CHAR_WIDTH as u16 - 1), (y_start, y_end));
            letter = [self.background; CHAR_WIDTH * CHAR_HEIGHT];
            lcd.write_rgb(&letter);
            self.new_char(ch)
                .get_letter_pixels(&mut letter, 0..CHAR_WIDTH);
            lcd.write_rgb(&letter);
        }

        // we want to print the start of the last character
        let (startbit, endbit) = (0, startbit);

        // We need to do this, because preparing a window requires at least 1
        // column to write to ((x, x), (y1, y2)) is for 1 column, we can't do
        // ((x, x - 1), (y1, y2))
        if offset == 0 {
            return;
        }

        // wipe area we will be using
        letter = [self.background; CHAR_WIDTH * CHAR_HEIGHT];

        let rem_width = CHAR_WIDTH as u16 - truncated_width;
        let x_start = x_start + truncated_width + (CHAR_WIDTH as u16) * (self.width as u16 - 1);
        lcd.prepare_window((x_start, x_start + rem_width - 1), (y_start, y_end));
        lcd.write_rgb(&letter[..rem_width as usize * CHAR_HEIGHT]);

        let ch = text[(startch + self.width) % text.len()];
        self.new_char(ch)
            .get_letter_pixels(&mut letter, startbit..endbit);
        lcd.write_rgb(&letter[..rem_width as usize * CHAR_HEIGHT]);

        // for i in (0..rem_width as usize * 16).step_by(rem_width as usize) {
        //     println!("{:?}", &letter[i..i + rem_width as usize]);
        // }
        // println!("char = {:?}", char::from_u32(ch as _));
        // println!("  [{startbit}..{endbit}]");
        // println!(
        //     "  window = {:?}",
        //     ((x_start, x_start + rem_width - 1), (y_start, y_end))
        // );
        // println!(
        //     "  wrote {} pixels",
        //     &letter[..rem_width as usize * CHAR_HEIGHT].len()
        // );
    }
}
