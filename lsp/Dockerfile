FROM nikolaik/python-nodejs:python3.11-nodejs19-slim

RUN yarn global add diagnostic-languageserver
RUN yarn global add pyright
RUN set -eux; \
    arch="$(dpkg --print-architecture)"; arch="${arch##*-}"; \
    url=; \
    case "$arch" in \
    'amd64') \
    targz='go1.19.3.linux-amd64.tar.gz'; \
    ;; \
    'arm64') \
    targz='go1.19.3.linux-arm64.tar.gz'; \
    ;; \
    'armhf') \
    targz='go1.19.3.linux-armv6l.tar.gz'; \
    ;; \
    *) echo >&2 "error: unsupported architecture '$arch' (likely packaging update needed)"; exit 1 ;; \
    esac; \
    wget "https://golang.org/dl/$targz" -nv && tar -C /usr/local -xzf "$targz" && rm "$targz";

ENV PATH="${PATH}:/usr/local/go/bin"
ENV GO_PATH=/usr/local/go/bin/go
ENV GOROOT=/usr/local/go
RUN /usr/local/go/bin/go install -v golang.org/x/tools/gopls@latest
RUN pip3 install black tornado python-lsp-jsonrpc
COPY --from=denoland/deno:latest /usr/bin/deno /usr/bin/deno

# docker does not support conditional COPY and we want to use the same Dockerfile for both amd64 and arm64 and privilege the official image
COPY --from=lukechannings/deno:latest /usr/bin/deno /usr/bin/deno-arm
RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then rm /usr/bin/deno-arm; elif [ "$TARGETPLATFORM" = "linux/arm64" ]; then mv /usr/bin/deno-arm /usr/bin/deno; fi


COPY Pipfile .

RUN cat Pipfile

RUN pip install Cython

RUN pipenv install

COPY pyls_launcher.py .

EXPOSE 3001

CMD ["python3" ,"pyls_launcher.py"]

