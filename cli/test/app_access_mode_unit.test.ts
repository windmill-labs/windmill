import { expect, test } from "bun:test";
import {
  executionModeForPush,
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
  await generatingPolicy(
    guest,
    "u/test/app",
    executionModeFromAppFile(guest),
    undefined,
  );
  expect(guest.policy.execution_mode).toBe("guest");

  const anonymous: any = { policy: { execution_mode: "anonymous" }, value: {} };
  markAccessFromPolicy(anonymous);
  anonymous.policy = undefined;
  expect(anonymous.public).toBe(true);
  expect(executionModeFromAppFile(anonymous)).toBe("anonymous");

  expect(executionModeFromAppFile({ policy: { execution_mode: "publisher" } })).toBe("publisher");
  expect(executionModeFromAppFile({})).toBe("publisher");
});

// `viewer` is the narrowest mode — each runnable runs as the viewer, not as the
// app's identity — and the only one with no marker in the file, so both ways it
// can reach a push must survive rather than widen to `publisher`.
test("viewer is never widened to publisher by a push", () => {
  // Carried over from the deployed app: a pull writes no marker for it.
  expect(executionModeForPush({}, { execution_mode: "viewer" })).toBe("viewer");
  // Stated by the file, which is all a first push has to go on.
  expect(executionModeForPush({ policy: { execution_mode: "viewer" } }, undefined)).toBe("viewer");
  // The open-access markers still win, in either direction.
  expect(executionModeForPush({ public: true }, { execution_mode: "viewer" })).toBe("anonymous");
  expect(executionModeForPush({}, { execution_mode: "anonymous" })).toBe("publisher");
});
