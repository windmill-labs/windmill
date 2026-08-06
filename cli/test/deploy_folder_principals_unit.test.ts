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
    // `group_` rows only — what an identity rule resolves against.
    listGroups: async (p: { workspace: string; page?: number }) =>
      p.workspace === "dst" && p.page === 1 ? [{ name: "both" }] : [],
    // `group_` unioned with instance groups — what an owner or ACL entry is matched against.
    listGroupNames: async (p: { workspace: string }) =>
      p.workspace === "dst" ? ["both", "igroup"] : [],
    createFolder: async (p: any) =>
      void captured.push(["createFolder", p.requestBody]),
    updateFolder: async (p: any) =>
      void captured.push(["updateFolder", p.requestBody]),
  } as any;
}

const translatable = {
  name: "x",
  summary: "s",
  owners: ["u/alice_src", "u/bob", "g/both", "g/igroup"],
  extra_perms: {
    "u/alice_src": true,
    "u/bob": false,
    "g/only-src": true,
    "g/igroup": true,
  },
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
    // narrowing a folder is safe where naming a stranger is not. `g/igroup` is an instance
    // group — instance-wide, so it grants access in the target and must survive.
    expect(body.owners).toEqual(["u/alice_dst", "g/both", "g/igroup"]);
    expect(body.extra_perms).toEqual({ "u/alice_dst": true, "g/igroup": true });
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

test("deployItem: an instance group is not accepted as an identity rule", async () => {
  // `ensure_permissioned_as_exists` resolves a rule against `group_` alone, so carrying an
  // instance group here would write a folder the server then rejects every item deploy into —
  // the opposite of the owners/ACL case in the test above, which is why the two use different
  // group sets rather than one.
  const captured: [string, any][] = [];
  const result = await deployItem(
    folderProvider(captured, {
      ...translatable,
      default_permissioned_as: [
        { path_glob: "**", permissioned_as: "g/igroup" },
      ],
    }),
    "folder" as any,
    "f/x",
    "src",
    "dst",
  );

  expect(result.success).toBe(false);
  expect(captured).toEqual([]);
  expect(result.error).toContain("g/igroup");
});

test("deployItem: an all-untranslatable owners list never blanks the target's", async () => {
  // `update_folder` force-appends the caller only when they are not an admin, so writing `[]`
  // over an existing list would leave an admin-deployed folder with no owners at all.
  const captured: [string, any][] = [];
  const result = await deployItem(
    folderProvider(
      captured,
      { ...translatable, owners: ["u/bob"], extra_perms: {} },
      true,
    ),
    "folder" as any,
    "f/x",
    "src",
    "dst",
  );

  expect(result.success).toBe(true);
  expect(captured[0][0]).toBe("updateFolder");
  expect("owners" in JSON.parse(JSON.stringify(captured[0][1]))).toBe(false);
  expect(result.droppedAccess).toEqual(["u/bob"]);
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
