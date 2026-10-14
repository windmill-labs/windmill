import { expect, test } from "bun:test";
import { preservedIdentity } from "../src/commands/workspace/merge.ts";

// A merge reads an app's identity out of the source workspace, but `u/` names are per-workspace,
// so what it sends has to be the target's name for the same account. Getting this wrong hands the
// app to whoever holds that username in the target, silently — so pin each branch.
//
// Both caches are pre-seeded, which is also what keeps this off the network: `ensureUserCache`
// short-circuits on a non-empty map.
function seeded(...users: { username: string; email: string }[]) {
  const cache = new Map<string, { username: string; email: string }>();
  for (const u of users) {
    cache.set(u.username, u);
    cache.set(u.email, u);
  }
  return cache;
}

function providerFor(on_behalf_of: string | undefined) {
  return { getAppByPath: async () => ({ policy: { on_behalf_of } }) } as any;
}

const source = () => seeded({ username: "alice", email: "alice@corp" });
const target = () => seeded({ username: "alice_dst", email: "alice@corp" });

test("preservedIdentity: a member's principal is remapped to the target's name for them", async () => {
  expect(
    await preservedIdentity(
      providerFor("u/alice"),
      "app" as any,
      "f/x/a",
      "src",
      "dst",
      source(),
      target(),
    ),
  ).toBe("u/alice_dst");
});

test("preservedIdentity: a group carries over by name", async () => {
  expect(
    await preservedIdentity(
      providerFor("g/ops"),
      "app" as any,
      "f/x/a",
      "src",
      "dst",
      source(),
      target(),
    ),
  ).toBe("g/ops");
});

// An instance superadmin has no `usr` row in either workspace, so there is nothing to map — and
// nothing to map *to*: the backend resolves that principal through the instance-wide `password`
// fallback, which means the same account in every workspace.
test("preservedIdentity: an identity with no member behind it carries over untouched", async () => {
  expect(
    await preservedIdentity(
      providerFor("u/superadmin-external"),
      "app" as any,
      "f/x/a",
      "src",
      "dst",
      source(),
      target(),
    ),
  ).toBe("u/superadmin-external");
});

// ...but only while the target has no member of that name. `resolve_username_to_email` prefers a
// `usr` row over the `password` fallback, so carrying it across would run the app as that member.
test("preservedIdentity: a name the target has a member for is refused, not carried", async () => {
  expect(
    preservedIdentity(
      providerFor("u/saext"),
      "app" as any,
      "f/x/a",
      "src",
      "dst",
      source(),
      seeded({ username: "saext", email: "someone-else@corp" }),
    ),
  ).rejects.toThrow(/cannot be carried across/);
});

test("preservedIdentity: a member absent from the target fails rather than falling back", async () => {
  expect(
    preservedIdentity(
      providerFor("u/alice"),
      "app" as any,
      "f/x/a",
      "src",
      "dst",
      source(),
      seeded({ username: "bob", email: "bob@corp" }),
    ),
  ).rejects.toThrow(/Could not find username for email/);
});
