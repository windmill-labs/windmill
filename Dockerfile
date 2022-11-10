FROM debian:buster-slim as nsjail

WORKDIR /nsjail

RUN apt-get -y update \
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
    protobuf-compiler=3.6.*

RUN git clone -b master --single-branch https://github.com/google/nsjail.git . \
    && git checkout dccf911fd2659e7b08ce9507c25b2b38ec2c5800
RUN make

FROM rust:slim-buster AS rust_base

RUN apt-get update && apt-get install -y git libssl-dev pkg-config npm

RUN apt-get -y update \
    && apt-get install -y \
    curl lld nodejs npm

RUN rustup component add rustfmt

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install cargo-chef 

WORKDIR /windmill

ENV SQLX_OFFLINE=true
ENV CARGO_INCREMENTAL=1

FROM node:19-alpine as frontend

# install dependencies
WORKDIR /frontend
COPY ./frontend/package.json ./frontend/package-lock.json ./
RUN npm ci

# Copy all local files into the image.
COPY frontend .
RUN mkdir /backend
COPY /backend/windmill-api/openapi.yaml /backend/windmill-api/openapi.yaml
COPY /openflow.openapi.yaml /openflow.openapi.yaml
RUN npm run generate-backend-client
ENV NODE_OPTIONS "--max-old-space-size=8192"
RUN npm run build
RUN npm run check


FROM rust_base AS planner

COPY ./openflow.openapi.yaml /openflow.openapi.yaml
COPY ./backend ./

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo chef prepare  --recipe-path recipe.json

FROM rust_base AS builder

COPY --from=planner /windmill/recipe.json recipe.json

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo chef cook --release --recipe-path recipe.json

COPY ./openflow.openapi.yaml /openflow.openapi.yaml
COPY ./backend ./

COPY --from=frontend /frontend /frontend
COPY .git/ .git/

RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release


FROM python:3.11.0-slim-buster

ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates wget curl git jq libprotobuf-dev libnl-route-3-dev \
    && rm -rf /var/lib/apt/lists/*

RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    url=; \
    case "$arch" in \
    'amd64') \
    targz='go1.19.3.linux-amd64.tar.gz'; \
    sha256='74b9640724fd4e6bb0ed2a1bc44ae813a03f1e72a4c76253e2d5c015494430ba'; \
    ;; \
    'arm64') \
    targz='go1.19.3.linux-arm64.tar.gz'; \
    sha256='99de2fe112a52ab748fb175edea64b313a0c8d51d6157dba683a6be163fd5eab'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    wget "https://golang.org/dl/$targz" && tar -C /usr/local -xzf "$targz" && rm "$targz";

ENV PATH="${PATH}:/usr/local/go/bin"
ENV GO_PATH=/usr/local/go/bin/go

ENV TZ=Etc/UTC

RUN /usr/local/bin/python3 -m pip install pip-tools

COPY --from=builder /windmill/target/release/windmill ${APP}/windmill

COPY --from=nsjail /nsjail/nsjail /bin/nsjail

COPY --from=denoland/deno:latest /usr/bin/deno /usr/bin/deno

RUN mkdir -p ${APP}

WORKDIR ${APP}

EXPOSE 8000

CMD ["./windmill"]
