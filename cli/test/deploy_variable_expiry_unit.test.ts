import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// Workspace deploy builds the variable body field by field, so a field the variable
// carries but the body forgets is dropped on promotion — silently, since the deploy
// still reports success. The clear half matters as much as the carry: the server reads
// an absent `value_expires_at` as "leave the stored date alone", so a source with no
// expiry has to say so explicitly or the target keeps a date the source no longer has.
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

  // The field is written out once per branch, so exercise both: a variable absent from
  // the target (create) and one already there (update).
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
