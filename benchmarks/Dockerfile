FROM denoland/deno:alpine-2.1.4

WORKDIR /app

USER deno

ADD ./lib.ts .
ADD ./action.ts .
ADD ./main.ts .
RUN deno cache main.ts
ADD ./worker.ts .
RUN deno cache worker.ts
ADD ./scraper.ts .
RUN deno cache scraper.ts
ADD ./benchmark_oneoff.ts .
RUN deno cache benchmark_oneoff.ts
ADD ./benchmark_suite.ts .
RUN deno cache benchmark_suite.ts

ENTRYPOINT [ "/tini", "--", "docker-entrypoint.sh", "run", "-A", "main.ts" ]
