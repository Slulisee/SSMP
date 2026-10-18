FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release --locked --no-default-features

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 10001 --create-home ssmp \
    && mkdir -p /data /music && chown ssmp:ssmp /data /music
COPY --from=builder /src/target/release/ssmp-desktop /usr/local/bin/SSMP
USER 10001:10001
ENV SSMP_DATA_DIR=/data
VOLUME ["/data"]
EXPOSE 4534
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s CMD curl -fsS http://127.0.0.1:4534/api/v1/health || exit 1
ENTRYPOINT ["SSMP"]
CMD ["--watch", "/music", "--serve", "--bind", "0.0.0.0:4534"]
