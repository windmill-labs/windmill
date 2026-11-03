import { describe, expect, it } from "bun:test";
import { rootNodes } from "./pipelineRun";

describe("rootNodes", () => {
  const paths = ["f/shop/ingest", "f/shop/daily"];
  const edges = [
    { runnable_path: "f/shop/ingest", asset_kind: "ducklake", asset_path: "main/orders", access_type: "w" as const },
    { runnable_path: "f/shop/daily", asset_kind: "ducklake", asset_path: "main/orders", access_type: "r" as const },
  ];

  it("does not run a node that reads an upstream table directly, whatever it triggers on", () => {
    const triggers = [
      { trigger_kind: "asset", asset_kind: "ducklake", asset_path: "main/order", runnable_path: "f/shop/daily" },
    ];
    expect(rootNodes(paths, triggers, edges)).toEqual(["f/shop/ingest"]);
  });

  it("keeps a node that reads its own materialized table a root", () => {
    const selfRead = [
      { runnable_path: "f/shop/ingest", asset_kind: "ducklake", asset_path: "main/orders", access_type: "rw" as const },
    ];
    expect(rootNodes(["f/shop/ingest"], [], selfRead)).toEqual(["f/shop/ingest"]);
  });
});
