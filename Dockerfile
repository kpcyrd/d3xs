FROM rust:1-alpine3.18
ENV RUSTFLAGS="-C target-feature=-crt-static"
RUN apk add musl-dev
WORKDIR /app
COPY ./ /app
RUN --mount=type=cache,target=/var/cache/buildkit \
    CARGO_HOME=/var/cache/buildkit/cargo \
    CARGO_TARGET_DIR=/var/cache/buildkit/target \
    cargo build --release --locked && \
    cp -v /var/cache/buildkit/target/release/d3xs .
RUN strip d3xs

FROM alpine:3.18
RUN apk add libgcc
COPY --from=0 /app/d3xs /usr/bin
USER 1000
ENV D3XS_CONFIG=/config.toml
ENV D3XS_BIND=0.0.0.0:8080
ENTRYPOINT ["d3xs"]
