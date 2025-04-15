FROM clux/muslrust:nightly-2025-02-24 AS builder

WORKDIR /app

COPY ./api ./api
COPY ./cache ./cache
COPY ./env ./env
COPY ./gcp ./gcp
COPY ./openai ./openai
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY toolchain.toml toolchain.toml
COPY README.md README.md
COPY LICENSE LICENSE

ENV TARGET=x86_64-unknown-linux-musl
RUN cargo build --release



FROM alpine:3 AS production

WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/api /app/tts-gateway

CMD ["./tts-gateway"]
