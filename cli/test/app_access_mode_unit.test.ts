import { expect, test } from "bun:test";
import {
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
