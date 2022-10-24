FROM denoland/deno:alpine-1.26.2

WORKDIR /app

USER deno

ADD ./main.ts .
RUN deno cache --unstable main.ts
ADD ./worker.ts .
RUN deno cache --unstable worker.ts
ADD ./scraper.ts .
RUN deno cache --unstable scraper.ts

ENTRYPOINT [ "/tini", "--", "docker-entrypoint.sh", "run", "--unstable", "-A", "main.ts" ]