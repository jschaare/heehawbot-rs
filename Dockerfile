FROM rust:1.96-alpine AS builder-base
RUN apk add --update --no-cache \
    build-base \
    pkgconf \
    cmake \
    musl-dev
WORKDIR /

FROM builder-base AS builder
WORKDIR /usr/src/heehawbot
COPY ./Cargo.lock ./Cargo.toml ./
COPY ./src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/heehawbot/target \
    cargo build --release --locked && \
    cp target/release/heehawbot /usr/local/bin/heehawbot
WORKDIR /

FROM alpine:latest AS runner
RUN apk add --update --no-cache \
    ffmpeg \
    python3 \
    libgcc \
    ca-certificates
ADD --chmod=755 https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp /usr/local/bin/yt-dlp
COPY --from=builder /usr/local/bin/heehawbot /usr/local/bin/heehawbot
CMD ["heehawbot"]
