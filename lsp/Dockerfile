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
    targz='go1.22.5.linux-amd64.tar.gz'; \
    ;; \
    'arm64') \
    targz='go1.22.5.linux-arm64.tar.gz'; \
    ;; \
    'armhf') \
    targz='go1.22.5.linux-armv6l.tar.gz'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    wget "https://golang.org/dl/$targz" -nv && tar -C /usr/local -xzf "$targz" && rm "$targz";

ENV PATH="${PATH}:/usr/local/go/bin"
ENV GOBIN=/usr/local/go/bin
RUN /usr/local/go/bin/go install -v golang.org/x/tools/gopls@latest
RUN pip3 install tornado python-lsp-jsonrpc ruff-lsp

COPY --from=denoland/deno:2.0.2 --chmod=755 /usr/bin/deno /usr/bin/deno

COPY Pipfile .

RUN cat Pipfile

RUN pip install Cython

RUN pipenv install

COPY pyls_launcher.py .

RUN mkdir -p /tmp/monaco && chmod -R 777 /tmp/monaco

RUN cd /tmp/monaco && yarn add -D windmill-client

EXPOSE 3001

CMD ["python3" ,"pyls_launcher.py"]

