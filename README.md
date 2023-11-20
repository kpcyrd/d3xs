# d3xs 🔑🚪🧀

**Grant access to rooms with links instead of physical keys**

This project consists of the following components:

- A webserver for hosting assets and accepting websocket connections from the public internet
- A bridge that keeps track of configuration, authorization, connects to both the public websocket server and any microcontrollers
- A firmware for an **esp32c3** (riscv32, fairly affordable at about 6-8€ each) to control GPIO pins with a challenge/response protocol over bluetooth low energy

All components including the microcontroller firmware are written in Rust.

## 🏗️ Compiling

To build this project you need:

- A Rust compiler
- pkg-config
- make
- The dbus library and header files (`dbus` on Arch Linux, `libdbus-1-dev` on Debian, `dbus-dev` on Alpine)
- [wasm-pack](https://github.com/rustwasm/wasm-pack)
- [cargo-espflash](https://github.com/esp-rs/espflash) (optional)

```sh
git clone https://github.com/kpcyrd/d3xs
cd d3xs
make binaries
```

It's also possible to build just the bridge without needing `wasm-pack`:

```sh
git clone https://github.com/kpcyrd/d3xs
cd d3xs
cargo build --release --locked -p d3xs-bridge
```

The built binaries are then available at `./target/release/d3xs` and `./target/release/d3xs-bridge` respectively. You can copy them to `/usr/bin/` or `~/bin/` in your home folder.

## 🔑 Generating a bridge key

With the `d3xs-bridge` binary you can now generate yourself a bridge key (this is also the starting point of your configuration file):

```
$ d3xs-bridge keygen --bridge
[system]
# public_key = "cW49lkXDeM0wOT8N7QxAWePmWs8xZK1FXt1uQT/pcG4="
secret_key = "D6Ir3Ql7jYStdzIiIgCEZuc0L/TFNqQhH08kSNP3gpM="
```

See `example.toml` for further configuration.

## 📟 Compiling and flashing firmware

The firmware has the relevant keypairs baked into itself, so they need to be provided in environment variables during build. You can generate one like this:

```
$ d3xs-bridge keygen --firmware
# [doors.building]
# label = "Building"
# mac = "ec:da:3b:ff:ff:ff"
# public_key = "iNg2AUD8ONIHzqd7jqJt9aP8k04o1ZyZ7UyCo5OQmDQ="
D3XS_DOOR_KEY="w/CSnPJnWTaEIYpEvXvF+ktwh236iSDZfSx6hExB4bM="
```

The output outputs a secret key and example configuration on how to add this microcontroller to your configuration file.

Building the firmware using the secret key we just generated, and the bridge key of the previous step:

```sh
D3XS_DOOR_KEY="w/CSnPJnWTaEIYpEvXvF+ktwh236iSDZfSx6hExB4bM=" \
D3XS_BRIDGE_KEY="cW49lkXDeM0wOT8N7QxAWePmWs8xZK1FXt1uQT/pcG4=" \
make firmware
```

If `D3XS_DOOR_KEY` is not provided, a random one will be generated during build, the public key can be read from serial output during boot (as well as the bluetooth mac address). The `D3XS_BRIDGE_KEY` variable however is important or you won't be able to send any commands to the microcontroller.

You can also customize the bluetooth name by adding something like `D3XS_BLE_NAME=d3xs1`.

To flash the firmware to an attached esp32c3 use:

```sh
$ espflash flash target-firmware/riscv32imc-esp-espidf/release/d3xs-firmware --monitor
```

With `--monitor` espflash is automatically going to open the serial interface after flashing to read the boot log, this flag is optional and can be omitted though.

For more documentation see the [firmware folder](firmware/).

## 👥 Adding users

To add a user you can generate them a keypair:

```
$ d3xs-bridge keygen -pn alice
[users.alice]
# https://example.com/alice#ctkuV7vV8lSv6EpZt/e9tR9l1NqjF9A4Le8W8VlyZoQ=
public_key = "Ewok6RkMPbwbN3Vvdq5ajImlqks9uoBTvPBCfzOYKSg="
authorize = []
```

To grant access to rooms add the room id to the `authorize` list. Your final configuration may look like this:

```toml
[system]
# public_key = "cW49lkXDeM0wOT8N7QxAWePmWs8xZK1FXt1uQT/pcG4="
secret_key = "D6Ir3Ql7jYStdzIiIgCEZuc0L/TFNqQhH08kSNP3gpM="

[doors.home]
label = "Home"
mac = "ec:da:3b:ff:ff:ff"
public_key = "iNg2AUD8ONIHzqd7jqJt9aP8k04o1ZyZ7UyCo5OQmDQ="

[users.alice]
# https://example.com/alice#ctkuV7vV8lSv6EpZt/e9tR9l1NqjF9A4Le8W8VlyZoQ=
public_key = "Ewok6RkMPbwbN3Vvdq5ajImlqks9uoBTvPBCfzOYKSg="
authorize = ["home"]
```

The bridge automatically syncs the relevant parts of the configuration to the public webserver.

## ☁️ Setting up network access

The d3xs binary contains a webserver with embedded assets for the web interface. It has no configuration besides the port to bind to, and a uuid that is used as shared secret to authenticate the bridge.

```sh
d3xs -B 127.0.0.1:5000 2120a559-2fbd-4595-be57-4e78changeme
```

For security reasons this interface should be secured with https instead of exposing it directly to the network.

After this is setup you can start the bridge and connect it to the public webserver:

```sh
d3xs-bridge connect --config example.toml wss://example.com/bridge/2120a559-2fbd-4595-be57-4e78changeme
```

The url can also be configured in the config file:

```toml
[bridge]
# public_key = "cW49lkXDeM0wOT8N7QxAWePmWs8xZK1FXt1uQT/pcG4="
secret_key = "D6Ir3Ql7jYStdzIiIgCEZuc0L/TFNqQhH08kSNP3gpM="
url = "wss://example.com/bridge/2120a559-2fbd-4595-be57-4e78changeme"
```

To start the bridge automatically at boot there's a reference openrc config at `contrib/d3xs-bridge.init`.

## ⚖️ License

`GPL-3.0-or-later`

THERE IS NO WARRANTY FOR THE PROGRAM, TO THE EXTENT PERMITTED BY
APPLICABLE LAW.  EXCEPT WHEN OTHERWISE STATED IN WRITING THE COPYRIGHT
HOLDERS AND/OR OTHER PARTIES PROVIDE THE PROGRAM "AS IS" WITHOUT WARRANTY
OF ANY KIND, EITHER EXPRESSED OR IMPLIED, INCLUDING, BUT NOT LIMITED TO,
THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
PURPOSE.  THE ENTIRE RISK AS TO THE QUALITY AND PERFORMANCE OF THE PROGRAM
IS WITH YOU.  SHOULD THE PROGRAM PROVE DEFECTIVE, YOU ASSUME THE COST OF
ALL NECESSARY SERVICING, REPAIR OR CORRECTION.
