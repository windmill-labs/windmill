import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// Workspace deploy builds the variable body field by field, so a forgotten field is
// dropped silently on promotion, and a source with no expiry must send `null`.
function recordingProvider(
  captured: [string, any][],
  exists: boolean,
  value_expires_at: string | null,
) {
  return {
    existsVariable: async () => exists,
    getVariable: async () => ({
      path: "f/x/v",
      value: "secret",
      is_secret: true,
      description: "d",
      value_expires_at,
    }),
    createVariable: async (p: any) =>
      void captured.push(["createVariable", p.requestBody]),
    updateVariable: async (p: any) =>
      void captured.push(["updateVariable", p.requestBody]),
  } as any;
}

test("deployItem: carries value_expires_at, and clears it explicitly", async () => {
  const captured: [string, any][] = [];

  // Written once per branch: create and update.
  for (const exists of [false, true]) {
    await deployItem(
      recordingProvider(captured, exists, "2027-03-15T08:00:00Z"),
      "variable" as any,
      "f/x/v",
      "src",
      "dst",
    );
    await deployItem(
      recordingProvider(captured, exists, null),
      "variable" as any,
      "f/x/v",
      "src",
      "dst",
    );
  }

  expect(captured.map(([fn]) => fn)).toEqual([
    "createVariable",
    "createVariable",
    "updateVariable",
    "updateVariable",
  ]);
  expect(captured.map(([, body]) => body.value_expires_at)).toEqual([
    "2027-03-15T08:00:00Z",
    null,
    "2027-03-15T08:00:00Z",
    null,
  ]);
});
