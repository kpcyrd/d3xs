.PHONY: build wasm binaries firmware

build: wasm binaries firmware

wasm:
	CARGO_TARGET_DIR=$${CARGO_TARGET_DIR:-$(PWD)/target-wasm} wasm-pack build --release --target web protocol/

binaries:
	cargo build --release --locked -p d3xs -p d3xs-bridge

firmware:
	cd firmware; CARGO_TARGET_DIR=$${CARGO_TARGET_DIR:-$(PWD)/target-firmware} RUSTC_BOOTSTRAP=1 cargo build --release
