FROM rust:1.40 as builder

WORKDIR /usr/src/rss-watcher
COPY . .

RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/rss-watcher /usr/local/bin/rss-watcher
ENV RUST_LOG=info

CMD ["rss-watcher"]
