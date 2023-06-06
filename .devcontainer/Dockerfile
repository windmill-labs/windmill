FROM python:3.10-slim-bullseye as nsjail

WORKDIR /nsjail

RUN apt-get -y update \
    && apt-get install -y \
    bison \
    flex \
    g++ \
    gcc \
    git \
    libprotobuf-dev \
    libnl-route-3-dev \
    make \
    pkg-config \
    protobuf-compiler \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

RUN git clone -b master --single-branch https://github.com/google/nsjail.git . \
    && git checkout dccf911fd2659e7b08ce9507c25b2b38ec2c5800
RUN make

FROM mcr.microsoft.com/vscode/devcontainers/rust:bullseye as rust-deps

RUN cargo install sqlx-cli --no-default-features --features native-tls,postgres 
RUN cargo install deno --locked

FROM mcr.microsoft.com/vscode/devcontainers/rust:bullseye

RUN apt update \
    && apt-get install -y \
    python3 \
    libprotobuf-dev \ 
    libnl-route-3-dev \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

USER vscode

COPY --from=rust-deps /usr/local/cargo/bin/sqlx /usr/local/cargo/bin/sqlx
COPY --from=rust-deps /usr/local/cargo/bin/deno /usr/local/cargo/bin/deno
COPY --from=nsjail /nsjail/nsjail /bin/nsjail
