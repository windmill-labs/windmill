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

    // Check for remote_url cookie
    const cookies = request.headers.get("cookie") || "";
    const remoteUrlMatch = cookies.match(/remote_url=([^;]+)/);
    const targetHostname = remoteUrlMatch
      ? decodeURIComponent(remoteUrlMatch[1])
      : "app.windmill.dev";

    url.hostname = targetHostname;
    const res = await fetch(url.toString(), {
      method: request.method,
      body: request.body,
      headers: request.headers,
      redirect: "manual",
    });
    const newResponse = new Response(res.body, res);

    // Each cookie is passed on by itself, without its Domain: the backend's (e.g.
    // `windmill.dev`) does not match this host, and a browser drops a cookie whose Domain
    // does not match. Reading them joined (`headers.get`) would merge the session cookie with
    // the `wm_session` hint set beside it, Domain included.
    // Pages runs without a compatibility date, where `getSetCookie` does not exist; its
    // `getAll` returns the same list there, and only for `set-cookie`.
    const setCookies: string[] | undefined =
      typeof res.headers.getSetCookie === "function"
        ? res.headers.getSetCookie()
        : typeof res.headers.getAll === "function"
          ? res.headers.getAll("set-cookie")
          : undefined;
    if (setCookies) {
      newResponse.headers.delete("set-cookie");
      for (const c of setCookies) {
        newResponse.headers.append("set-cookie", c.replace(/;\s*Domain=[^;]*/gi, ""));
      }
    }

    newResponse.headers.set("Cross-Origin-Resource-Policy", "cross-origin");
    return newResponse;
  } catch (e) {
    return new Response(e.message, { status: 500 });
  }
}
