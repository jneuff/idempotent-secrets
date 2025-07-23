FROM rust:latest AS builder
WORKDIR /app
COPY src /app/src
COPY Cargo.toml /app/Cargo.toml
COPY Cargo.lock /app/Cargo.lock
RUN cargo build --release

FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/idempotent-secrets /bin/idempotent-secrets
CMD ["/bin/idempotent-secrets"]
