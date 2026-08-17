import { expect, test } from "bun:test";
import {
  AuthGatewayChallengeError,
  detectAuthGatewayChallenge,
} from "../src/utils/http_guards.ts";

const URL = "https://windmill.example.com/api/w/dev/scripts/create";

function htmlResponse(body: string, headers: Record<string, string> = {}) {
  return new Response(body, {
    status: 200,
    headers: { "content-type": "text/html; charset=utf-8", ...headers },
  });
}

test("throws on Cloudflare Access SSO HTML", async () => {
  const body =
    '<!DOCTYPE html><title>Sign in ・ Cloudflare Access</title><body>...</body>';
  const res = htmlResponse(body, {
    "cf-ray": "8a1234abcdef-ATL",
    "cf-mitigated": "challenge",
  });

  await expect(detectAuthGatewayChallenge(res, URL)).rejects.toBeInstanceOf(
    AuthGatewayChallengeError,
  );
});

test("error includes cf-ray, cf-mitigated, status, body snippet", async () => {
  const body = '<!DOCTYPE html><title>Sign in ・ Cloudflare Access</title>';
  const res = htmlResponse(body, {
    "cf-ray": "ray-123",
    "cf-mitigated": "challenge",
  });

  try {
    await detectAuthGatewayChallenge(res, URL);
    throw new Error("expected to throw");
  } catch (e) {
    expect(e).toBeInstanceOf(AuthGatewayChallengeError);
    const err = e as AuthGatewayChallengeError;
    expect(err.url).toBe(URL);
    expect(err.cfRay).toBe("ray-123");
    expect(err.cfMitigated).toBe("challenge");
    expect(err.status).toBe(200);
    expect(err.message).toContain("ray-123");
    expect(err.message).toContain("cf-mitigated=challenge");
    expect(err.message).toContain("Sign in");
  }
});

test("throws on generic HTML body even without cf headers", async () => {
  const body = "<!doctype html><html><body>Login required</body></html>";
  const res = htmlResponse(body);

  await expect(detectAuthGatewayChallenge(res, URL)).rejects.toBeInstanceOf(
    AuthGatewayChallengeError,
  );
});

test("throws on cf-mitigated=challenge even with non-HTML content type", async () => {
  // CF Access can challenge non-HTML responses too; the header alone is enough.
  const res = new Response('<title>Sign in ・ Cloudflare Access</title>', {
    status: 200,
    headers: {
      "content-type": "application/octet-stream",
      "cf-mitigated": "challenge",
    },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).rejects.toBeInstanceOf(
    AuthGatewayChallengeError,
  );
});

test("normalizes content-type case (Text/HTML)", async () => {
  const body = "<!doctype html><html></html>";
  const res = new Response(body, {
    status: 200,
    headers: { "content-type": "Text/HTML; charset=UTF-8" },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).rejects.toBeInstanceOf(
    AuthGatewayChallengeError,
  );
});

test("does NOT throw on JSON happy-path response", async () => {
  const res = new Response(JSON.stringify({ id: "abc" }), {
    status: 200,
    headers: { "content-type": "application/json" },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).resolves.toBeUndefined();
});

test("does NOT throw on cf-mitigated values other than 'challenge' (e.g. 'block') with non-HTML body", async () => {
  // If CF marks something with cf-mitigated: block on a non-HTML response, the
  // cheap path should skip the challenge predicate entirely.
  const res = new Response('{"error":"blocked"}', {
    status: 403,
    headers: {
      "content-type": "application/json",
      "cf-mitigated": "block",
    },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).resolves.toBeUndefined();
});

test("does NOT throw on text/plain body that isn't an auth challenge", async () => {
  const res = new Response("just some plaintext", {
    status: 200,
    headers: { "content-type": "text/plain" },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).resolves.toBeUndefined();
});

test("does NOT throw when body is empty", async () => {
  // Body absent (e.g. 204) — treat as non-challenge regardless of headers.
  const res = new Response(null, {
    status: 204,
    headers: { "content-type": "text/html" },
  });

  await expect(detectAuthGatewayChallenge(res, URL)).resolves.toBeUndefined();
});

test("falls back to response.url when caller url is omitted", async () => {
  const body = "<!doctype html><title>Sign in ・ Cloudflare Access</title>";
  const res = htmlResponse(body);

  try {
    await detectAuthGatewayChallenge(res);
    throw new Error("expected to throw");
  } catch (e) {
    const err = e as AuthGatewayChallengeError;
    // Response constructor doesn't set .url, so we expect the unknown sentinel.
    expect(err.url).toBe("(unknown)");
  }
});
