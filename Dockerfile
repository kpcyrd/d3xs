# temporarily build on :edge until wasm-bindgen is available in a release

FROM alpine:edge
ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN --mount=type=cache,target=/var/cache/apk ln -vs /var/cache/apk /etc/apk/cache && \
    apk add binaryen cargo dbus-dev make musl-dev pkgconf wasm-bindgen wasm-pack && \
    rm /etc/apk/cache
WORKDIR /app
COPY ./ /app
RUN --mount=type=cache,target=/var/cache/buildkit \
    CARGO_HOME=/var/cache/buildkit/cargo \
    CARGO_TARGET_DIR=/var/cache/buildkit/target-wasm \
    make wasm && \
    CARGO_HOME=/var/cache/buildkit/cargo \
    CARGO_TARGET_DIR=/var/cache/buildkit/target \
    cargo build --release --locked -p d3xs -p d3xs-bridge && \
    cp -v /var/cache/buildkit/target/release/d3xs* .
RUN strip d3xs d3xs-bridge

FROM alpine:edge
RUN apk add libgcc dbus-libs
COPY --from=0 /app/d3xs /app/d3xs-bridge /usr/bin
USER 1000
ENV D3XS_BIND=0.0.0.0:8080
ENTRYPOINT ["d3xs"]
