# syntax=docker.io/docker/dockerfile:1
FROM golang:1.21 AS kubo-build

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends llvm
RUN git clone https://github.com/zippiehq/cartesi-kubo -b ipfs-cartesi kubo && cd kubo && git checkout a9042bef91cf09f140bbf38034dca486d752d3f8

WORKDIR /app/kubo
RUN go mod download

COPY ./sys_linux_riscv64.go \
    /go/pkg/mod/github.com/marten-seemann/tcp@v0.0.0-20210406111302-dfbc87cc63fd/sys_linux_riscv64.go

RUN GOOS=linux GOARCH=riscv64 CGO_ENABLED=0 make nofuse

RUN llvm-strip -s /app/kubo/cmd/ipfs/ipfs


FROM ubuntu:22.04 AS builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.82.0

ARG DEBIAN_FRONTEND=noninteractive
RUN <<EOF
set -e
apt update
apt install -y --no-install-recommends \
    build-essential=12.9ubuntu3 \
    ca-certificates=20240203~22.04.1 \
    g++-riscv64-linux-gnu=4:11.2.0--1ubuntu1 \
    wget=1.21.2-2ubuntu1
EOF

RUN set -eux; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
    amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db' ;; \
    armhf) rustArch='armv7-unknown-linux-gnueabihf'; rustupSha256='f21c44b01678c645d8fbba1e55e4180a01ac5af2d38bcbd14aa665e0d96ed69a' ;; \
    arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800' ;; \
    i386) rustArch='i686-unknown-linux-gnu'; rustupSha256='e7b0f47557c1afcd86939b118cbcf7fb95a5d1d917bdd355157b63ca00fc4333' ;; \
    *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    url="https://static.rust-lang.org/rustup/archive/1.26.0/${rustArch}/rustup-init"; \
    wget "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

RUN rustup target add riscv64gc-unknown-linux-gnu

WORKDIR /opt/cartesi/dapp
COPY . .
RUN rm -f Cargo.lock && cargo generate-lockfile
RUN cargo build --release

RUN chmod +x /opt/cartesi/dapp/target/riscv64gc-unknown-linux-gnu/release/dapp

FROM --platform=linux/riscv64 ubuntu:22.04

LABEL io.cartesi.rollups.sdk_version=0.11.1
LABEL io.cartesi.rollups.ram_size=128Mi

ARG DEBIAN_FRONTEND=noninteractive
RUN <<EOF
set -e
apt-get update
apt-get install -y --no-install-recommends \
    busybox-static=1:1.30.1-7ubuntu3 \
    netcat-traditional \
    ca-certificates
rm -rf /var/lib/apt/lists/* /var/log/* /var/cache/*
useradd --create-home --user-group dapp
EOF

ARG MACHINE_EMULATOR_TOOLS_VERSION=0.16.2-coprocessor2
ADD https://github.com/zippiehq/cartesi-coprocessor-emulator-tools/releases/download/v${MACHINE_EMULATOR_TOOLS_VERSION}/machine-emulator-tools-v${MACHINE_EMULATOR_TOOLS_VERSION}.deb /
RUN dpkg -i /machine-emulator-tools-v${MACHINE_EMULATOR_TOOLS_VERSION}.deb \
  && rm /machine-emulator-tools-v${MACHINE_EMULATOR_TOOLS_VERSION}.deb
  
ENV PATH="/opt/cartesi/bin:/opt/cartesi/dapp:${PATH}"

WORKDIR /opt/cartesi/dapp
COPY --from=kubo-build /app/kubo/cmd/ipfs/ipfs /usr/bin/ipfs
RUN chmod +x /usr/bin/ipfs
COPY --from=builder /opt/cartesi/dapp/target/riscv64gc-unknown-linux-gnu/release/dapp .
COPY ./startup.sh /startup.sh
RUN chmod +x /startup.sh

ENV ROLLUP_HTTP_SERVER_URL="http://127.0.0.1:5004"

ENTRYPOINT ["/startup.sh"]
