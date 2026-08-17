/**
 * Unit tests for stable auto-numbered inline-script naming in apps.
 *
 * Bug: when an app has multiple components that share the same runnable
 * `name` (e.g. several "Eval of b" cells), the path-assigner auto-numbers
 * them (`eval_of_b`, `eval_of_b_1`, …) in traversal order. Before the fix,
 * traversal walked the in-memory `Object.entries` order returned by
 * `JSON.parse` — i.e. whatever key order the server happened to send —
 * while the YAML output sorted keys via `yamlOptions.sortMapEntries`. The
 * two orders could disagree, so identically named scripts ended up with
 * numbers that didn't line up with their position in `app.yaml` and could
 * shuffle between pulls when the server returned keys in a different
 * order.
 *
 * Fix: traverse with `yamlSortedEntries`, which sorts plain-object keys
 * with the same comparator as YAML output (and preserves array order).
 * Names are then assigned in the order the scripts will appear on disk,
 * making them deterministic across pulls.
 *
 * No backend required — tests the extractor directly.
 */

import { expect, test, describe } from "bun:test";
import {
  extractInlineScriptsForApps,
  yamlOptions,
} from "../src/commands/sync/sync.ts";
import { newPathAssigner } from "../windmill-utils-internal/src/path-utils/path-assigner.ts";
import { stringify as yamlStringify } from "yaml";

function makeEvalCell(name: string, expr: string) {
  return {
    componentInput: {
      type: "evalv2",
      expr,
      runnable: {
        name,
        inlineScript: {
          content: "return JSON.stringify(freezeTable.selectedRow, null, 2)",
          language: "frontend",
        },
      },
    },
  };
}

function buildAppValue(subgridKeysInOrder: string[]) {
  const subgrids: Record<string, any[]> = {};
  for (const key of subgridKeysInOrder) {
    subgrids[key] = [
      {
        id: `${key}_text`,
        data: makeEvalCell("Eval of b", `JSON.stringify(${key}.selectedRow)`),
      },
    ];
  }
  return { subgrids };
}

describe("extractInlineScriptsForApps — stable auto-numbered naming", () => {
  test("auto-numbered names match YAML output order, not server insertion order", () => {
    // Server returns subgrids with keys in non-alphabetical order — e.g.
    // "r-0" before "pendingContainer-0". This is the order the user reports
    // in their bug repro.
    const value = buildAppValue([
      "b-0",
      "d-0",
      "g-0",
      "n-0",
      "r-0",
      "pendingContainer-0",
    ]);

    const scripts = extractInlineScriptsForApps(
      undefined,
      value,
      newPathAssigner("bun"),
      (_, val) => val["name"],
      false,
    );

    // Names assigned in YAML output order (alphabetical via prioritizeName):
    //   b-0, d-0, g-0, n-0, pendingContainer-0, r-0
    const paths = scripts.map((s) => s.path);
    expect(paths).toEqual([
      "eval_of_b.inline_script.frontend.js",
      "eval_of_b_1.inline_script.frontend.js",
      "eval_of_b_2.inline_script.frontend.js",
      "eval_of_b_3.inline_script.frontend.js",
      "eval_of_b_4.inline_script.frontend.js", // pendingContainer-0
      "eval_of_b_5.inline_script.frontend.js", // r-0
    ]);
  });

  test("naming is identical regardless of server insertion order", () => {
    const insertionOrders = [
      ["b-0", "d-0", "g-0", "n-0", "r-0", "pendingContainer-0"],
      ["pendingContainer-0", "r-0", "b-0", "d-0", "g-0", "n-0"],
      ["r-0", "n-0", "g-0", "d-0", "pendingContainer-0", "b-0"],
    ];

    const refRefs: Record<string, string> = {};
    for (const order of insertionOrders) {
      const value = buildAppValue(order);
      extractInlineScriptsForApps(
        undefined,
        value,
        newPathAssigner("bun"),
        (_, val) => val["name"],
        false,
      );
      // After extraction, every cell's inlineScript.content has been
      // rewritten to "!inline <path>". Capture (subgridKey -> ref) so we
      // can compare across runs.
      const refs: Record<string, string> = {};
      for (const [subgridKey, items] of Object.entries(
        value.subgrids as Record<string, any[]>,
      )) {
        refs[subgridKey] = items[0].data.componentInput.runnable.inlineScript
          .content as string;
      }
      if (Object.keys(refRefs).length === 0) {
        Object.assign(refRefs, refs);
      } else {
        // Same subgrid must always get the same !inline reference,
        // regardless of which order the server returned the keys.
        expect(refs).toEqual(refRefs);
      }
    }
  });

  test("inline reference for each subgrid lines up with its YAML position", () => {
    const value = buildAppValue([
      "b-0",
      "d-0",
      "g-0",
      "n-0",
      "r-0",
      "pendingContainer-0",
    ]);
    extractInlineScriptsForApps(
      undefined,
      value,
      newPathAssigner("bun"),
      (_, val) => val["name"],
      false,
    );

    // After extraction, stringify with the same yamlOptions used in the
    // pull pipeline. The auto-numbered file references must appear in
    // sequential order (0, 1, 2, …) when read top-to-bottom.
    const yaml = yamlStringify(value, yamlOptions);
    const refs = [
      ...yaml.matchAll(/!inline (eval_of_b(?:_\d+)?)\.inline_script\.frontend\.js/g),
    ].map((m) => m[1]);
    expect(refs).toEqual([
      "eval_of_b",
      "eval_of_b_1",
      "eval_of_b_2",
      "eval_of_b_3",
      "eval_of_b_4",
      "eval_of_b_5",
    ]);
  });

  test("array order within a subgrid is preserved (not sorted)", () => {
    // Two cells with the same name in array order [zebra, apple]. Array
    // entries must keep their index order — sorting them would reorder
    // visible UI elements.
    const value = {
      grid: [
        {
          id: "zebra",
          data: makeEvalCell("Eval of b", "zebra.selectedRow"),
        },
        {
          id: "apple",
          data: makeEvalCell("Eval of b", "apple.selectedRow"),
        },
      ],
    };
    const scripts = extractInlineScriptsForApps(
      undefined,
      value,
      newPathAssigner("bun"),
      (_, val) => val["name"],
      false,
    );
    expect(scripts.map((s) => s.path)).toEqual([
      "eval_of_b.inline_script.frontend.js", // zebra (array index 0)
      "eval_of_b_1.inline_script.frontend.js", // apple (array index 1)
    ]);
  });
});
