FROM rust:slim as builder
WORKDIR /usr/src/holo-wtf-api
COPY . .
RUN apt-get update && apt-get -y upgrade && apt-get -y install pkg-config openssl libssl-dev
RUN cargo build --release
CMD ["cargo", "run", "--release"]

FROM debian:bullseye-slim as runner
COPY --from=builder /usr/src/holo-wtf-api/target/release/holo-wtf-api /usr/local/bin/holo-wtf-api
COPY --from=builder /usr/src/holo-wtf-api/Rocket.toml /usr/local/bin/Rocket.toml
WORKDIR "/usr/local/bin"
CMD ["./holo-wtf-api"]
