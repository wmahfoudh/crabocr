FROM rust:alpine as builder

RUN apk add --no-cache musl-dev gcc g++ make cmake clang llvm-dev libstdc++

WORKDIR /app
COPY . .

# Build for musl
RUN cargo build --release --target x86_64-unknown-linux-musl

# Final stage empty? Or scratch?
# For now, just builder is fine to extract artifact.
