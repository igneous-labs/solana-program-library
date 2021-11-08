FROM rust:latest
RUN apt-get update && apt-get install -y build-essential binutils-dev libunwind-dev libblocksruntime-dev liblzma-dev
RUN cargo install honggfuzz

COPY .. /spl
WORKDIR /spl

ENTRYPOINT [ "bash" ]