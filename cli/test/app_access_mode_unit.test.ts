import { expect, test } from "bun:test";
import {
  deployedPolicyBase,
  executionModeFromAppFile,
  generatingPolicy,
  markAccessFromPolicy,
} from "../src/commands/app/app.ts";

// The access mode is the one policy field a tracked app keeps; a pull then a push must
// deploy the mode that was pulled, guest included, not a default.
test("the access mode survives the app.yaml round trip", async () => {
  const guest: any = { policy: { execution_mode: "guest" }, value: {} };
  markAccessFromPolicy(guest);
  guest.policy = undefined;
  expect(guest.guests).toBe(true);
  expect(guest.public).toBeUndefined();
  expect(executionModeFromAppFile(guest)).toBe("guest");
  await generatingPolicy(guest, "u/test/app", executionModeFromAppFile(guest));
  expect(guest.policy.execution_mode).toBe("guest");

  const anonymous: any = { policy: { execution_mode: "anonymous" }, value: {} };
  markAccessFromPolicy(anonymous);
  anonymous.policy = undefined;
  expect(anonymous.public).toBe(true);
  expect(executionModeFromAppFile(anonymous)).toBe("anonymous");

  expect(executionModeFromAppFile({ policy: { execution_mode: "publisher" } })).toBe("publisher");
  expect(executionModeFromAppFile({})).toBe("publisher");
});

// `viewer` has no representation in the app file, so a push that read the mode
// off the file alone demoted every viewer app to publisher — running the
// publisher's identity for viewers instead of each viewer's own.
test("a deployed viewer mode is not demoted by a file that cannot express it", () => {
  expect(executionModeFromAppFile({}, { execution_mode: "viewer" } as any)).toBe("viewer");
  expect(executionModeFromAppFile({ policy: { execution_mode: "viewer" } })).toBe("viewer");
  // Widening still has to come from the file, and dropping `public` still demotes.
  expect(executionModeFromAppFile({}, { execution_mode: "anonymous" } as any)).toBe("publisher");
  expect(
    executionModeFromAppFile({ public: true }, { execution_mode: "viewer" } as any),
  ).toBe("anonymous");
});

// A push regenerates the policy, so whatever `deployedPolicyBase` does not carry
// over is dropped from the deployed app. Older pulls wrote the whole policy into
// the app file, so repos still ship `on_behalf_of` keys — letting one win would
// hand the app's execution identity to checked-in content, which is what
// `preserve_on_behalf_of` exists to prevent.
test("deployedPolicyBase: the file never sets the run-as identity", () => {
  const deployed: any = {
    on_behalf_of: "u/svc",
    on_behalf_of_email: "svc@corp",
    sandbox: true,
  };
  for (
    const local of [
      undefined,
      { on_behalf_of: null, on_behalf_of_email: null },
      { on_behalf_of: "u/stale", on_behalf_of_email: "stale@corp" },
    ] as any[]
  ) {
    expect(deployedPolicyBase(deployed, local)).toMatchObject({
      on_behalf_of: "u/svc",
      on_behalf_of_email: "svc@corp",
      sandbox: true,
    });
  }
});

// Grants keyed to the sources a deploy replaces must not survive it, from either
// side of the merge: the backend folds legacy `triggerables` into
// `triggerables_v2` on read and enforces S3 access against `s3_inputs`.
test("deployedPolicyBase: derived grants are never carried over", () => {
  const stale: any = {
    triggerables: { "gone:script/f/gone": {} },
    s3_inputs: [{ allowed_resources: ["u/gone"] }],
    allowed_s3_keys: [{ s3_path: "gone" }],
  };
  for (const [deployed, local] of [[stale, undefined], [{}, stale]] as any[]) {
    const base = deployedPolicyBase(deployed, local) as any;
    expect(base.triggerables).toBeUndefined();
    expect(base.s3_inputs).toBeUndefined();
    expect(base.allowed_s3_keys).toBeUndefined();
  }
});
