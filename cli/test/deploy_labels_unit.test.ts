import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// Variables, resources and folders are deployed from a body built field by field rather than
// spread from the source item, so any field left out of it is silently dropped in the target.
test("deployItem: carries labels for variables, resources and folders", async () => {
  const labels = ["prod", "billing"];
  const writes: [string, any][] = [];
  const provider = (exists: boolean) =>
    ({
      existsVariable: async () => exists,
      existsResource: async () => exists,
      existsFolder: async () => exists,
      getVariable: async () => ({ value: "v", is_secret: false, labels }),
      getResource: async () => ({
        value: {},
        resource_type: "postgresql",
        labels,
      }),
      getFolder: async () => ({
        name: "x",
        owners: [],
        extra_perms: {},
        labels,
      }),
      ...Object.fromEntries(
        [
          "createVariable",
          "updateVariable",
          "createResource",
          "updateResource",
          "createFolder",
          "updateFolder",
        ].map((fn) => [
          fn,
          async (p: any) => void writes.push([fn, p.requestBody]),
        ]),
      ),
    }) as any;

  for (const exists of [false, true]) {
    for (const [kind, path] of [
      ["variable", "f/x/v"],
      ["resource", "f/x/r"],
      ["folder", "f/x"],
    ]) {
      const result = await deployItem(
        provider(exists),
        kind as any,
        path,
        "src",
        "dst",
      );
      expect(result).toEqual({ success: true });
    }
  }

  expect(writes.map(([fn]) => fn).sort()).toEqual([
    "createFolder",
    "createResource",
    "createVariable",
    "updateFolder",
    "updateResource",
    "updateVariable",
  ]);
  for (const [, body] of writes) expect(body.labels).toEqual(labels);
});

test("deployItem: clears a folder's labels when the source has none", async () => {
  let body: any;
  await deployItem(
    {
      existsFolder: async () => true,
      // A folder with cleared labels reads back without the field at all.
      getFolder: async () => ({ name: "x", owners: [], extra_perms: {} }),
      updateFolder: async (p: any) => void (body = p.requestBody),
    } as any,
    "folder",
    "f/x",
    "src",
    "dst",
  );
  expect(body.labels).toEqual([]);
});
