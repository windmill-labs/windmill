/**
 * An agent's run-as identity lives in its value, which the pull leaves out. A
 * push must neither see that gap as a change nor let the file pick who the
 * agent runs as: only the deployed identity is claimed back, and only by a
 * pusher the backend lets keep it.
 */

import { expect, test } from "bun:test";
import { isSuperset } from "../src/types.ts";
import { agentOnBehalfOf } from "../src/commands/resource/resource.ts";

const ctx = (userIsAdminOrDeployer: boolean) => ({
  userCache: new Map(),
  userIsAdminOrDeployer,
  userEmail: "me@x.dev",
});

function agents(localObo?: string) {
  const deployed = {
    resource_type: "ai_agent",
    value: { system_prompt: "hi", on_behalf_of: "u/bob" },
  } as any;
  const local = {
    resource_type: "ai_agent",
    value: { system_prompt: "hi", ...(localObo ? { on_behalf_of: localObo } : {}) },
  } as any;
  return { deployed, local };
}

test("an unchanged agent still compares equal to the deployed one", () => {
  const { deployed, local } = agents();
  agentOnBehalfOf(deployed, local, ctx(true));
  expect(isSuperset(local, deployed)).toBe(true);
});

test("an admin or deployer claims the deployed identity, never the file's", () => {
  const { deployed, local } = agents("u/admin");
  expect(agentOnBehalfOf(deployed, local, ctx(true))).toBe("u/bob");
  expect(local.value.on_behalf_of).toBeUndefined();
});

test("anyone else claims nothing", () => {
  const { deployed, local } = agents("u/bob");
  expect(agentOnBehalfOf(deployed, local, ctx(false))).toBeUndefined();
  expect(agentOnBehalfOf(agents().deployed, agents().local, undefined)).toBeUndefined();
  expect(local.value.on_behalf_of).toBeUndefined();
});

test("other resources are left alone, an agent retyped into one included", () => {
  for (const deployedType of ["json", "ai_agent"]) {
    const deployed = { resource_type: deployedType, value: { on_behalf_of: "u/bob" } } as any;
    const local = { resource_type: "json", value: { on_behalf_of: "u/x" } } as any;
    expect(agentOnBehalfOf(deployed, local, ctx(true))).toBeUndefined();
    expect(local.value.on_behalf_of).toBe("u/x");
    expect(deployed.value.on_behalf_of).toBe("u/bob");
  }
});
