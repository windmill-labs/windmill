// Detects responses that look like an upstream auth-gateway challenge
// (Cloudflare Access SSO page, Vercel auth wall, basic-auth form, etc.) and
// surfaces a clear error instead of letting the caller treat the HTML body as
// a typed Windmill response.

const ACCESS_TITLE = /<title>\s*Sign in[^<]*Cloudflare Access\s*<\/title>/i;
const HTML_DOCTYPE = /^\s*<(!doctype|html)/i;

export class AuthGatewayChallengeError extends Error {
  override name = "AuthGatewayChallengeError";
  constructor(
    public url: string,
    public cfRay: string | undefined,
    public cfMitigated: string | undefined,
    public status: number,
    public bodySnippet: string,
  ) {
    const cfPart = [
      cfRay ? `cf-ray=${cfRay}` : null,
      cfMitigated ? `cf-mitigated=${cfMitigated}` : null,
    ]
      .filter(Boolean)
      .join(", ");
    super(
      `Got an HTML response from ${url} (status ${status}${cfPart ? `, ${cfPart}` : ""}). ` +
        `The request was intercepted by an upstream auth gateway (likely Cloudflare Access) ` +
        `before reaching Windmill. Verify the runner is on the right network or pass service-token headers via the HEADERS env var ` +
        `(e.g. HEADERS="CF-Access-Client-Id: <id>, CF-Access-Client-Secret: <secret>"). ` +
        `Body starts with: ${JSON.stringify(bodySnippet.slice(0, 120))}`,
    );
  }
}

export async function detectAuthGatewayChallenge(
  response: Response,
  url?: string,
): Promise<void> {
  const contentType = (response.headers.get("content-type") ?? "").toLowerCase();
  const cfMitigated = response.headers.get("cf-mitigated") ?? undefined;
  const looksHtml = contentType.includes("text/html");

  // Cheap check first; only peek the body when something already smells off.
  if (!looksHtml && cfMitigated !== "challenge") return;

  let snippet = "";
  try {
    snippet = (await response.clone().text()).slice(0, 256);
  } catch {
    /* body unreadable — fall through */
  }

  const isChallenge =
    cfMitigated === "challenge" ||
    ACCESS_TITLE.test(snippet) ||
    (looksHtml && HTML_DOCTYPE.test(snippet));

  if (!isChallenge) return;

  throw new AuthGatewayChallengeError(
    url || response.url || "(unknown)",
    response.headers.get("cf-ray") ?? undefined,
    cfMitigated,
    response.status,
    snippet,
  );
}
