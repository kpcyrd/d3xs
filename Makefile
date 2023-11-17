.PHONY: build wasm binaries firmware

build: wasm binaries firmware

wasm:
	CARGO_TARGET_DIR=$(PWD)/wasm-target wasm-pack build --release --target web protocol/

binaries:
	cargo build --release -p d3xs -p d3xs-bridge

firmware:
	cd firmware; CARGO_TARGET_DIR=$(PWD)/firmware-target RUSTC_BOOTSTRAP=1 cargo build --release
