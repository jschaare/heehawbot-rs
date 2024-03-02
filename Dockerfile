FROM rust:1.74-alpine as builder-base
RUN apk add --update \
    make \
    cmake \
    libressl-dev \
    musl-dev
WORKDIR /

FROM builder-base as builder
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

FROM alpine:3.18 as runner
RUN apk add --no-cache python3 xz curl ffmpeg
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp
COPY --from=builder /usr/local/cargo/bin/heehawbot /usr/local/bin/heehawbot
CMD ["heehawbot"]