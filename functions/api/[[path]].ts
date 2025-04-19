//this is purely for cloudflare pages redirection purposes

export async function onRequest(context) {
  // Contents of context object
  const {
    request, // same as existing Worker API
    env, // same as existing Worker API
    params, // if filename includes [id] or [[path]]
    waitUntil, // same as ctx.waitUntil in existing Worker API
    next, // used for middleware or to fetch assets
    data, // arbitrary space for passing data between middlewares
  } = context;

  try {
    const url = new URL(request.url);
    url.hostname = "app.windmill.dev";
    const res = await fetch(url.toString(), {
      method: request.method,
      body: request.body,
      headers: request.headers,
      redirect: "manual",
    });
    const newResponse = new Response(res.body, res);
    newResponse.headers.set(
      "set-cookie",
      "token" +
        newResponse.headers
          .get("set-cookie")
          ?.replace("Domain=windmill.dev;", "")
          ?.split("token")
          .pop() ?? ""
    );

    newResponse.headers.set("Cross-Origin-Resource-Policy", "cross-origin");
    newResponse.headers.set("Cross-Origin-Opener-Policy", "same-origin");
    newResponse.headers.set("Cross-Origin-Embedder-Policy", "require-corp");
    return newResponse;
  } catch (e) {
    return new Response(e.message, { status: 500 });
  }
}
