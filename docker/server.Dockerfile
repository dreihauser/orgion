# syntax=docker/dockerfile:1
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY apps/server ./apps/server
RUN cargo build --release -p orgion-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 orgion
COPY --from=builder /app/target/release/orgion /usr/local/bin/orgion
COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh && mkdir -p /data && chown orgion:orgion /data
USER orgion
WORKDIR /home/orgion
EXPOSE 3030
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
