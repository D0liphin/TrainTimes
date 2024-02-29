#![feature(trait_alias)]
#![no_std]
#![no_main]

mod lazy_spinlock;
mod lcd;
mod st7789;
mod term;

mod types {
    use esp32s3_hal::prelude::*;

    /// I'm not quite sure what's going on here -- will leave it as this for now...
    pub trait OutputPinV2 = _embedded_hal_digital_v2_OutputPin;
}

use esp32s3_hal::{
    clock::ClockControl,
    peripherals::Peripherals,
    prelude::*,
    spi::{master::Spi, SpiMode},
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

    let mut term = Term::<30, 15>::new();

    lcd.prepare_window((0, 240), (0, 240));
    for _ in 0..=240 {
        lcd.write_rgb(&[Rgb16::WHITE; 240]);
    }

    let msgs: &[&'static [u8]] = &[
        &b"if Term::<30, 15>::works(t) {"[..],
        &b"    println!(\"yippee!\");"[..],
        &b"}"[..],
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
