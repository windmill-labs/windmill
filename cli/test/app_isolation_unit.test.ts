import { expect, test } from "bun:test";
import {
  finalizeDerivedPolicy,
  isolationUpToDate,
  markIsolationFromPolicy,
  statedIsolation,
} from "../src/commands/app/app.ts";

function pushed(file: any, deployed: any, rawApp = true) {
  const policy = { ...deployed };
  finalizeDerivedPolicy(policy, "publisher", statedIsolation(file, rawApp), rawApp);
  return policy;
}

test("a pull writes isolation only when it is on, and a push deploys what was pulled", () => {
  const off: any = { policy: { sandbox: false, frontend_sdk_scopes: ["jobs:run"] } };
  markIsolationFromPolicy(off, true);
  expect("sandbox" in off).toBe(false);
  expect("frontend_api" in off).toBe(false);

  const on: any = { policy: { sandbox: true, frontend_sdk_scopes: ["jobs:run"] } };
  markIsolationFromPolicy(on, true);
  expect(on.sandbox).toBe(true);
  expect(on.frontend_api).toEqual(["jobs:run"]);
  // Into a workspace where the app does not exist yet.
  const fresh = pushed(on, undefined);
  expect(fresh.sandbox).toBe(true);
  expect(fresh.frontend_sdk_scopes).toEqual(["jobs:run"]);
});

test("the file wins when it states sandbox, the deployed policy carries over when it does not", () => {
  const deployed = { sandbox: true, frontend_sdk_scopes: ["jobs:run"] };

  const silent = pushed({}, deployed);
  expect(silent.sandbox).toBe(true);
  expect(silent.frontend_sdk_scopes).toEqual(["jobs:run"]);
  expect(isolationUpToDate(statedIsolation({}, true), deployed, true)).toBe(true);

  // A file with the key states the scopes too: none here.
  const narrowed = pushed({ sandbox: true }, deployed);
  expect(narrowed.sandbox).toBe(true);
  expect(narrowed.frontend_sdk_scopes).toBeUndefined();
  expect(isolationUpToDate(statedIsolation({ sandbox: true }, true), deployed, true)).toBe(false);

  const opened = pushed({ sandbox: false }, deployed);
  expect(opened.sandbox).toBe(false);
  expect(opened.frontend_sdk_scopes).toBeUndefined();
});

test("scopes are refused without sandbox, outside the allowlist, or repeated", () => {
  expect(() => statedIsolation({ frontend_api: ["jobs:run"] }, true)).toThrow("requires 'sandbox: true'");
  expect(() => statedIsolation({ sandbox: false, frontend_api: ["jobs:run"] }, true)).toThrow(
    "requires 'sandbox: true'",
  );
  expect(() => statedIsolation({ sandbox: true, frontend_api: ["apps:write"] }, true)).toThrow("Invalid");
  expect(() => statedIsolation({ sandbox: true, frontend_api: ["jobs:run", "jobs:run"] }, true)).toThrow(
    "Duplicate",
  );
  expect(() => statedIsolation({ sandbox: true, frontend_api: ["jobs:run"] }, false)).toThrow("full-code");
});
