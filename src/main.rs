#![feature(trait_alias)]
#![no_std]
#![no_main]

use core::mem::transmute;

mod types {
    use esp32s3_hal::prelude::*;

    /// I'm not quite sure what's going on here -- will leave it as this for now...
    pub trait OutputPinV2 = _embedded_hal_digital_v2_OutputPin;
}

use esp32s3_hal::{
    clock::ClockControl,
    gpio::OutputPin,
    peripherals::{Peripherals, SPI2},
    prelude::*,
    spi::{master::Spi, FullDuplexMode, SpiMode},
    Delay, IO,
};
use esp_backtrace as _;
use esp_println::println;
use lcd::Rgb16;

use crate::{
    lcd::Lcd,
    term::{Char, Term},
};

/// esp_println, but maybe I want to make it write errors?
macro_rules! eprintln {
    ($($t:tt)*) => {
        println!($($t)*)
    };
}

mod st7789 {
    use crate::types::OutputPinV2;
    use core::ops::RangeBounds;

    use esp32s3_hal::{
        peripherals::SPI2,
        prelude::*,
        spi::{master::Spi, FullDuplexMode},
        Delay,
    };
    use esp_println::println;

    pub struct St7789<'a, Dc: OutputPinV2, Bl: OutputPinV2> {
        pub spi: Spi<'a, SPI2, FullDuplexMode>,
        pub dc: Dc,
        pub bl: Bl,
    }

    impl<'a, Dc: OutputPinV2, Bl: OutputPinV2> St7789<'a, Dc, Bl> {
        /// SPI write_bytes
        /// Logs errors -- because what are we going to do anyway?
        pub fn write_bytes(&mut self, bytes: &[u8]) {
            self.spi.write(bytes);
        }

        pub fn flush(&mut self) {
            _ = self.spi.write(&[]);
        }

        pub fn write_command(&mut self, command: u8, data: &[u8]) {
            self.write_commands(&[command]);
            self.write_data(data);
        }

        /// `write_bytes`, but first set command mode
        pub fn write_commands(&mut self, bytes: &[u8]) {
            _ = self.dc.set_low();
            self.write_bytes(bytes);
        }

        /// `write_bytes`, but first set data mode
        pub fn write_data(&mut self, bytes: &[u8]) {
            _ = self.dc.set_high();
            self.write_bytes(bytes);
        }

        /// Initialize this device, chucks a bunch of magic bytes into the
        /// peripheral, that I stole from some other pyhton code.
        pub fn init(&mut self, delay: &mut Delay) {
            self.write_command(SWRESET, &[]); // software reset
            delay.delay_ms(300u32); // we have to sleep -- not sure why
            println!("SWRESET");

            self.write_command(MADCTL, &[0x70]);

            // frame rate control -- idle mode
            self.write_command(FRMCTR2, &[0x0c, 0x0c, 0x00, 0x33, 0x33]);
            self.write_command(COLMOD, &[0x05]);
            self.write_command(GCTRL, &[0x14]);
            self.write_command(VCOMS, &[0x37]);

            // power control
            self.write_command(LCMCTRL, &[0x2c]);
            self.write_command(VDVVRHEN, &[0x01]);
            self.write_command(VRHS, &[0x12]);
            self.write_command(VDVS, &[0x20]);

            self.write_command(0xd0, &[0xa4, 0xa1]);
            self.write_command(FRCTRL2, &[0x0f]);

            // set gamma
            self.write_command(
                GMCTRP1,
                &[
                    0xd0, 0x04, 0x0d, 0x11, // not sure
                    0x13, 0x2b, 0x3f, 0x54, // what this
                    0x4c, 0x18, 0x0d, 0x0b, // does (comments are for rustfmt)
                    0x1f, 0x23,
                ],
            );

            // set gamma
            self.write_command(
                GMCTRN1,
                &[
                    0xd0, 0x04, 0x0c, 0x11, //
                    0x13, 0x2c, 0x3f, 0x44, //
                    0x51, 0x2f, 0x1f, 0x1f, //
                    0x20, 0x23,
                ],
            );

            self.write_command(SLPOUT, &[]);
            self.write_command(DISPON, &[]);
            println!("DISPON");
            delay.delay_ms(300u32);
        }

        pub fn set_bl_high(&mut self) {
            _ = self.bl.set_high();
        }

        /// Set the window x and y ranges, e.g.
        /// ```no_run
        /// lcd.set_window((0, 240), (0, 240));
        /// ```
        pub fn set_window(&mut self, x: (u16, u16), y: (u16, u16)) {
            fn b(n: u16) -> (u8, u8) {
                ((n >> 8) as u8, n as u8)
            }

            self.write_command(CASET, &[b(x.0).0, b(x.0).1, b(x.1).0, b(x.1).1]);
            self.write_command(RASET, &[b(y.0).0, b(y.0).1, b(y.1).0, b(y.1).1]);
        }

        pub fn write_pixels(&mut self, pixels: &[u8]) {
            self.set_window((0, 240), (0, 240));
            self.write_command(RAMWR, pixels);
        }
    }

    pub const NOP: u8 = 0x00;
    pub const SWRESET: u8 = 0x01;
    pub const RDDID: u8 = 0x04;
    pub const RDDST: u8 = 0x09;

    pub const SLPIN: u8 = 0x10;
    pub const SLPOUT: u8 = 0x11;
    pub const PTLON: u8 = 0x12;
    pub const NORON: u8 = 0x13;

    pub const INVOFF: u8 = 0x20;
    pub const INVON: u8 = 0x21;
    pub const DISPOFF: u8 = 0x28;
    pub const DISPON: u8 = 0x29;

    pub const CASET: u8 = 0x2A;
    pub const RASET: u8 = 0x2B;
    pub const RAMWR: u8 = 0x2C;
    pub const RAMRD: u8 = 0x2E;

    pub const PTLAR: u8 = 0x30;
    pub const MADCTL: u8 = 0x36;
    pub const COLMOD: u8 = 0x3A;

    pub const FRMCTR1: u8 = 0xB1;
    pub const FRMCTR2: u8 = 0xB2;
    pub const FRMCTR3: u8 = 0xB3;
    pub const INVCTR: u8 = 0xB4;
    pub const DISSET5: u8 = 0xB6;

    pub const GCTRL: u8 = 0xB7;
    pub const GTADJ: u8 = 0xB8;
    pub const VCOMS: u8 = 0xBB;

    pub const LCMCTRL: u8 = 0xC0;
    pub const IDSET: u8 = 0xC1;
    pub const VDVVRHEN: u8 = 0xC2;
    pub const VRHS: u8 = 0xC3;
    pub const VDVS: u8 = 0xC4;
    pub const VMCTR1: u8 = 0xC5;
    pub const FRCTRL2: u8 = 0xC6;
    pub const CABCCTRL: u8 = 0xC7;

    pub const RDID1: u8 = 0xDA;
    pub const RDID2: u8 = 0xDB;
    pub const RDID3: u8 = 0xDC;
    pub const RDID4: u8 = 0xDD;

    pub const GMCTRP1: u8 = 0xE0;
    pub const GMCTRN1: u8 = 0xE1;

    pub const PWCTR6: u8 = 0xFC;
}

