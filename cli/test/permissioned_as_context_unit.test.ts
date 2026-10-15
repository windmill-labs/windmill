/**
 * Regression guard: a superadmin whose `usr` row in the workspace is not an
 * admin still preserves ownership on the backend, so the CLI must not refuse
 * their push as if it would reassign every item to them.
 */

import { expect, test } from "bun:test";
import { mockServices } from "./mock_services.ts";

mockServices({
  whoami: async () => ({
    email: "super@windmill.dev",
    username: "super",
    is_admin: false,
    is_super_admin: true,
    groups: [],
  }),
});

const { buildPermissionedAsContext } = await import(
  "../src/core/permissioned_as.ts"
);

test("a superadmin with a non-admin membership counts as able to preserve", async () => {
  const ctx = await buildPermissionedAsContext("w", "v1");
  expect(ctx?.userIsAdminOrDeployer).toBe(true);
});
