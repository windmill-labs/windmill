FROM debian:buster-slim as downloader

ARG TARGETPLATFORM

SHELL ["/bin/bash", "-c"]

RUN apt update -y
RUN apt install -y unzip curl

RUN [ "$TARGETPLATFORM" == "linux/arm64" ] && curl -Lsf https://github.com/LukeChannings/deno-arm64/releases/download/v1.40.2/deno-linux-arm64.zip -o deno.zip || true
RUN [ "$TARGETPLATFORM" == "linux/amd64" ] && curl -Lsf https://github.com/denoland/deno/releases/download/v1.40.2/deno-x86_64-unknown-linux-gnu.zip -o deno.zip || true

RUN unzip deno.zip && rm deno.zip


FROM nikolaik/python-nodejs:python3.11-nodejs19-slim

RUN apt-get update \
    && apt-get install -y shellcheck
RUN yarn global add diagnostic-languageserver
RUN yarn global add pyright
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    url=; \
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
ENV GOBIN=/usr/local/go/bin
RUN /usr/local/go/bin/go install -v golang.org/x/tools/gopls@latest
RUN pip3 install black tornado python-lsp-jsonrpc ruff-lsp

COPY --from=downloader /deno /usr/bin/deno
RUN chmod 755 /usr/bin/deno

COPY Pipfile .

RUN cat Pipfile

RUN pip install Cython

RUN pipenv install

COPY pyls_launcher.py .

RUN mkdir -p /tmp/monaco && chmod -R 777 /tmp/monaco

RUN cd /tmp/monaco && yarn add -D windmill-client

EXPOSE 3001

CMD ["python3" ,"pyls_launcher.py"]

