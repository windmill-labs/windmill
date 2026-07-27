/**
 * A raw-app bundle with no styles may have no `.css` blob stored at all, which
 * older backends serve as a 404. Deploy re-fetches both parts, so letting that
 * 404 reject makes the app permanently un-deployable.
 *
 * The tolerance is deliberately narrow, and that narrowness is the point: a
 * blanket catch would swallow auth/network failures and silently deploy an app
 * with its styles stripped, and a missing `.js` is a genuinely broken bundle.
 *
 * No backend required — the provider is a stub.
 */

import { expect, test, describe } from "bun:test";
import { getRawAppBundlePart } from "../windmill-utils-internal/src/deploy.ts";

function providerThatFailsWith(err: unknown) {
  return {
    getRawAppData: () => Promise.reject(err),
  } as any;
}

function httpError(status: number) {
  return Object.assign(new Error(`HTTP ${status}`), { status });
}

describe("getRawAppBundlePart", () => {
  test("returns the blob when the fetch succeeds", async () => {
    const provider = {
      getRawAppData: (p: { secretWithExtension: string }) =>
        Promise.resolve(`body-of-${p.secretWithExtension}`),
    } as any;

    expect(await getRawAppBundlePart(provider, "sec", "js", "ws")).toBe(
      "body-of-sec.js"
    );
    expect(await getRawAppBundlePart(provider, "sec", "css", "ws")).toBe(
      "body-of-sec.css"
    );
  });

  test("treats a missing .css as empty rather than fatal", async () => {
    const provider = providerThatFailsWith(httpError(404));
    expect(await getRawAppBundlePart(provider, "sec", "css", "ws")).toBe("");
  });

  test("still fails loudly on a missing .js", async () => {
    const provider = providerThatFailsWith(httpError(404));
    expect(getRawAppBundlePart(provider, "sec", "js", "ws")).rejects.toThrow();
  });

  test("does not swallow non-404 failures on .css", async () => {
    for (const status of [401, 403, 500]) {
      const provider = providerThatFailsWith(httpError(status));
      expect(
        getRawAppBundlePart(provider, "sec", "css", "ws")
      ).rejects.toThrow();
    }
  });

  test("does not swallow a .css failure that carries no status", async () => {
    const provider = providerThatFailsWith(new Error("network down"));
    expect(getRawAppBundlePart(provider, "sec", "css", "ws")).rejects.toThrow(
      "network down"
    );
  });
});
