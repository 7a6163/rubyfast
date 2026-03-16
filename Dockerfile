FROM rust:1 AS builder

RUN apt-get update && apt-get install -y clang libclang-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release && strip target/release/rubyfast

# -----------------------------------------------------------
FROM gcr.io/distroless/cc-debian13:debug

COPY --from=builder /build/target/release/rubyfast /usr/local/bin/rubyfast

WORKDIR /workspace

ENTRYPOINT ["rubyfast"]
