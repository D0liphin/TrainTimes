#![no_std]
#![no_main]

use core::mem::transmute;

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

/// esp_println, but maybe I want to make it write errors?
macro_rules! eprintln {
    ($($t:tt)*) => {
        println!($($t)*)
    };
}

mod st7789 {
    use core::ops::RangeBounds;

    use esp32s3_hal::{
        peripherals::SPI2,
        prelude::{_embedded_hal_blocking_spi_Write, *},
        spi::{master::Spi, FullDuplexMode},
        Delay,
    };
    use esp_println::println;

    pub struct St7789<
        'a,
        Dc: _embedded_hal_digital_v2_OutputPin,
        Bl: _embedded_hal_digital_v2_OutputPin,
    > {
        pub spi: Spi<'a, SPI2, FullDuplexMode>,
        pub dc: Dc,
        pub bl: Bl,
    }

    impl<'a, Dc: _embedded_hal_digital_v2_OutputPin, Bl: _embedded_hal_digital_v2_OutputPin>
        St7789<'a, Dc, Bl>
    {
        /// SPI write and flush
        /// Logs errors -- because what are we going to do anyway?
        pub fn write_bytes(&mut self, bytes: &[u8]) {
            if let Err(e) = self.spi.write(bytes) {
                eprintln!("{:?}", e);
            }
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

fn pixels_as_bytes(bytes: &[u16]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(bytes as *const [u16] as _, bytes.len() * 2) }
}

fn main2() -> ! {
    println!("running...");
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
        spi: Spi::new(peripherals.SPI2, 100u32.kHz(), SpiMode::Mode0, &clocks)
            .with_sck(sck)
            .with_mosi(mosi)
            .with_cs(cs),
        dc,
        bl,
    };
    lcd.init(&mut delay);
    lcd.set_bl_high();

    let mut red = [0u16; 240 * 2401];
    for (i, b) in red.iter_mut().enumerate() {
        *b = 0b11111_000000_11111;
    }

    let mut blue = [0u16; 240 * 240];
    for (i, b) in blue.iter_mut().enumerate() {
        *b = 0b11111_111111_00000;
    }

    loop {
        println!("did a loop");
        lcd.write_pixels(pixels_as_bytes(&red));
        delay.delay_ms(100u32);
        lcd.write_pixels(pixels_as_bytes(&blue));
        delay.delay_ms(100u32);
    }
}

#[entry]
fn main() -> ! {
    main2()
}
