import { afterAll, mock } from "bun:test";

// Every in-process suite that stubs the API client goes through here. Bun
// fixes a mocked module's export names at the first `mock.module` call of the
// run, so a stub missing a name leaves it undefined for every later suite, even
// ones that stub it themselves. Stubs therefore always sit on top of the real
// exports, captured once, before any suite has replaced them.
const real = { ...(await import("../gen/services.gen.ts")) };

export function mockServices(stubs: Record<string, unknown>): void {
  mock.module("../gen/services.gen.ts", () => ({ ...real, ...stubs }));
  afterAll(() => {
    mock.module("../gen/services.gen.ts", () => real);
  });
}
