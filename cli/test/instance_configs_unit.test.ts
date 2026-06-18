/**
 * Unit tests for pullInstanceConfigs / pushInstanceConfigs in settings.ts.
 *
 * Verifies that:
 * - pullInstanceConfigs writes only worker group configs (no alerts)
 * - pushInstanceConfigs calls updateConfig with worker__ prefix
 * - pushInstanceConfigs calls deleteConfig with worker__ prefix for removed configs
 * - pushInstanceConfigs skips unchanged configs
 */

import { expect, test, describe, beforeEach, afterEach, mock } from "bun:test";
import { writeFile, readFile, mkdir, rm } from "node:fs/promises";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { stringify as yamlStringify } from "yaml";

// Track calls to mocked wmill functions
let listWorkerGroupsResult: any[] = [];
let updateConfigCalls: { name: string; requestBody: any }[] = [];
let deleteConfigCalls: { name: string }[] = [];

// Mock the wmill module before importing settings.ts
mock.module("../gen/services.gen.ts", () => ({
  listWorkerGroups: async () => listWorkerGroupsResult,
  updateConfig: async (args: { name: string; requestBody: any }) => {
    updateConfigCalls.push(args);
  },
  deleteConfig: async (args: { name: string }) => {
    deleteConfigCalls.push(args);
  },
  listConfigs: async () => {
    throw new Error("listConfigs should not be called");
  },
}));

import {
  pullInstanceConfigs,
  pushInstanceConfigs,
  readLocalConfigs,
} from "../src/core/settings.ts";

describe("instance configs", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), "wm-config-test-"));
    listWorkerGroupsResult = [];
    updateConfigCalls = [];
    deleteConfigCalls = [];
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true });
  });

  // =========================================================================
  // readLocalConfigs
  // =========================================================================

  describe("readLocalConfigs", () => {
    test("reads configs from instance_configs.yaml", async () => {
      const configs = [
        { name: "default", config: { worker_tags: ["deno", "bun"] } },
        { name: "gpu", config: { dedicated_worker: "ws:f/gpu_script" } },
      ];
      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(configs),
        "utf-8"
      );

      const result = await readLocalConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });
      expect(result).toEqual(configs);
    });

    test("returns empty array when file does not exist", async () => {
      const result = await readLocalConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });
      expect(result).toEqual([]);
    });
  });

  // =========================================================================
  // pullInstanceConfigs
  // =========================================================================

  describe("pullInstanceConfigs", () => {
    test("writes worker group configs to instance_configs.yaml", async () => {
      listWorkerGroupsResult = [
        { name: "default", config: { worker_tags: ["deno", "bun"] } },
        { name: "native", config: { worker_tags: ["nativets"] } },
      ];

      // readLocalConfigs sets instanceConfigsPath when prefix is used
      const opts = {
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      };
      await readLocalConfigs(opts);
      await pullInstanceConfigs(opts);

      const content = await readFile(
        join(tempDir, "instance_configs.yaml"),
        "utf-8"
      );
      expect(content).toContain("default");
      expect(content).toContain("native");
      expect(content).toContain("deno");
      // Should not contain alert entries (listWorkerGroups filters them)
      expect(content).not.toContain("alert");
    });

    test("preview mode returns change count without writing file", async () => {
      listWorkerGroupsResult = [
        { name: "default", config: { worker_tags: ["deno"] } },
      ];

      const changes = await pullInstanceConfigs(
        {
          prefix: tempDir,
          folderPerInstance: true,
          prefixSettings: true,
        },
        true
      );

      // One remote config not in local = 1 change
      expect(changes).toBe(1);
    });

    test("preview mode returns 0 when remote matches local", async () => {
      const configs = [
        { name: "default", config: { worker_tags: ["deno"] } },
      ];
      listWorkerGroupsResult = configs;

      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(configs),
        "utf-8"
      );

      const changes = await pullInstanceConfigs(
        {
          prefix: tempDir,
          folderPerInstance: true,
          prefixSettings: true,
        },
        true
      );

      expect(changes).toBe(0);
    });
  });

  // =========================================================================
  // pushInstanceConfigs
  // =========================================================================

  describe("pushInstanceConfigs", () => {
    test("calls updateConfig with worker__ prefix for new configs", async () => {
      listWorkerGroupsResult = [];
      const localConfigs = [
        { name: "mygroup", config: { worker_tags: ["python3"] } },
      ];
      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(localConfigs),
        "utf-8"
      );

      await pushInstanceConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });

      expect(updateConfigCalls).toHaveLength(1);
      expect(updateConfigCalls[0].name).toBe("worker__mygroup");
      expect(updateConfigCalls[0].requestBody).toEqual({
        worker_tags: ["python3"],
      });
    });

    test("calls deleteConfig with worker__ prefix for removed configs", async () => {
      listWorkerGroupsResult = [
        { name: "old_group", config: { worker_tags: ["bash"] } },
      ];
      // Empty local configs = old_group should be deleted
      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify([]),
        "utf-8"
      );

      await pushInstanceConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });

      expect(deleteConfigCalls).toHaveLength(1);
      expect(deleteConfigCalls[0].name).toBe("worker__old_group");
    });

    test("skips unchanged configs", async () => {
      const configs = [
        { name: "default", config: { worker_tags: ["deno"] } },
      ];
      listWorkerGroupsResult = configs;

      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(configs),
        "utf-8"
      );

      await pushInstanceConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });

      expect(updateConfigCalls).toHaveLength(0);
      expect(deleteConfigCalls).toHaveLength(0);
    });

    test("updates changed configs and deletes removed ones", async () => {
      listWorkerGroupsResult = [
        { name: "keep", config: { worker_tags: ["old_tag"] } },
        { name: "remove_me", config: { worker_tags: ["bash"] } },
      ];
      const localConfigs = [
        { name: "keep", config: { worker_tags: ["new_tag"] } },
        { name: "add_me", config: { worker_tags: ["python3"] } },
      ];
      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(localConfigs),
        "utf-8"
      );

      await pushInstanceConfigs({
        prefix: tempDir,
        folderPerInstance: true,
        prefixSettings: true,
      });

      // "keep" was changed, "add_me" is new
      expect(updateConfigCalls).toHaveLength(2);
      const updateNames = updateConfigCalls.map((c) => c.name).sort();
      expect(updateNames).toEqual(["worker__add_me", "worker__keep"]);

      // "remove_me" was deleted
      expect(deleteConfigCalls).toHaveLength(1);
      expect(deleteConfigCalls[0].name).toBe("worker__remove_me");
    });

    test("preview mode returns change count without calling API", async () => {
      listWorkerGroupsResult = [
        { name: "default", config: { worker_tags: ["deno"] } },
      ];
      const localConfigs = [
        { name: "default", config: { worker_tags: ["bun"] } },
        { name: "new_group", config: { worker_tags: ["go"] } },
      ];
      await writeFile(
        join(tempDir, "instance_configs.yaml"),
        yamlStringify(localConfigs),
        "utf-8"
      );

      const changes = await pushInstanceConfigs(
        {
          prefix: tempDir,
          folderPerInstance: true,
          prefixSettings: true,
        },
        true
      );

      // "default" changed + "new_group" added = 2 changes
      expect(changes).toBe(2);
      expect(updateConfigCalls).toHaveLength(0);
      expect(deleteConfigCalls).toHaveLength(0);
    });
  });
});
