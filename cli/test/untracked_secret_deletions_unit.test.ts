/**
 * A remote variable or resource with no local file is deleted either way, but one
 * this branch has never tracked was provisioned outside the repository — a
 * credential set on the instance, or runtime state a script wrote — and the push
 * says so. These pin what that claim rests on: the classification (which files
 * count, which look alike but don't) and the evidence rule (a file in committed
 * history vouches for the whole object).
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
    // A file resource's content file: deleting it deletes the resource outright,
    // so it must be guarded like the metadata file.
    expect(secretBearingObjectKind("f/test/conf.resource.file.ini")).toBe(
      "resource",
    );
  });

  test("ignores files that only look like one", () => {
    for (const p of [
      // Carries no secret, and is not what the issue is about.
      "f/test/my_type.resource-type.json",
      // A fileset child can be any file: its deletion re-pushes the parent
      // resource rather than deleting anything.
      "f/test/data.fileset/edge/inner.resource.yaml",
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

  test("history recorded under a workspace-specific name still vouches", () => {
    // `specificItems` commits `y.staging.variable.yaml` while the changeset
    // carries the base path, so both names have to be searched.
    expect(
      untrackedSecretBearingDeletions(
        changes,
        {
          kind: "known",
          paths: new Set([
            "f/test/protocol.staging.variable.yaml",
            "f/test/erp_access.staging.resource.yaml",
          ]),
        },
        (p) => p.replace(/\.staging\.(variable|resource)\./, ".$1."),
      ),
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

  test("a companion file the push is not deleting still vouches", () => {
    // Only the content file goes; `f/test/conf.resource.yaml` stays on disk, so it
    // is not a candidate — but it is the history that proves the repo owned `f/test/conf`.
    expect(
      untrackedSecretBearingDeletions(
        [{ name: "deleted", path: "f/test/conf.resource.file.ini" }],
        { kind: "known", paths: new Set(["f/test/conf.resource.yaml"]) },
      ),
    ).toEqual([]);
  });

  test("an object with any file in history is not reported as never tracked", () => {
    // A file resource is two files for one resource; one of them in history is
    // proof the repository owned it, so calling it never-tracked would be false.
    expect(
      untrackedSecretBearingDeletions(
        [
          { name: "deleted", path: "f/test/conf.resource.yaml" },
          { name: "deleted", path: "f/test/conf.resource.file.ini" },
        ],
        { kind: "known", paths: new Set(["f/test/conf.resource.yaml"]) },
      ),
    ).toEqual([]);
  });

  test("a variable and a resource at one path vouch only for themselves", () => {
    // They share a path but are two backend objects, so committing the variable
    // says nothing about a resource the repository never had.
    expect(
      untrackedSecretBearingDeletions(
        [
          { name: "deleted", path: "f/test/pair.variable.yaml" },
          { name: "deleted", path: "f/test/pair.resource.yaml" },
        ],
        { kind: "known", paths: new Set(["f/test/pair.variable.yaml"]) },
      ).map((c) => c.path),
    ).toEqual(["f/test/pair.resource.yaml"]);
  });

  test("surfaces everything when the history cannot be consulted", () => {
    // Nothing is proven either way, so every candidate comes back and the caller
    // hedges the wording instead of asserting.
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

  test("counts a file resource's two files as the one resource they delete", () => {
    expect(
      describeSecretBearingChanges([
        { path: "f/c.resource.yaml" },
        { path: "f/c.resource.file.ini" },
      ]),
    ).toBe("1 resource");
  });
});
