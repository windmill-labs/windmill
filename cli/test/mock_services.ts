import { afterAll, mock } from "bun:test";

// Every in-process suite that stubs the API client goes through here. Bun
// fixes a mocked module's export names at the first `mock.module` call of the
// run, so a stub missing a name leaves it undefined for every later suite, even
// ones that stub it themselves. The module is therefore mocked once, with every
// real export behind a dispatcher: a suite swaps its stubs in and out of
// `active`, and never depends on a later `mock.module` call taking effect.
const real = { ...(await import("../gen/services.gen.ts")) } as Record<
  string,
  unknown
>;
let active: Record<string, unknown> = {};

const dispatched: Record<string, unknown> = {};
for (const [name, value] of Object.entries(real)) {
  dispatched[name] =
    typeof value === "function"
      ? (...args: unknown[]) =>
          ((active[name] ?? value) as (...a: unknown[]) => unknown)(...args)
      : value;
}
mock.module("../gen/services.gen.ts", () => dispatched);

export function mockServices(stubs: Record<string, unknown>): void {
  for (const name of Object.keys(stubs)) {
    if (!(name in real)) {
      throw new Error(`mockServices: services.gen.ts has no export ${name}`);
    }
  }
  active = stubs;
  afterAll(() => {
    active = {};
  });
}
