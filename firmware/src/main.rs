#![cfg(target_os = "espidf")]

mod keys;

use d3xs_firmware::chall;
use d3xs_firmware::errors::*;
use d3xs_protocol::chall::RingBuffer;
use d3xs_protocol::crypto;
use data_encoding::BASE64;
use esp32_nimble::utilities::{mutex::Condvar, mutex::Mutex, BleUuid};
use esp32_nimble::{BLEDevice, NimbleProperties};
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::sys;
use smart_leds::hsv::RGB;
use smart_leds::SmartLedsWrite;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const SERVICE_UUID: BleUuid = BleUuid::Uuid16(0xffff);
const CHAR_UUID: BleUuid = BleUuid::Uuid16(0xaaaa);
const BLE_NAME: Option<&str> = option_env!("D3XS_BLE_NAME");
const BUZZ_SECONDS: usize = 8;

const LED_RED: RGB<u8> = RGB::new(16, 0, 0);
const LED_GREEN: RGB<u8> = RGB::new(0, 16, 0);
// const LED_YELLOW: RGB<u8> = RGB::new(10, 10, 0);
const LED_OFF: RGB<u8> = RGB::new(0, 0, 0);

#[derive(PartialEq)]
pub enum MainAction {
    LedSuccess,
    LedFail,
}

#[inline(always)]
fn ble_name() -> &'static str {
    BLE_NAME.unwrap_or("esp32c3-d3xs")
}

fn detect_ble_mac() -> Result<String> {
    let mut mac = [0u8; 6];
    let ret =
        unsafe { sys::esp_read_mac(mac.as_mut_ptr() as *mut _, sys::esp_mac_type_t_ESP_MAC_BT) };
    if ret != sys::ESP_OK {
        return Err(Error::EspError("esp_read_mac"));
    }

    let mut s = String::new();
    for b in mac {
        if !s.is_empty() {
            s.push(':');
        }
        write!(s, "{b:02x}").unwrap();
    }
    Ok(s)
}

fn main() -> ! {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("[✨] hello, world!");
    let self_secret_key = keys::door_key();
    let salsa = Arc::new(crypto::SalsaBox::new(&keys::bridge_key(), &self_secret_key));

    let self_public_key = self_secret_key.public_key();
    println!(
        "[🔑] public key: {}",
        BASE64.encode(self_public_key.as_bytes())
    );
    if let Ok(mac) = detect_ble_mac() {
        println!("[🔑] ble mac: {}", mac);
    }

    let peripherals = Peripherals::take().unwrap();
    let mut switch = PinDriver::output(peripherals.pins.gpio4).unwrap();
    switch.set_low().unwrap();

    let mut ws2812 = Ws2812Esp32Rmt::new(0, 8).unwrap();
    ws2812.write([LED_OFF].into_iter()).unwrap();

    let challenges: Arc<Mutex<RingBuffer>> = Arc::new(Mutex::new(RingBuffer::new()));
    let main_action: Arc<Mutex<Option<MainAction>>> = Arc::new(Mutex::new(None));
    let notify: Arc<Condvar> = Arc::new(Condvar::new());
    let notify_mutex = Mutex::new(());

    let ble_device = BLEDevice::take();
    let server = ble_device.get_server();
    server.on_connect(|server, desc| {
        println!("[🤝] client connected");

        server
            .update_conn_params(desc.conn_handle, 24, 48, 0, 60)
            .unwrap();

        // Multi-connect support: start advertising
        ble_device.get_advertising().start().unwrap();
    });
    server.on_disconnect(|_desc, reason| {
        println!("[✌️] client disconnected ({:X})", reason);
    });
    let service = server.create_service(SERVICE_UUID);

    // A writable characteristic
    let characteristic = service
        .lock()
        .create_characteristic(CHAR_UUID, NimbleProperties::READ | NimbleProperties::WRITE);

    let challenges_read = challenges.clone();
    let challenges_write = challenges.clone();
    let main_action_write = main_action.clone();
    let notify_write = notify.clone();

    characteristic
        .lock()
        .on_read(move |attr, _| {
            println!("[🎲] sending nonce");

            if let Some(chall) = challenges_read.lock().current() {
                attr.set_value(&chall.encrypted);
            } else {
                attr.set_value(&[]);
            }
        })
        .on_write(move |args| {
            let buf = args.recv_data;
            println!("[🔍] wrote to writable characteristic: {buf:?}");

            let (action, ret) = {
                let mut chall = challenges_write.lock();
                if chall.verify(buf).is_ok() {
                    println!("[✅] success");
                    chall.reset();
                    (MainAction::LedSuccess, 0)
                } else {
                    (MainAction::LedFail, 1)
                }
            };

            let mut guard = main_action_write.lock();
            // never replace a pending success operation
            if *guard != Some(MainAction::LedSuccess) {
                *guard = Some(action);
            }
            // notify subscribers about a value being available
            notify_write.notify_all();

            // complete ble write operation
            args.reject_with_error_code(ret);
        });

    let ble_advertising = ble_device.get_advertising();
    ble_advertising.name(ble_name());

    println!("[📻] starting ble server");
    ble_advertising.start().unwrap();

    loop {
        {
            let mut challenges = challenges.lock();
            challenges.generate_next::<chall::Random>(&salsa)
        };

        // lock mutex, read action and immediately release mutex
        let action = { main_action.lock().take() };

        if let Some(action) = action {
            match action {
                MainAction::LedSuccess => {
                    switch.set_high().unwrap();
                    for _ in 0..BUZZ_SECONDS {
                        ws2812.write([LED_GREEN].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                        ws2812.write([LED_OFF].into_iter()).unwrap();
                        esp_idf_hal::delay::FreeRtos::delay_ms(250);
                    }
                    switch.set_low().unwrap();

                    // remove any action queued while the door was open
                    *main_action.lock() = None;
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
        } else {
            notify.wait_timeout(notify_mutex.lock(), Duration::from_secs(5));
        }
    }
}
