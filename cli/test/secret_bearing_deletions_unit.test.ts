/**
 * `sync push` deletes a variable or resource outright, and both go to the workspace
 * trash rather than being lost, which the CLI is the one surface that never said. The
 * notice it now prints is only as good as its classification: these pin which files
 * count as one of those two kinds, which look alike but aren't, and how a file
 * resource's two files collapse to the single deletion they are.
 */

import { describe, expect, test } from "bun:test";
import {
  secretBearingObjectKind,
  describeSecretBearingChanges,
} from "../src/commands/sync/sync.ts";

describe("secretBearingObjectKind", () => {
  test("matches variable and resource metadata in both serializations", () => {
    expect(secretBearingObjectKind("f/test/protocol.variable.yaml")).toBe(
      "variable",
    );
    expect(secretBearingObjectKind("f/test/erp_access.resource.json")).toBe(
      "resource",
    );
    // A file resource's content file deletes the resource outright, so it counts.
    expect(secretBearingObjectKind("f/test/conf.resource.file.ini")).toBe(
      "resource",
    );
  });

  test("ignores files that only look like one", () => {
    for (const p of [
      "f/test/my_type.resource-type.json",
      // A fileset child can be any file; deleting one re-pushes the parent resource
      // rather than deleting anything.
      "f/test/data.fileset/edge/inner.resource.yaml",
      "f/test/bar.script.yaml",
      "f/test/foo.flow/flow.yaml",
      // The apply loop skips a `.lock` deletion outright, so counting one would
      // announce a deletion that never happens. Reachable for a resource type whose
      // format_extension is literally `lock`.
      "f/test/conf.resource.file.lock",
    ]) {
      expect(secretBearingObjectKind(p)).toBeUndefined();
    }
  });
});

describe("describeSecretBearingChanges", () => {
  test("counts each kind separately and pluralizes", () => {
    expect(
      describeSecretBearingChanges([
        { path: "f/a.variable.yaml" },
        { path: "f/b.variable.yaml" },
        { path: "f/c.resource.yaml" },
      ]),
    ).toBe("2 variables and 1 resource");
  });

  test("counts a file resource's two files as the one resource they delete", () => {
    expect(
      describeSecretBearingChanges([
        { path: "f/c.resource.yaml" },
        { path: "f/c.resource.file.ini" },
      ]),
    ).toBe("1 resource");
  });
});
