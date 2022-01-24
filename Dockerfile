FROM rust:1.58 as builder

WORKDIR /usr/src/rss-watcher
COPY . .

RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/rss-watcher /usr/local/bin/rss-watcher
ENV RUST_LOG=info

CMD ["rss-watcher"]
