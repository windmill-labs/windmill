/**
 * Unit tests for git utility functions.
 * Tests pure functions only — no git subprocess calls.
 */

import { expect, test, describe } from "bun:test";
import {
  getOriginalBranchForWorkspaceForks,
  getWorkspaceIdForWorkspaceForkFromBranchName,
} from "../src/utils/git.ts";

// =============================================================================
// getOriginalBranchForWorkspaceForks
// =============================================================================

describe("getOriginalBranchForWorkspaceForks", () => {
  test("extracts original branch from valid fork branch name", () => {
    expect(getOriginalBranchForWorkspaceForks("wm-fork/main/my-workspace")).toBe("main");
  });

  test("extracts multi-segment original branch", () => {
    expect(
      getOriginalBranchForWorkspaceForks("wm-fork/feature/cool-thing/my-workspace")
    ).toBe("feature/cool-thing");
  });

  test("returns null for null input", () => {
    expect(getOriginalBranchForWorkspaceForks(null)).toBeNull();
  });

  test("returns null for empty string", () => {
    expect(getOriginalBranchForWorkspaceForks("")).toBeNull();
  });

  test("returns null for non-fork branch", () => {
    expect(getOriginalBranchForWorkspaceForks("main")).toBeNull();
    expect(getOriginalBranchForWorkspaceForks("feature/my-feature")).toBeNull();
  });

  test("returns null for branch that starts with wm-fork but has no slashes after", () => {
    expect(getOriginalBranchForWorkspaceForks("wm-fork")).toBeNull();
  });

  test("returns null when branch segment between slashes is empty", () => {
    // "wm-fork//workspace" — start=8, end=8, end - start = 0
    expect(getOriginalBranchForWorkspaceForks("wm-fork//workspace")).toBeNull();
  });
});

// =============================================================================
// getWorkspaceIdForWorkspaceForkFromBranchName
// =============================================================================

describe("getWorkspaceIdForWorkspaceForkFromBranchName", () => {
  test("extracts workspace id from valid fork branch name", () => {
    expect(
      getWorkspaceIdForWorkspaceForkFromBranchName("wm-fork/main/my-workspace")
    ).toBe("wm-fork-my-workspace");
  });

  test("returns null for non-fork branch", () => {
    expect(getWorkspaceIdForWorkspaceForkFromBranchName("main")).toBeNull();
    expect(
      getWorkspaceIdForWorkspaceForkFromBranchName("feature/my-feature")
    ).toBeNull();
  });

  test("extracts workspace id with multi-segment original branch", () => {
    expect(
      getWorkspaceIdForWorkspaceForkFromBranchName("wm-fork/feature/cool/ws-id")
    ).toBe("wm-fork-ws-id");
  });
});
