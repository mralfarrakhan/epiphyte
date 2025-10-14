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

RUN rustup target add i686-pc-windows-gnu

ENV CARGO_BUILD_TARGET=i686-pc-windows-gnu

WORKDIR /workspace

CMD ["/bin/bash"]
