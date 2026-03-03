FROM rust:1 AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release && strip target/release/rubyfast

# -----------------------------------------------------------
FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=builder /build/target/release/rubyfast /usr/local/bin/rubyfast

WORKDIR /workspace

ENTRYPOINT ["rubyfast"]
