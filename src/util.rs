//! HAL doesn't really work with my IDE... at all. So, I'm redefining a lot of
//! the stuff here so that I can actually use it.

use esp32s3_hal::peripherals::Peripherals;

pub fn take_peripherals() -> Peripherals {
    Peripherals::take()
} 


/*
MOSI -> CIPO = Controller In, Peripheral Out
CS = Chip Select
SCK = Serial Clock

*/
