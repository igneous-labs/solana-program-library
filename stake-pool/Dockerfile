# stake-pool program build environment

FROM rust:1.54-slim-buster

RUN apt-get update -y && \
    apt-get install wget bzip2 -y && \
    mkdir -p /solana && \
    cd /solana && \
    wget https://github.com/solana-labs/solana/releases/download/v1.7.14/solana-release-x86_64-unknown-linux-gnu.tar.bz2 && \
    tar jxf solana-release-x86_64-unknown-linux-gnu.tar.bz2
    
ENV PATH="/solana/solana-release/bin:${PATH}"

# first run of cargo build-bpf needs to download ~200MB of required solana toolchain packages,
# recommend running this as a long running container