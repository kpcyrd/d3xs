# d3xs-firmware

Firmware for `esp32c3` microcontroller (riscv32imc-unknown-none-elf).

## Building and testing the firmware

Build the firmware binary `target/riscv32imc-unknown-none-elf/release/d3xs-firmware`:
```
cd firmware/
RUSTC_BOOTSTRAP=1 cargo build --release
```

Flash to an attached esp32c3:
```
cd ..
espflash flash ./target/riscv32imc-unknown-none-elf/release/d3xs-firmware
```

For quickly editing the firmware and opening a serial monitor after flashing you can use this command:
```
cd firmware/
RUSTC_BOOTSTRAP=1 cargo espflash flash --release -M
```

## Unit testing

For development, run these from **outside** of the firmware directory. This is to avoid loading the firmware configuration from `firmware/.cargo/`.
```
cargo watch -- cargo test -p d3xs-firmware
cargo check -p d3xs-firmware --lib
cargo clippy -p d3xs-firmware --lib
```
