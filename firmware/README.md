# d3xs-firmware

Firmware for `esp32c3` microcontroller (riscv32imc-esp-espidf).

## Building and testing the firmware

For basic usage refer to the regular README in the parent directory.

For development you can use this command to compile the firmware, flash it to an attached esp32c3 and open the serial monitor after flashing:

```sh
cd firmware
D3XS_BRIDGE_KEY="cW49lkXDeM0wOT8N7QxAWePmWs8xZK1FXt1uQT/pcG4=" RUSTC_BOOTSTRAP=1 cargo espflash flash --release -M
```

## Unit testing

For development, run these from **outside** of the firmware directory. This is to avoid loading the firmware configuration from `firmware/.cargo/`.
```
cargo watch -- cargo test -p d3xs-firmware
cargo check -p d3xs-firmware --lib
cargo clippy -p d3xs-firmware --lib
```
