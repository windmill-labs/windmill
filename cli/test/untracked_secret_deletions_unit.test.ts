/**
 * `sync push` archives a script but hard-deletes a variable or a resource, so a
 * remote-only one it has never tracked must not be read as a deletion: it is a
 * credential provisioned on the instance, or runtime state a script wrote. These
 * pin the classification (which files count, which look alike but don't) and the
 * evidence rule (only committed history vouches for a deletion).
 */

import { describe, expect, test } from "bun:test";
import {
  secretBearingObjectKind,
  untrackedSecretBearingDeletions,
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
    // Workspace-specific items carry the workspace name before the suffix.
    expect(secretBearingObjectKind("f/test/db.prod.resource.yaml")).toBe(
      "resource",
    );
  });

  test("ignores files that only look like one", () => {
    for (const p of [
      // Carries no secret, and is not what the issue is about.
      "f/test/my_type.resource-type.json",
      // A fileset/file-resource child: pushed by re-pushing its parent, never
      // deleted on its own.
      "f/test/conf.resource.file.ini",
      "f/test/bar.script.yaml",
      "f/test/foo.flow/flow.yaml",
    ]) {
      expect(secretBearingObjectKind(p)).toBeUndefined();
    }
  });
});

describe("untrackedSecretBearingDeletions", () => {
  const VAR = "f/test/protocol.variable.yaml";
  const RES = "f/test/erp_access.resource.yaml";
  const changes = [
    { name: "deleted", path: VAR },
    { name: "deleted", path: RES },
    { name: "deleted", path: "f/test/bar.script.yaml" },
    { name: "added", path: "f/test/new.resource.yaml" },
  ];

  test("trusts a deletion the repository has committed before", () => {
    expect(
      untrackedSecretBearingDeletions(changes, {
        kind: "known",
        paths: new Set([VAR, RES]),
      }),
    ).toEqual([]);
  });

  test("history recorded under the other serialization still vouches", () => {
    // A repo that switched between YAML and `--json` tracked the same object
    // under a different extension.
    expect(
      untrackedSecretBearingDeletions(changes, {
        kind: "known",
        paths: new Set([
          "f/test/protocol.variable.json",
          "f/test/erp_access.resource.json",
        ]),
      }),
    ).toEqual([]);
  });

  test("flags objects this repository has never recorded", () => {
    expect(
      untrackedSecretBearingDeletions(changes, {
        kind: "known",
        paths: new Set(),
      }).map((c) => c.path),
    ).toEqual([VAR, RES]);
  });

  test("surfaces everything when the history cannot be consulted", () => {
    // Nothing is proven either way, so every candidate comes back for the caller
    // to warn about (and, on a TTY, ask about).
    expect(
      untrackedSecretBearingDeletions(changes, {
        kind: "unknown",
        reason: "this is a shallow clone, so its history is truncated",
        remedy: "Fetch the full history (for actions/checkout, fetch-depth: 0)",
      }).map((c) => c.path),
    ).toEqual([VAR, RES]);
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
    expect(
      describeSecretBearingChanges([{ path: "f/c.resource.yaml" }]),
    ).toBe("1 resource");
  });
});
