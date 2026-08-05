import { expect, test } from "bun:test";
import { deployItem } from "../windmill-utils-internal/src/deploy.ts";

// A folder's owners, ACL and identity rules name `u/<username>`, which is per-(workspace, email).
// Copying one verbatim hands the folder — or an item's execution identity — to whoever happens to
// hold that username in the target. Alice is the same person under two usernames, Bob exists only
// in the source, and `g/only-src` is a source-only group.
function folderProvider(
  captured: [string, any][],
  folder: Record<string, unknown>,
  alreadyExists = false,
) {
  const users: Record<string, { email: string; username: string }[]> = {
    src: [
      { username: "alice_src", email: "alice@corp" },
      { username: "bob", email: "bob@corp" },
    ],
    dst: [{ username: "alice_dst", email: "alice@corp" }],
  };
  return {
    existsFolder: async () => alreadyExists,
    getFolder: async () => folder,
    listUsers: async (p: { workspace: string }) => users[p.workspace],
    listGroups: async (p: { workspace: string; page?: number }) =>
      p.workspace === "dst" && p.page === 1 ? [{ name: "both" }] : [],
    createFolder: async (p: any) =>
      void captured.push(["createFolder", p.requestBody]),
    updateFolder: async (p: any) =>
      void captured.push(["updateFolder", p.requestBody]),
  } as any;
}

const translatable = {
  name: "x",
  summary: "s",
  owners: ["u/alice_src", "u/bob", "g/both"],
  extra_perms: { "u/alice_src": true, "u/bob": false, "g/only-src": true },
  default_permissioned_as: [{ path_glob: "**", permissioned_as: "u/alice_src" }],
  labels: ["l"],
};

test("deployItem: folder principals are translated into the target's naming", async () => {
  // Create and update build the same body, and only one of them is exercised by any given
  // deploy, so a translation dropped from one alone would go unnoticed.
  for (const [alreadyExists, fn] of [
    [false, "createFolder"],
    [true, "updateFolder"],
  ] as const) {
    const captured: [string, any][] = [];
    const result = await deployItem(
      folderProvider(captured, translatable, alreadyExists),
      "folder" as any,
      "f/x",
      "src",
      "dst",
    );

    expect(result.success).toBe(true);
    expect(captured.map(([name]) => name)).toEqual([fn]);
    const body = captured[0][1];
    // Alice resolves through her email; Bob and the source-only group are dropped, since
    // narrowing a folder is safe where naming a stranger is not.
    expect(body.owners).toEqual(["u/alice_dst", "g/both"]);
    expect(body.extra_perms).toEqual({ "u/alice_dst": true });
    expect(body.default_permissioned_as).toEqual([
      { path_glob: "**", permissioned_as: "u/alice_dst" },
    ]);
    // Both were dropped entirely by the folder branch before this existed.
    expect(body.labels).toEqual(["l"]);
    expect(body.summary).toBe("s");
    // Named once each, though Bob appears in both owners and the ACL.
    expect(result.droppedAccess).toEqual(["u/bob", "g/only-src"]);
  }
});

test("deployItem: an unresolvable identity rule writes nothing", async () => {
  // Dropping the rule would run items landing in the folder as whoever deployed them, and
  // carrying it verbatim would create a folder the server rejects every later item deploy into
  // (`create_folder` checks a rule's shape, `ensure_permissioned_as_exists` its principal's
  // existence). Refusing is the only option that leaves nothing broken behind.
  const captured: [string, any][] = [];
  const result = await deployItem(
    folderProvider(captured, {
      ...translatable,
      default_permissioned_as: [{ path_glob: "**", permissioned_as: "u/bob" }],
    }),
    "folder" as any,
    "f/x",
    "src",
    "dst",
  );

  expect(result.success).toBe(false);
  expect(captured).toEqual([]);
  expect(result.error).toContain("u/bob");
  expect(result.error).toContain("has no account in dst");
});

test("deployItem: a rule whose user left the source says so", async () => {
  // `u/ghost` has no source row to read an email from, so no change to the target can resolve
  // it — telling the user to add them there would send them round a loop.
  const captured: [string, any][] = [];
  const result = await deployItem(
    folderProvider(captured, {
      ...translatable,
      default_permissioned_as: [{ path_glob: "**", permissioned_as: "u/ghost" }],
    }),
    "folder" as any,
    "f/x",
    "src",
    "dst",
  );

  expect(result.success).toBe(false);
  expect(captured).toEqual([]);
  expect(result.error).toContain("no longer has an account in src");
});
