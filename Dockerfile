FROM rust:1.96-alpine AS builder-base
RUN apk add --update --no-cache \
    alpine-sdk \
    pkgconf \
    cmake \
    openssl-dev \
    openssl-libs-static \
    musl-dev
WORKDIR /

FROM builder-base AS builder
WORKDIR /usr/src/heehawbot
# add actual project dependencies
COPY ./Cargo.lock ./Cargo.toml ./
# build dummy project to cache dependencies and speed up builds
RUN mkdir src && \
    echo "fn main() {println!(\"should never see this...\")}" > src/main.rs && \
    cargo build --release --locked
RUN rm -f target/release/deps/heehawbot*
# build actual project, should be faster if dependencies didn't change
COPY ./src ./src
RUN cargo install --locked --path .
WORKDIR /

FROM alpine:latest AS runner
RUN apk add --update --no-cache \
    ffmpeg \
    python3 \
    py3-pip \
    libgcc \
    ca-certificates
RUN pip install --break-system-packages yt-dlp
COPY --from=builder /usr/local/cargo/bin/heehawbot /usr/local/bin/heehawbot
CMD ["heehawbot"]
