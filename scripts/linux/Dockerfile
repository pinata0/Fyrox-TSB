FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config curl ca-certificates python3 \
    libasound2-dev libudev-dev libx11-dev libxi-dev libxcursor-dev \
    libxrandr-dev libxinerama-dev libgl1-mesa-dev libwayland-dev \
    libxkbcommon-dev xvfb xauth mesa-utils libgl1-mesa-dri \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain 1.95.0 \
    && rm /tmp/rustup-init.sh

RUN apt-get update && apt-get install -y --no-install-recommends libxkbcommon-x11-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
