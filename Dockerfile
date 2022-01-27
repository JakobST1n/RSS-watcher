FROM rust:1.58 as builder

# Install and configure dependencies needed for building for musl target
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

# Add source code to image
WORKDIR /usr/src/rss-watcher
COPY . .

# Build
RUN cargo build --target x86_64-unknown-linux-musl --release

# Move to a smaller image
FROM scratch

# Copy binary from builder
COPY --from=builder /usr/src/rss-watcher/target/x86_64-unknown-linux-musl/release/rss-watcher /rss-watcher

# Add log level info, if we don't do this, no logs will be written
ENV RUST_LOG=info

# Start target
CMD ["/rss-watcher"]
