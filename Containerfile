FROM docker.io/rust:1.84-bullseye as build

WORKDIR /src
COPY . .
RUN cargo build --release

FROM ubuntu:22.04
COPY --from=build /src/target/release/umq /usr/local/bin
