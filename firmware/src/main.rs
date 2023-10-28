#![cfg(target_os = "espidf")]

use d3xs_firmware::crypto;
use std::str;
// use d3xs_firmware::errors::*;
use esp32_nimble::utilities::BleUuid;
use esp32_nimble::{BLEDevice, NimbleProperties};

const SERVICE_UUID: BleUuid = BleUuid::Uuid16(0xffff);
const CHAR_UUID: BleUuid = BleUuid::Uuid16(0xaaaa);
const BLE_NAME: Option<&str> = option_env!("BLE_NAME");

#[inline(always)]
fn ble_name() -> &'static str {
    BLE_NAME.unwrap_or("esp32c3-d3xs")
}

fn main() -> ! {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("[~] hello, world!");

    println!("Testing encryption...");
    if crypto::test_sodium_crypto().is_ok() {
        println!("Tests have passed ✨");
    }
    println!("All clear ✅");

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
    characteristic
        .lock()
        .on_read(move |attr, _| {
            println!("[~] sending nonce");
            let mut nonce = [0u8; 32];
            crypto::getrandom(&mut nonce);
            attr.set_value(&nonce);
        })
        .on_write(move |args| {
            let s = str::from_utf8(args.recv_data);
            println!("[~] wrote to writable characteristic: {:?}", s);
            // TODO: validate nonce

            // success
            args.reject_with_error_code(0);
        });

    let ble_advertising = ble_device.get_advertising();
    ble_advertising.name(ble_name());

    println!("[~] starting ble server");
    ble_advertising.start().unwrap();

    loop {
        esp_idf_hal::delay::FreeRtos::delay_ms(50);
    }
}
