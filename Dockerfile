FROM rust:1.58 as builder

WORKDIR /usr/src/rss-watcher
COPY . .

RUN cargo install --path .

#FROM debian:buster-slim
#RUN apt-get update \
#    && apt-get install -y libssl-dev libc-bin libc6 \
#    && rm -rf /var/lib/apt/lists/*
#COPY --from=builder /usr/local/cargo/bin/rss-watcher /usr/local/bin/rss-watcher
#ENV RUST_LOG=info

CMD ["rss-watcher"]
