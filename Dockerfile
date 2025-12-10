FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y curl
RUN curl  -H "Range: bytes=0-0" -L -v https://huggingface.co/thenlper/gte-small/resolve/main/tokenizer.json