mod bmp {
    use esp_println::println;

    use crate::lcd::Rgb16;

    /// Extract the data of a `bmp` image. The assumption is that you know the
    /// format already, and you just neeed the data. Panics if the BMP is not valid.
    pub fn bmp_data(bytes: &[u8]) -> &[u8] {
        // BMP is a little-endian format
        let offset = u32::from_le_bytes(
            bytes[10..14]
                .try_into()
                .expect("caller asserts this is valid bmp, which always contains a 14 byte header"),
        ) as usize;
        &bytes[offset..]
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct Rgba(u8, u8, u8, u8);

    impl Rgba {
        /// Converts to RGB565, ignoring alpha completely
        pub fn to_rgb16(&self) -> Rgb16 {
            if self.3 == 0 {
                return Rgb16::IGNORE;
            }

            Rgb16::from_rgb(self.0, self.1, self.2)
        }
    }

    pub fn bytes_as_rgba(bytes: &[u8]) -> &[Rgba] {
        unsafe { core::slice::from_raw_parts(bytes as *const [u8] as _, bytes.len() / 4) }
    }

    pub fn bytes_as_rgb16(bytes: &[u8], buf: &mut [Rgb16]) {
        let rgbas = bytes_as_rgba(bytes);
        for (i, color) in rgbas.iter().enumerate() {
            println!("{i}");
            buf[i] = color.to_rgb16();
        }
    }
}

mod lcd {
    use core::fmt::Debug;

    use crate::st7789::{St7789, RAMWR};
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

    impl<'a, Dc, Bl> Lcd for St7789<'a, Dc, Bl>
    where
        Dc: OutputPinV2,
        Bl: OutputPinV2,
    {
        fn prepare_window(&mut self, x: (u16, u16), y: (u16, u16)) {
            self.set_window(x, y);
            self.write_commands(&[RAMWR]);
        }

        fn write_rgb(&mut self, rgb: &[Rgb16]) {
            self.write_data(Rgb16::as_bytes(rgb));
        }
    }
}

mod lazy_spinlock {
    use core::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        sync::atomic::{AtomicI8, Ordering},
    };

    use esp_println::println;

    const UNINIT: i8 = 0;
    const INIT: i8 = 1;
    const LOCKED: i8 = 2;

    pub struct LazySpinlock<T, F> {
        value: UnsafeCell<MaybeUninit<T>>,
        init: F,
        state: AtomicI8,
    }

    unsafe impl<T, F> Sync for LazySpinlock<T, F> {}

