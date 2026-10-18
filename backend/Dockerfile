FROM rust:1.82-alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:3.20

RUN apk add --no-cache ca-certificates libressl

WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend /app/backend

EXPOSE 8080 4000

CMD ["/app/backend"]
