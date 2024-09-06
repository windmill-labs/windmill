import { Application } from "../mod.ts";

const app = new Application();

app.use((ctx) => {
  ctx.response.body = {
    headers: [...ctx.request.headers],
    secure: ctx.request.secure,
    ip: ctx.request.ip,
    ips: ctx.request.ips,
    url: ctx.request.url.toString(),
  };
  ctx.response.type = "json";
});

app.listen({ port: 80 });
