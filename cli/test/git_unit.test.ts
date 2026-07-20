/**
 * Unit tests for git utility functions.
 * Tests pure functions only — no git subprocess calls.
 */

import { expect, test, describe } from "bun:test";
import {
  getOriginalBranchForWorkspaceForks,
  getWorkspaceIdForWorkspaceForkFromBranchName,
  computeGitSyncDeployBranch,
  composeGitSyncCommitHeader,
  forkBranchName,
  gitSyncIncludePattern,
  deriveGitSyncDeployIncludes,
  gitSyncCommitMessage,
  isForkWorkspace,
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

// =============================================================================
// computeGitSyncDeployBranch — the regression guard.
//
// hub/28229 dropped this logic and pushed every deploy straight to the cloned
// base branch (e.g. protected `main`), breaking promotion. These pin the
// hub/28217 / hub/28230 branch-selection semantics.
// =============================================================================

describe("computeGitSyncDeployBranch", () => {
  const base = {
    workspaceId: "prod",
    clonedBranchName: "main",
    groupByFolder: false,
  };

  test("use_individual_branch=true -> dedicated wm_deploy branch (NOT main)", () => {
    const branch = computeGitSyncDeployBranch({
      ...base,
      useIndividualBranch: true,
      items: [{ path_type: "script", path: "f/foo/bar" }],
    });
    expect(branch).toBe("wm_deploy/prod/script/f__foo__bar");
    expect(branch).not.toBe("main");
  });

  test("dev workspace (parent + label) deploys to the label branch", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        workspaceId: "staging-ws",
        parentWorkspaceId: "prod",
        devWorkspaceLabel: "staging",
        useIndividualBranch: false,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBe("staging");
  });

  test("dev workspace in promotion mode -> per-item wm_deploy branch, not the label branch", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        workspaceId: "staging-ws",
        parentWorkspaceId: "prod",
        devWorkspaceLabel: "staging",
        useIndividualBranch: true,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBe("wm_deploy/staging-ws/script/f__foo__bar");
  });

  test("dev workspace in promotion mode honors group_by_folder", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        workspaceId: "staging-ws",
        parentWorkspaceId: "prod",
        devWorkspaceLabel: "staging",
        useIndividualBranch: true,
        groupByFolder: true,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBe("wm_deploy/staging-ws/f__foo");
  });

  test("use_individual_branch=false -> null (stay on base/main, workspace-wide mode)", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: false,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBeNull();
  });

  test("group_by_folder=true -> per-folder wm_deploy branch (first 2 path segments)", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: true,
        groupByFolder: true,
        items: [{ path_type: "script", path: "f/team_a/deep/nested" }],
      })
    ).toBe("wm_deploy/prod/f__team_a");
  });

  test("falls back to parent_path when path is absent", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: true,
        items: [{ path_type: "flow", path: null, parent_path: "f/x/y" }],
      })
    ).toBe("wm_deploy/prod/flow/f__x__y");
  });

  test("user/group objects never get a dedicated branch", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: true,
        items: [{ path_type: "user", path: "u/alice" }],
      })
    ).toBeNull();
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: true,
        items: [{ path_type: "group", path: "g/admins" }],
      })
    ).toBeNull();
  });

  test("empty items -> null", () => {
    expect(
      computeGitSyncDeployBranch({
        ...base,
        useIndividualBranch: true,
        items: [],
      })
    ).toBeNull();
  });

  test("fork workspace -> wm-fork/<clonedBranch>/<id> regardless of individual-branch", () => {
    expect(
      computeGitSyncDeployBranch({
        workspaceId: "wm-fork-myfork",
        clonedBranchName: "main",
        groupByFolder: false,
        useIndividualBranch: false,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBe("wm-fork/main/myfork");
  });

  test("prefix-less fork (parent set, no label) beats the wm_deploy derivation", () => {
    expect(
      computeGitSyncDeployBranch({
        workspaceId: "mydev",
        parentWorkspaceId: "prod",
        clonedBranchName: "main",
        groupByFolder: false,
        useIndividualBranch: true,
        items: [{ path_type: "script", path: "f/foo/bar" }],
      })
    ).toBe("wm-fork/main/mydev");
  });
});

describe("forkBranchName", () => {
  test("maps wm-fork-<id> to wm-fork/<branch>/<id>", () => {
    expect(forkBranchName("wm-fork-abc", "main")).toBe("wm-fork/main/abc");
  });

  test("dev workspace label wins: the branch is the label verbatim", () => {
    expect(forkBranchName("staging-ws", "main", "staging")).toBe("staging");
    expect(forkBranchName("wm-fork-abc", "main", "dev")).toBe("dev");
  });

  test("prefix-less id without a label falls back to the wm-fork form", () => {
    expect(forkBranchName("staging-ws", "main")).toBe("wm-fork/main/staging-ws");
  });
});

describe("composeGitSyncCommitHeader", () => {
  test("single type", () => {
    expect(
      composeGitSyncCommitHeader([{ path_type: "script", path: "a" }])
    ).toBe("[WM]: Deployed 1 script");
  });

  test("exactly 3 types: pluralises, orders by count, joins last with 'and'", () => {
    expect(
      composeGitSyncCommitHeader([
        { path_type: "script", path: "a" },
        { path_type: "script", path: "b" },
        { path_type: "flow", path: "c" },
        { path_type: "app", path: "d" },
      ])
    ).toBe("[WM]: Deployed 2 scripts, 1 flow, and 1 app");
  });

  test("4+ types: overflow collapses into 'other objects'", () => {
    expect(
      composeGitSyncCommitHeader([
        { path_type: "script", path: "a" },
        { path_type: "script", path: "b" },
        { path_type: "flow", path: "c" },
        { path_type: "app", path: "d" },
        { path_type: "resource", path: "e" },
        { path_type: "variable", path: "f" },
      ])
    ).toBe("[WM]: Deployed 2 scripts, 1 flow, 1 app and 2 other objects");
  });
});

// =============================================================================
// gitSyncIncludePattern / deriveGitSyncDeployIncludes — replaces the hub
// script's regexFromPath + wmill_sync_pull include-derivation.
// =============================================================================

describe("gitSyncIncludePattern", () => {
  test("script includes __mod/ folder for module scripts", () => {
    expect(gitSyncIncludePattern("script", "f/foo/bar")).toBe(
      "f/foo/bar.*,f/foo/bar__mod/**"
    );
  });
  test("flow/app expand to dotted + __ folder patterns", () => {
    expect(gitSyncIncludePattern("flow", "f/x")).toBe("f/x.flow/*,f/x__flow/*");
    expect(gitSyncIncludePattern("app", "f/x")).toBe("f/x.app/*,f/x__app/*");
    expect(gitSyncIncludePattern("raw_app", "f/x")).toBe(
      "f/x.raw_app/**,f/x__raw_app/**"
    );
  });
  test("folder and triggers", () => {
    expect(gitSyncIncludePattern("folder", "f/x")).toBe("f/x/folder.meta.*");
    expect(gitSyncIncludePattern("httptrigger", "f/t")).toBe(
      "f/t.http_trigger.*"
    );
    expect(gitSyncIncludePattern("gcptrigger", "f/t")).toBe(
      "f/t.gcp_trigger.*"
    );
  });
});

describe("deriveGitSyncDeployIncludes", () => {
  test("splits multi-pattern includes (flow) and includes parent_path", () => {
    const r = deriveGitSyncDeployIncludes(
      [{ path_type: "flow", path: "f/a", parent_path: "f/b" }],
      false
    );
    expect(r.extraIncludes).toEqual([
      "f/a.flow/*",
      "f/a__flow/*",
      "f/b.flow/*",
      "f/b__flow/*",
    ]);
  });

  test("workspace-wide mode force-includes deployed default-excluded kinds", () => {
    const r = deriveGitSyncDeployIncludes(
      [
        { path_type: "schedule", path: "f/s" },
        { path_type: "group", path: "g/g" },
        { path_type: "httptrigger", path: "f/t" },
        { path_type: "settings", path: "" },
        { path_type: "key", path: "" },
        { path_type: "user", path: "u/u" },
      ],
      false
    );
    // Full-mirror repo: a deployed object of a default-excluded kind must be
    // re-included even if wmill.yaml would skip it, so the flag is forced on.
    expect(r.forcedIncludes).toEqual({
      includeSchedules: true,
      includeGroups: true,
      includeTriggers: true,
      includeSettings: true,
      includeKey: true,
      includeUsers: true,
    });
  });

  test("workspace-wide mode only forces the kinds actually deployed", () => {
    const r = deriveGitSyncDeployIncludes(
      [{ path_type: "script", path: "f/s" }],
      false
    );
    // Scripts are included by default — nothing to force.
    expect(r.forcedIncludes).toEqual({});
  });

  test("individual-branch (promotion) mode forces NOTHING — defers to wmill.yaml", () => {
    // Regression: these flags used to be force-disabled (set to false) in
    // promotion mode, which CLOBBERED the promotion target's effective
    // wmill.yaml config (an explicit false wins in pull's Object.assign merge).
    // The server then stripped the object from the tarball, the pull wrote
    // nothing, and `git add '<path>**'` failed with "pathspec did not match
    // any files". Forcing nothing leaves the keys absent so the target's
    // effective filters govern; extraIncludes still scopes the pull to the
    // changed object.
    const r = deriveGitSyncDeployIncludes(
      [
        { path_type: "schedule", path: "f/s" },
        { path_type: "kafkatrigger", path: "f/t" },
      ],
      true
    );
    expect(r.forcedIncludes).toEqual({});
    expect(r.extraIncludes).toContain("f/s.schedule.*");
    expect(r.extraIncludes).toContain("f/t.kafka_trigger.*");
  });

  test("regression: http_trigger promotion deploy does not clobber the target's includeTriggers", () => {
    // Brad's scenario: an HTTP trigger is deployed and the promotion repo uses
    // individual branches. path_type is "httptrigger" (the no-underscore value
    // the backend puts on item.path_type — see git_sync_ee.rs
    // insert_path_type_and_return_message). includeTriggers must NOT be forced
    // false here, so the target's effective includeTriggers (true in Brad's
    // config) is honored and the trigger file is pulled and committed.
    const r = deriveGitSyncDeployIncludes(
      [{ path_type: "httptrigger", path: "f/platform/on_call_chat_http_route" }],
      true
    );
    expect(r.forcedIncludes.includeTriggers).toBeUndefined();
    expect(r.extraIncludes).toContain(
      "f/platform/on_call_chat_http_route.http_trigger.*"
    );
  });
});

// =============================================================================
// isForkWorkspace + gitSyncCommitMessage — 1:1 fidelity with the hub script.
// =============================================================================

describe("isForkWorkspace", () => {
  test("true only for the wm-fork- workspace-id prefix", () => {
    expect(isForkWorkspace("wm-fork-abc")).toBe(true);
    expect(isForkWorkspace("prod")).toBe(false);
    // "wm-fork" without the trailing dash is the BRANCH prefix, not a ws id
    expect(isForkWorkspace("wm-fork")).toBe(false);
  });

  test("a parent workspace id marks prefix-less ids (dev workspaces) as forks", () => {
    expect(isForkWorkspace("mydev", "prod")).toBe(true);
    expect(isForkWorkspace("mydev", undefined)).toBe(false);
    expect(isForkWorkspace("wm-fork-abc", undefined)).toBe(true);
  });
});

describe("gitSyncCommitMessage (mirrors hub git_push quirks)", () => {
  test("single item: uses its commit_msg verbatim, empty description", () => {
    expect(
      gitSyncCommitMessage([{ path_type: "script", path: "a", commit_msg: "msg" }])
    ).toEqual({ header: "msg", description: "" });
  });

  test("single item with NO commit_msg -> 'no commit msg' (hub fallback)", () => {
    expect(
      gitSyncCommitMessage([{ path_type: "script", path: "a" }])
    ).toEqual({ header: "no commit msg", description: "" });
  });

  test("multi item: composed header + newline-joined msgs, undefined -> ''", () => {
    // hub pushes commit_msg once per item UNCONDITIONALLY, so the second
    // (missing) entry joins as an empty string -> "first\n".
    expect(
      gitSyncCommitMessage([
        { path_type: "script", path: "a", commit_msg: "first" },
        { path_type: "flow", path: "b" },
      ])
    ).toEqual({ header: "[WM]: Deployed 1 script, 1 flow", description: "first\n" });
  });
});
