FROM python:3.12-slim as python-base
FROM node:22-slim as node-base

FROM python-base
COPY --from=node-base /usr/local /usr/local

ENV PATH="/usr/local/bin:${PATH}"
ENV PIPENV_VENV_IN_PROJECT=1
ENV XDG_CACHE_HOME=/pyls/.cache

RUN apt-get update \
    && apt-get install -y shellcheck wget git ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && pip install pipenv

RUN npm install -g diagnostic-languageserver pyright

RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    url=; \
    case "$arch" in \
    'amd64') \
    targz='go1.26.0.linux-amd64.tar.gz'; \
    ;; \
    'arm64') \
    targz='go1.26.0.linux-arm64.tar.gz'; \
    ;; \
    'armhf') \
    targz='go1.26.0.linux-armv6l.tar.gz'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    wget "https://golang.org/dl/$targz" -nv && tar -C /usr/local -xzf "$targz" && rm "$targz";

ENV PATH="${PATH}:/usr/local/go/bin"
ENV GOBIN=/usr/local/go/bin
RUN /usr/local/go/bin/go install -v golang.org/x/tools/gopls@latest

RUN pip3 install tornado python-lsp-jsonrpc ruff

COPY --from=denoland/deno:2.2.1 --chmod=755 /usr/bin/deno /usr/bin/deno

RUN mkdir -p /pyls/.cache

WORKDIR /pyls
COPY Pipfile .
RUN cat Pipfile
RUN pip install Cython
RUN pipenv install

COPY pyls_launcher.py .

RUN mkdir -p /tmp/monaco && chmod -R 777 /tmp/monaco
RUN cd /tmp/monaco && npm install --save-dev windmill-client

RUN chmod -R a+rX /usr/local && \
    chmod -R a+rX /pyls

EXPOSE 3001

CMD ["sh", "-c", "if [ -n \"$NETRC\" ]; then echo \"$NETRC\" > /root/.netrc && chmod 600 /root/.netrc; fi && if [ -d /root/.cache ]; then export XDG_CACHE_HOME=/root/.cache && cp -r /pyls/.cache /root/.cache; fi && python3 pyls_launcher.py"]
