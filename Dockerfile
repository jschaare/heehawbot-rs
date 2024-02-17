FROM rust:1.74-alpine as builder-base
RUN apk add --update \
    make \
    cmake \
    libressl-dev \
    musl-dev
WORKDIR /

FROM builder-base as builder
WORKDIR /usr/src/heehawbot
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN mkdir src && \
    echo "// cache build" > src/lib.rs && \
    cargo build --release --locked && \
    rm -r src
COPY ./src ./src
RUN cargo install --locked --path .
WORKDIR /

FROM alpine:3.18 as runner
RUN apk add --no-cache python3 xz curl ffmpeg
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp
COPY --from=builder /usr/local/cargo/bin/heehawbot /usr/local/bin/heehawbot
CMD ["heehawbot"]