FROM debian:buster-slim as nsjail

WORKDIR /nsjail

ARG nsjail=""

RUN  if [ "$nsjail" = "true" ]; then apt-get -y update \
    && apt-get install -y \
    bison=2:3.3.* \
    flex=2.6.* \
    g++=4:8.3.* \
    gcc=4:8.3.* \
    git=1:2.20.* \
    libprotobuf-dev=3.6.* \
    libnl-route-3-dev=3.4.* \
    make=4.2.* \
    pkg-config=0.29-6 \
    protobuf-compiler=3.6.*; fi


RUN if [ "$nsjail" = "true" ]; then git clone -b master --single-branch https://github.com/google/nsjail.git . \
    && git checkout dccf911fd2659e7b08ce9507c25b2b38ec2c5800; fi
RUN if [ "$nsjail" = "true" ]; then make; else touch nsjail; fi

FROM rust:slim-buster AS rust_base

RUN apt-get update && apt-get install -y git libssl-dev pkg-config npm

RUN apt-get -y update \
    && apt-get install -y \
    curl nodejs npm

RUN rustup component add rustfmt

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install cargo-chef 

WORKDIR /windmill

ENV SQLX_OFFLINE=true
ENV CARGO_INCREMENTAL=1

FROM node:20-alpine as frontend

# install dependencies
WORKDIR /frontend
COPY ./frontend/package.json ./frontend/package-lock.json ./
RUN npm ci

# Copy all local files into the image.
COPY frontend .
RUN mkdir /backend
COPY /backend/windmill-api/openapi.yaml /backend/windmill-api/openapi.yaml
COPY /openflow.openapi.yaml /openflow.openapi.yaml
COPY /backend/windmill-api/build_openapi.sh /backend/windmill-api/build_openapi.sh

RUN cd /backend/windmill-api && . ./build_openapi.sh
COPY /backend/parsers/windmill-parser-wasm/pkg/ /backend/parsers/windmill-parser-wasm/pkg/

RUN npm run generate-backend-client
ENV NODE_OPTIONS "--max-old-space-size=8192"
RUN npm run build


FROM rust_base AS planner

COPY ./openflow.openapi.yaml /openflow.openapi.yaml
COPY ./backend ./

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo chef prepare --recipe-path recipe.json

FROM rust_base AS builder
ARG features=""

COPY --from=planner /windmill/recipe.json recipe.json

RUN apt-get update && apt-get install -y libxml2-dev libxmlsec1-dev clang libclang-dev cmake

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true RUST_BACKTRACE=1 cargo chef cook --release --features "$features" --recipe-path recipe.json

COPY ./openflow.openapi.yaml /openflow.openapi.yaml
COPY ./backend ./

COPY --from=frontend /frontend /frontend
COPY --from=frontend /backend/windmill-api/openapi-deref.yaml ./windmill-api/openapi-deref.yaml
COPY .git/ .git/

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release --features "$features"


FROM debian:buster-slim as downloader

ARG TARGETPLATFORM

SHELL ["/bin/bash", "-c"]

RUN apt update -y
RUN apt install -y unzip curl

RUN [ "$TARGETPLATFORM" == "linux/arm64" ] && curl -Lsf https://github.com/LukeChannings/deno-arm64/releases/download/v1.36.3/deno-linux-arm64.zip -o deno.zip || true
RUN [ "$TARGETPLATFORM" == "linux/amd64" ] && curl -Lsf https://github.com/denoland/deno/releases/download/v1.36.3/deno-x86_64-unknown-linux-gnu.zip -o deno.zip || true

RUN unzip deno.zip && rm deno.zip

FROM python:3.11.4-slim-buster

ARG TARGETPLATFORM

ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates wget curl git jq libprotobuf-dev libnl-route-3-dev unzip build-essential unixodbc xmlsec1 \
    && rm -rf /var/lib/apt/lists/*

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then apt-get update -y && apt install libicu-dev -y && wget -O 'pwsh.deb' 'https://github.com/PowerShell/PowerShell/releases/download/v7.3.5/powershell_7.3.5-1.deb_amd64.deb' && \
    dpkg --install 'pwsh.deb' && \
    rm 'pwsh.deb'; else echo 'pwshell not on amd64'; fi

RUN arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    wget https://get.helm.sh/helm-v3.12.0-linux-$arch.tar.gz && \
    tar -zxvf helm-v3.12.0-linux-$arch.tar.gz  && \
    mv linux-$arch/helm /usr/local/bin/helm &&\
    chmod +x /usr/local/bin/helm

RUN arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    curl -LO "https://dl.k8s.io/release/v1.27.2/bin/linux/$arch/kubectl" && \
    install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    case "$arch" in \
    'amd64') \
    zip='awscli-exe-linux-x86_64.zip'; \
    ;; \
    'arm64') \
    zip='awscli-exe-linux-aarch64.zip'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    apt-get update && apt install unzip && curl "https://awscli.amazonaws.com/$zip" -o "awscliv2.zip" && \
    unzip awscliv2.zip && \
    ./aws/install && rm awscliv2.zip

RUN arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    curl -o rclone.zip "https://downloads.rclone.org/v1.60.1/rclone-v1.60.1-linux-$arch.zip"; \
    unzip -p rclone.zip rclone-v1.60.1-linux-$arch/rclone > /usr/bin/rclone; rm rclone.zip; \
    chown root:root /usr/bin/rclone; chmod 755 /usr/bin/rclone

RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    case "$arch" in \
    'amd64') \
    targz='go1.21.0.linux-amd64.tar.gz'; \
    ;; \
    'arm64') \
    targz='go1.21.0.linux-arm64.tar.gz'; \
    ;; \
    'armhf') \
    targz='go1.21.0.linux-armv6l.tar.gz'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    wget "https://golang.org/dl/$targz" -nv && tar -C /usr/local -xzf "$targz" && rm "$targz";

ENV PATH="${PATH}:/usr/local/go/bin"
ENV GO_PATH=/usr/local/go/bin/go

# go build is slower the first time it is ran, so we prewarm it in the build
RUN mkdir -p /tmp/gobuildwarm && cd /tmp/gobuildwarm && go mod init gobuildwarm &&  printf "package foo\nimport (\"fmt\")\nfunc main() { fmt.Println(42) }" > warm.go && go build -x && rm -rf /tmp/gobuildwarm

ENV TZ=Etc/UTC

RUN /usr/local/bin/python3 -m pip install pip-tools

COPY --from=frontend /frontend/build /static_frontend
COPY --from=builder /windmill/target/release/windmill ${APP}/windmill


COPY --from=downloader /deno /usr/bin/deno
RUN chmod 755 /usr/bin/deno

COPY --from=nsjail /nsjail/nsjail /bin/nsjail

COPY --from=oven/bun:1.0.0 /usr/local/bin/bun /usr/bin/bun

# add the docker client to call docker from a worker if enabled
COPY --from=docker:dind /usr/local/bin/docker /usr/local/bin/

RUN mkdir -p ${APP}

RUN ln -s ${APP}/windmill /usr/local/bin/windmill

WORKDIR ${APP}

EXPOSE 8000

CMD ["windmill"]
