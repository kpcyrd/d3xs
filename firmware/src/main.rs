#![cfg(target_os = "none")]
#![no_std]
#![no_main]

use d3xs_firmware::crypto;
use d3xs_firmware::errors::*;
use esp_backtrace as _;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let mut rng = crypto::Rng::from(hal::Rng::new(peripherals.RNG));

    println!("Hello world!");

    println!("Testing encryption...");
    if crypto::test_sodium_crypto(&mut rng).is_ok() {
        println!("Tests have passed ✨");
    }

    let _ = clocks;
    println!("All clear ✅");

    // Set GPIO7 as an output, and set its state high initially.

    // Initialize the Delay peripheral, and use it to toggle the LED state in a
    // loop.
    loop {}
}
