/**
 * The trashbin notice a push prints is only as good as its classification, which has
 * to agree with the apply loop on two things: which deleted files are a variable or a
 * resource — a path the loop skips must not be counted, or the notice announces a
 * deletion that never happened — and that the unit is the server-side object, so a
 * file resource's two files are the one deletion they cause.
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
