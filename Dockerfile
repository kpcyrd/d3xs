FROM alpine:3.18
ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN --mount=type=cache,target=/var/cache/apk ln -vs /var/cache/apk /etc/apk/cache && \
    apk add musl-dev cargo wasm-pack make pkgconf dbus-dev && \
    rm /etc/apk/cache
WORKDIR /app
COPY ./ /app
RUN --mount=type=cache,target=/var/cache/buildkit \
    CARGO_HOME=/var/cache/buildkit/cargo \
    CARGO_TARGET_DIR=/var/cache/buildkit/target-wasm \
    make wasm && \
    CARGO_HOME=/var/cache/buildkit/cargo \
    CARGO_TARGET_DIR=/var/cache/buildkit/target \
    make binaries && \
    cp -v /var/cache/buildkit/target/release/d3xs* .
RUN strip d3xs d3xs-bridge

FROM alpine:3.18
RUN apk add libgcc dbus-libs
COPY --from=0 /app/d3xs /app/d3xs-bridge /usr/bin
USER 1000
ENV D3XS_BIND=0.0.0.0:8080
ENTRYPOINT ["d3xs"]