    impl<T, F> LazySpinlock<T, F>
    where
        F: FnOnce() -> T + Copy,
    {
        pub const fn uninit(init: F) -> Self {
            Self {
                value: UnsafeCell::new(MaybeUninit::uninit()),
                init,
                state: AtomicI8::new(UNINIT),
            }
        }

        /// Initialize if not yet init
        pub fn initialize(&self) {
            println!("initialize()");
            loop {
                println!("loop...");
                if self.state.load(Ordering::Relaxed) == INIT {
                    return;
                }

                // This isn't likely to happen anyway
                while self.state.load(Ordering::Relaxed) == LOCKED {
                    println!("looping some more");
                }

                let result = self.state.compare_exchange(
                    UNINIT,
                    LOCKED,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );
                // If we fail to lock, someone else is initializing, so let's
                // just wait for them to do it
                if result.is_err() {
                    continue;
                }

                unsafe {
                    *self.value.get() = MaybeUninit::new((self.init)());
                }

                self.state.store(INIT, Ordering::Relaxed);
            }
        }

        pub fn is_init(&self) -> bool {
            self.state.load(Ordering::Relaxed) == INIT
        }

        pub fn get(&self) -> &T {
            self.initialize();
            // SAFETY: we run `self.initialize()` first, so it's going to have
            // to initialize!
            unsafe { (*self.value.get()).assume_init_ref() }
        }
    }
}

mod term {
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

    pub static FONT: &[Rgb16] = include_rgb565!("./image/font.rgb565");

    /// Represents a single character on the terminal, a character has a
    /// background and a foreground color, as well as a value.
    #[derive(Clone, Copy)]
    pub struct Char {
        /// The ascii character at this location, first bit is reserved
        pub value: u8,
        // pub foreground: Rgb16,
        // pub background: Rgb16,
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

        /// Get a byte array of pixels reperesenting this letter
        pub fn get_letter(&self) -> &[Rgb16] {
            let letter_size: usize = 16 * 8;
            let start = (self.value() - b' ') as usize * letter_size;
            let start = start as usize;
            &FONT[start..start + letter_size]
        }

        pub fn write_foreground(&self, lcd: &mut impl Lcd) {
            lcd.write_rgb(self.get_letter());
        }
    }

    impl Default for Char {
        fn default() -> Self {
            Self {
                value: 0b1000_0000 | b' ',
                // foreground: Rgb16::WHITE,
                // background: Rgb16::BLACK,
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

        pub fn set_char(&mut self, coords: (usize, usize), mut val: Char) {
            val.mark_clogged();
            self.cells[coords.1][coords.0] = val;
        }

        pub fn set_row_vals(&mut self, row: usize, s: &[u8]) {
            for (&s, c) in s.iter().zip(self.cells[row].iter_mut()) {
                c.value = s;
                c.mark_clogged();
            }
        }

        pub fn display(&mut self, lcd: &mut impl Lcd) {
            for (i, row) in self.cells.iter_mut().enumerate() {
                for (j, c) in row.iter_mut().enumerate() {
                    if c.is_flushed() {
                        continue;
                    }
                    // TODO: white border
                    lcd.prepare_window(
                        ((j * 8) as u16, (j * 8 + 7) as u16),
                        ((i * 16) as u16, (i * 16 + 15) as u16),
                    );
                    c.write_foreground(lcd);
                    c.mark_flushed();
                }
            }
        }
    }
}

fn pixels_as_bytes(bytes: &[u16]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(bytes as *const [u16] as _, bytes.len() * 2) }
}

fn main2() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    let cs = io.pins.gpio4;
    let sck = io.pins.gpio5;
    let mosi = io.pins.gpio6;
    let dc = io.pins.gpio7.into_push_pull_output();
    let bl = io.pins.gpio15.into_push_pull_output();
    let mut lcd = st7789::St7789 {
        spi: Spi::new(peripherals.SPI2, 80u32.MHz(), SpiMode::Mode0, &clocks)
            .with_sck(sck)
            .with_mosi(mosi)
            .with_cs(cs),
        dc,
        bl,
    };
    lcd.init(&mut delay);
    lcd.set_bl_high();

    delay.delay_ms(100u32);
    let mut term = Term::<30, 15>::new();

    lcd.prepare_window((0, 240), (0, 240));
    for _ in 0..=240 {
        lcd.write_rgb(&[Rgb16::WHITE; 240]);
    }

    let msgs: &[&'static [u8]] = &[
        &b"if Term::<30, 15>::works(t) {"[..],
        &b"    println!(\"yippee!\");"[..],
        &b"}"[..]
    ];
    for (i, msg) in msgs.iter().enumerate() {
        for (j, &b) in msg.iter().enumerate() {
            term.set_char((j, i), Char { value: b });
            term.display(&mut lcd);
        }
    }

    // let line = concat![
    //     "hello, world! this line is too long to fit... as a result, I am scrolling it along at a ",
    //     "relatively slow pace. the other option would be to do this pixel by pixel. this makes the ",
    //     "code a bit more complicated though, so i'm hesitant! :o "
    // ].as_bytes();

    // let mut i = 0;
    // loop {
    //     delay.delay_ms(100u32);
    //     term.set_row_vals(0, &line[i..]);
    //     i = (i + 1) % line.len();
    //     term.display(&mut lcd);
    // }

    println!("here");
    loop {}
}

#[entry]
fn main() -> ! {
    println!("hi");
    main2();
}
