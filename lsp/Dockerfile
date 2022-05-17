FROM nikolaik/python-nodejs

RUN yarn global add diagnostic-languageserver
RUN yarn global add pyright
RUN pip3 install black tornado python-lsp-jsonrpc
COPY --from=denoland/deno:latest /usr/bin/deno /usr/bin/deno

COPY Pipfile .

RUN cat Pipfile

RUN pipenv install

COPY pyls_launcher.py .

CMD ["python3" ,"pyls_launcher.py"]

