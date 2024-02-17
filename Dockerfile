FROM rust:1.74.0 as builder-base
RUN apt update &&  \
    apt install -y curl libssl-dev libopus-dev && \
    rm -rf /var/lib/apt/lists/*
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp
WORKDIR /

FROM builder-base AS builder
WORKDIR /usr/src/heehawbot
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN mkdir src && \
    echo "// cache build" > src/lib.rs && \
    cargo build --release --locked && \
    rm -r src
COPY ./src ./src
RUN cargo install --locked --path .
# RUN cargo build --release --locked
WORKDIR /

FROM debian:bookworm-slim as runner
RUN apt-get update &&  \
    apt-get install -y libssl3 libopus-dev python3 python3-pip && \
    rm -rf /var/lib/apt/lists/*
RUN python3 -m pip install -U yt-dlp --break-system-packages
# COPY --from=builder /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
COPY --from=builder /usr/local/cargo/bin/heehawbot /usr/local/bin/heehawbot
CMD ["heehawbot"]