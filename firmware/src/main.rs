#![cfg(target_os = "espidf")]

use d3xs_firmware::chall::Challenge;
use d3xs_firmware::crypto;
use std::str;
// use d3xs_firmware::errors::*;
use esp32_nimble::utilities::{mutex::Condvar, mutex::Mutex, BleUuid};
use esp32_nimble::{BLEDevice, NimbleProperties};
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::*;
use smart_leds::hsv::RGB;
use smart_leds::SmartLedsWrite;
use std::sync::Arc;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const SERVICE_UUID: BleUuid = BleUuid::Uuid16(0xffff);
const CHAR_UUID: BleUuid = BleUuid::Uuid16(0xaaaa);
const BLE_NAME: Option<&str> = option_env!("BLE_NAME");

#[inline(always)]
fn ble_name() -> &'static str {
    BLE_NAME.unwrap_or("esp32c3-d3xs")
}

fn ctrl_public_key() -> crypto::PublicKey {
    crypto::PublicKey::from([
        0xe8, 0x98, 0xc, 0x86, 0xe0, 0x32, 0xf1, 0xeb, 0x29, 0x75, 0x5, 0x2e, 0x8d, 0x65, 0xbd,
        0xdd, 0x15, 0xc3, 0xb5, 0x96, 0x41, 0x17, 0x4e, 0xc9, 0x67, 0x8a, 0x53, 0x78, 0x9d, 0x92,
        0xc7, 0x54,
    ])
}

fn self_secret_key() -> crypto::SecretKey {
    crypto::SecretKey::from([
        0xb5, 0x81, 0xfb, 0x5a, 0xe1, 0x82, 0xa1, 0x6f, 0x60, 0x3f, 0x39, 0x27, 0xd, 0x4e, 0x3b,
        0x95, 0xbc, 0x0, 0x83, 0x10, 0xb7, 0x27, 0xa1, 0x1d, 0xd4, 0xe7, 0x84, 0xa0, 0x4, 0x4d,
        0x46, 0x1b,
    ])
}

const LED_RED: RGB<u8> = RGB::new(16, 0, 0);
const LED_GREEN: RGB<u8> = RGB::new(0, 16, 0);
// const LED_YELLOW: RGB<u8> = RGB::new(10, 10, 0);
const LED_OFF: RGB<u8> = RGB::new(0, 0, 0);

pub enum MainAction {
    LedSuccess,
    LedFail,
}

fn main() -> ! {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("[~] hello, world!");
    let salsa = Arc::new(crypto::SalsaBox::new(
        &ctrl_public_key(),
        &self_secret_key(),
    ));

    let peripherals = Peripherals::take().unwrap();
    let mut led = PinDriver::output(peripherals.pins.gpio4).unwrap();
    let mut ws2812 = Ws2812Esp32Rmt::new(0, 8).unwrap();
    ws2812.write([LED_OFF].into_iter()).unwrap();

    println!("Testing encryption...");
    if crypto::test_sodium_crypto().is_ok() {
        println!("Tests have passed ✨");
    }
    println!("All clear ✅");

    let latest_nonce: Arc<Mutex<Option<Challenge>>> = Arc::new(Mutex::new(None));
    let main_action: Arc<Mutex<Option<MainAction>>> = Arc::new(Mutex::new(None));
    let notify: Arc<Condvar> = Arc::new(Condvar::new());
    let notify_mutex = Mutex::new(());

    let ble_device = BLEDevice::take();
    let server = ble_device.get_server();
    server.on_connect(|server, desc| {
        println!("[~] client connected");

        server
            .update_conn_params(desc.conn_handle, 24, 48, 0, 60)
            .unwrap();

        // Multi-connect support: start advertising
        ble_device.get_advertising().start().unwrap();
    });
    server.on_disconnect(|_desc, reason| {
        println!("[~] client disconnected ({:X})", reason);
    });
    let service = server.create_service(SERVICE_UUID);

    // A writable characteristic
    let characteristic = service
        .lock()
        .create_characteristic(CHAR_UUID, NimbleProperties::READ | NimbleProperties::WRITE);

    let latest_nonce_read = latest_nonce.clone();
    let latest_nonce_write = latest_nonce.clone();
    let main_action_write = main_action.clone();
    let notify_write = notify.clone();
    let salsa_write = salsa.clone();

    characteristic
        .lock()
        .on_read(move |attr, _| {
            println!("[~] sending nonce");

            if let Some(chall) = &*latest_nonce_read.lock() {
                attr.set_value(&chall.encrypted);
            } else {
                attr.set_value(&[]);
            }
        })
        .on_write(move |args| {
            let s = str::from_utf8(args.recv_data);
            println!("[~] wrote to writable characteristic: {:?}", s);

            if args.recv_data.len() == 1 {
                let mut guard = main_action_write.lock();
                *guard = Some(MainAction::LedSuccess);
                notify_write.notify_all();
            }

            if args.recv_data.len() == 2 {
                let mut guard = main_action_write.lock();
                *guard = Some(MainAction::LedFail);
                notify_write.notify_all();
            }

            if let Some(chall) = &*latest_nonce_write.lock() {
                if chall.verify(&salsa_write, &args.recv_data).is_ok() {
                    // success
                    println!("[~] success");
                    args.reject_with_error_code(0);
                } else {
                    args.reject();
                }
            } else {
                args.reject();
            }
        });

    let ble_advertising = ble_device.get_advertising();
    ble_advertising.name(ble_name());

    println!("[~] starting ble server");
    ble_advertising.start().unwrap();

    loop {
        if let Ok(chall) = Challenge::generate(&salsa) {
            *latest_nonce.lock() = Some(chall);
        }

        if let Some(action) = main_action.lock().take() {
            match action {
                MainAction::LedSuccess => {
                    led.set_high().unwrap();
                    for _ in 0..10 {
                        ws2812.write([LED_GREEN].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                        ws2812.write([LED_OFF].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                    }
                    led.set_low().unwrap();
                }
                MainAction::LedFail => {
                    for _ in 0..2 {
                        ws2812.write([LED_RED].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                        ws2812.write([LED_OFF].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                    }
                }
            }
        }

        notify.wait_timeout(notify_mutex.lock(), Duration::from_secs(5));
    }
}
