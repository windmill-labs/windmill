FROM python:3.10-slim-buster as nsjail

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

RUN cargo install cargo-chef 

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

RUN cargo chef prepare  --recipe-path recipe.json

FROM rust_base AS builder

COPY --from=planner /windmill/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY ./openflow.openapi.yaml /openflow.openapi.yaml
COPY ./backend ./

COPY --from=frontend /frontend /frontend
COPY .git/ .git/

RUN cargo build --release


FROM debian:buster-slim
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq5 \
    make build-essential libssl-dev zlib1g-dev libbz2-dev libreadline-dev \
    libsqlite3-dev wget curl llvm libncurses5-dev libncursesw5-dev xz-utils tk-dev libxml2-dev \
    libxmlsec1-dev libffi-dev liblzma-dev mecab-ipadic-utf8 libgdbm-dev libc6-dev git libprotobuf-dev=3.6.* libnl-route-3-dev=3.4.* \
    libv8-dev tesseract-ocr \
    && rm -rf /var/lib/apt/lists/*

RUN wget https://golang.org/dl/go1.19.1.linux-amd64.tar.gz && tar -C /usr/local -xzf go1.19.1.linux-amd64.tar.gz
ENV PATH="${PATH}:/usr/local/go/bin"
ENV GO_PATH=/usr/local/go/bin/go

ENV TZ=Etc/UTC

ENV PYTHON_VERSION 3.10.4

RUN wget https://www.python.org/ftp/python/${PYTHON_VERSION}/Python-${PYTHON_VERSION}.tgz \
    && tar -xf Python-${PYTHON_VERSION}.tgz && cd Python-${PYTHON_VERSION}/ && ./configure --enable-optimizations \
    && make -j 4 && make install

RUN /usr/local/bin/python3 -m pip install pip-tools
RUN /usr/local/bin/python3 -m pip install nltk
RUN mkdir -p /nsjail_data/python && HOME=/nsjail_data/python /usr/local/bin/python3 -m nltk.downloader vader_lexicon

COPY --from=builder /windmill/target/release/windmill ${APP}/windmill

COPY --from=nsjail /nsjail/nsjail /bin/nsjail

COPY --from=denoland/deno:latest /usr/bin/deno /usr/bin/deno

RUN mkdir -p ${APP}

WORKDIR ${APP}

EXPOSE 8000

CMD ["./windmill"]
