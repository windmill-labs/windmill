FROM nikolaik/python-nodejs:python3.11-nodejs19-slim

RUN yarn global add diagnostic-languageserver
RUN yarn global add pyright
RUN wget https://golang.org/dl/go1.19.3.linux-amd64.tar.gz && tar -C /usr/local -xzf go1.19.3.linux-amd64.tar.gz
ENV PATH="${PATH}:/usr/local/go/bin:/root/go/bin"
ENV GO_PATH=/usr/local/go/bin/go
ENV GOROOT=/usr/local/go
RUN /usr/local/go/bin/go install -v golang.org/x/tools/gopls@latest
RUN pip3 install black tornado python-lsp-jsonrpc
COPY --from=denoland/deno:latest /usr/bin/deno /usr/bin/deno

COPY Pipfile .

RUN cat Pipfile

RUN pip install Cython

RUN pipenv install

COPY pyls_launcher.py .

CMD ["python3" ,"pyls_launcher.py"]

