/** Only a `.css` 404 may be forgiven — see `getRawAppBundlePart`. */

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
  test("requests each half under its own extension", async () => {
    const asked: Array<{ secretWithExtension: string; workspace: string }> = [];
    const provider = {
      getRawAppData: (p: { secretWithExtension: string; workspace: string }) => {
        asked.push(p);
        return Promise.resolve(`body-of-${p.secretWithExtension}`);
      },
    } as any;

    expect(await getRawAppBundlePart(provider, "sec", "js", "ws")).toBe(
      "body-of-sec.js"
    );
    expect(await getRawAppBundlePart(provider, "sec", "css", "ws")).toBe(
      "body-of-sec.css"
    );
    // Swapping the two extensions would deploy the stylesheet as the bundle.
    expect(asked).toEqual([
      { secretWithExtension: "sec.js", workspace: "ws" },
      { secretWithExtension: "sec.css", workspace: "ws" },
    ]);
  });

  test("treats a missing .css as empty rather than fatal", async () => {
    const provider = providerThatFailsWith(httpError(404));
    expect(await getRawAppBundlePart(provider, "sec", "css", "ws")).toBe("");
  });

  test("still fails loudly on a missing .js", async () => {
    const provider = providerThatFailsWith(httpError(404));
    await expect(
      getRawAppBundlePart(provider, "sec", "js", "ws")
    ).rejects.toThrow();
  });

  test("does not swallow non-404 failures on .css", async () => {
    for (const status of [401, 403, 500]) {
      const provider = providerThatFailsWith(httpError(status));
      await expect(
        getRawAppBundlePart(provider, "sec", "css", "ws")
      ).rejects.toThrow();
    }
  });

  test("does not swallow a .css failure that carries no status", async () => {
    const provider = providerThatFailsWith(new Error("network down"));
    await expect(
      getRawAppBundlePart(provider, "sec", "css", "ws")
    ).rejects.toThrow("network down");
  });
});
