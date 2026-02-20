FROM rust:latest

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    gcc-mingw-w64-i686 \
    pkg-config \
    make \
    cmake \
    git \
    openssh-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

ENV CARGO_BUILD_TARGET=i686-pc-windows-gnu

RUN rustup target add i686-pc-windows-gnu \
    --toolchain nightly-2025-08-11

RUN rustup toolchain install nightly-2025-08-11 \
    --component clippy \
    --component rustfmt

CMD ["/bin/bash"]