FROM rust:latest

WORKDIR /usr/src/streamlink-rs
COPY . .

RUN cargo install --path .

CMD ["streamlink-rs"]
