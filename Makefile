.PHONY: build wasm bridge binaries firmware

build: wasm binaries firmware

wasm:
	CARGO_TARGET_DIR=$${CARGO_TARGET_DIR:-$(shell pwd)/target-wasm} wasm-pack build --release --target web --mode no-install protocol/

binaries: wasm bridge
	cargo build --release --locked -p d3xs

bridge:
	cargo build --release --locked -p d3xs-bridge

firmware:
	cd firmware; CARGO_TARGET_DIR=$${CARGO_TARGET_DIR:-$(shell pwd)/target-firmware} RUSTC_BOOTSTRAP=1 cargo build --release
